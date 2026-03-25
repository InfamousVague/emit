import { useState, useEffect, useMemo, useRef } from "react";
import type { CommandEntry, Settings } from "../lib/types";
import { getSettings, search, searchStatic } from "../lib/tauri";
import { useCommandParser } from "./useCommandParser";
import { SETTING_DEFS } from "../components/Settings/Settings";

export type SearchMode = "search" | "command";

/** Lower number = higher priority in results list. */
const CATEGORY_PRIORITY: Record<string, number> = {
  Calculator: 0,
  Bitwarden: 1,
  "Developer Tools": 2,
  Productivity: 3,
  Design: 4,
  Security: 5,
  System: 6,
  Settings: 6,
  Applications: 7,
  "Window Management": 8,
  Notion: 9,
  Files: 10,
  "Web Search": 11,
};

/** Score threshold above which a result's score takes precedence over category ordering. */
const HIGH_SCORE_THRESHOLD = 70;

const BOOLEAN_SETTING_KEYS = new Set([
  "replace_spotlight",
  "launch_at_login",
  "show_in_dock",
  "check_for_updates",
]);

function matchSettings(
  query: string,
  settings: Settings | null,
): CommandEntry[] {
  if (!query || !settings) return [];
  const q = query.toLowerCase();
  return SETTING_DEFS
    .filter(
      (s) =>
        s.label.toLowerCase().includes(q) ||
        s.description.toLowerCase().includes(q),
    )
    .map((s) => {
      const idx = s.label.toLowerCase().indexOf(q);
      const descIdx = s.description.toLowerCase().indexOf(q);
      const matchStart = idx !== -1 ? idx : descIdx;
      const isBool = BOOLEAN_SETTING_KEYS.has(s.key);
      const currentValue = isBool
        ? (settings[s.key as keyof Settings] ? "On" : "Off")
        : null;
      return {
        id: `setting:${s.key}`,
        name: s.label,
        description: currentValue
          ? `${s.description} · Currently: ${currentValue}`
          : s.description,
        category: "Settings",
        icon: null,
        match_indices: idx !== -1
          ? Array.from({ length: q.length }, (_, i) => idx + i)
          : [],
        score: matchStart === 0 ? 100 : 50,
      };
    });
}

function categoryRank(category: string): number {
  return CATEGORY_PRIORITY[category] ?? 3;
}

/** Maps user-facing prefix to internal category name. */
const FILTER_PREFIXES: Record<string, string> = {
  "notion:": "Notion",
  "file:": "Files",
  "app:": "Applications",
  "system:": "System",
  "dev:": "Developer Tools",
  "design:": "Design",
  "prod:": "Productivity",
  "security:": "Security",
  "pw:": "PasswordGenerator",
  "settings:": "Settings",
};

export function useSearch() {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<CommandEntry[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [isSearching, setIsSearching] = useState(false);
  const dynamicDebounceRef = useRef<ReturnType<typeof setTimeout> | undefined>(
    undefined,
  );
  const dynamicRequestRef = useRef(0);
  const [cachedSettings, setCachedSettings] = useState<Settings | null>(null);

  // Load settings for injecting into search results
  useEffect(() => {
    getSettings().then(setCachedSettings);
  }, []);

  const mode: SearchMode = query.startsWith("/") ? "command" : "search";

  const commandParser = useCommandParser(query, setQuery);

  // ── Filter prefix detection ───────────────────────────────────────────
  const { filterCategory, filterQuery, filterPrefixLength } = useMemo(() => {
    if (query.startsWith("/")) {
      return { filterCategory: null, filterQuery: query, filterPrefixLength: 0 };
    }
    const lower = query.toLowerCase();
    for (const [prefix, category] of Object.entries(FILTER_PREFIXES)) {
      if (lower.startsWith(prefix)) {
        return {
          filterCategory: category,
          filterQuery: query.slice(prefix.length).trimStart(),
          filterPrefixLength: prefix.length,
        };
      }
    }
    return { filterCategory: null, filterQuery: query, filterPrefixLength: 0 };
  }, [query]);

  // Ghost text for filter prefix suggestions (e.g. "no" → "tion:")
  const filterGhostText = useMemo(() => {
    if (filterCategory || query.startsWith("/") || query.length === 0) return "";
    const lower = query.toLowerCase();
    for (const prefix of Object.keys(FILTER_PREFIXES)) {
      if (prefix.startsWith(lower) && lower.length < prefix.length) {
        return prefix.slice(lower.length);
      }
    }
    return "";
  }, [query, filterCategory]);

  // The actual query to send to backends (stripped of filter prefix)
  const searchQuery = filterCategory ? filterQuery : query;

  // ── Immediate static search ───────────────────────────────────────────
  useEffect(() => {
    if (query.startsWith("/")) {
      setResults([]);
      setIsSearching(false);
      return;
    }
    let cancelled = false;
    searchStatic(searchQuery).then((items) => {
      if (!cancelled) {
        const filtered = filterCategory
          ? items.filter((r) => r.category === filterCategory)
          : items;
        // Inject matching settings into results
        const settingsItems =
          !filterCategory || filterCategory === "Settings"
            ? matchSettings(searchQuery, cachedSettings)
            : [];
        const merged = [...filtered, ...settingsItems];
        merged.sort((a, b) => {
          const aScore = a.score ?? 0;
          const bScore = b.score ?? 0;
          const aHigh = aScore >= HIGH_SCORE_THRESHOLD;
          const bHigh = bScore >= HIGH_SCORE_THRESHOLD;
          // When either result has a strong match score, sort by score first
          if (aHigh || bHigh) return bScore - aScore;
          const catDiff = categoryRank(a.category) - categoryRank(b.category);
          if (catDiff !== 0) return catDiff;
          return bScore - aScore;
        });
        setResults(merged);
        setSelectedIndex(0);
      }
    });
    return () => {
      cancelled = true;
    };
  }, [query, searchQuery, filterCategory, cachedSettings]);

  // ── Debounced dynamic search ──────────────────────────────────────────
  useEffect(() => {
    if (query.startsWith("/") || searchQuery.trim() === "") {
      setIsSearching(false);
      return;
    }
    if (dynamicDebounceRef.current) clearTimeout(dynamicDebounceRef.current);
    setIsSearching(true);
    const requestId = ++dynamicRequestRef.current;
    // Shorter debounce when a filter is active — user explicitly wants that category
    const debounceMs = filterCategory ? 50 : 200;
    dynamicDebounceRef.current = setTimeout(() => {
      search(searchQuery).then((dynamicItems) => {
        if (requestId === dynamicRequestRef.current) {
          setIsSearching(false);
        }
        // Apply category filter to dynamic results too
        const items = filterCategory
          ? dynamicItems.filter((d) => d.category === filterCategory)
          : dynamicItems;
        if (items.length > 0) {
          setResults((prev) => {
            const existingIds = new Set(prev.map((r) => r.id));
            const newDynamic = items.filter(
              (d) => !existingIds.has(d.id),
            );
            if (newDynamic.length === 0) return prev;
            const merged = [...prev, ...newDynamic];
            merged.sort((a, b) => {
              const catDiff = categoryRank(a.category) - categoryRank(b.category);
              if (catDiff !== 0) return catDiff;
              return (b.score ?? 0) - (a.score ?? 0);
            });
            return merged;
          });
        }
      });
    }, debounceMs);
    return () => {
      if (dynamicDebounceRef.current) clearTimeout(dynamicDebounceRef.current);
    };
  }, [query, searchQuery, filterCategory]);

  const refreshSettings = () => {
    getSettings().then(setCachedSettings);
  };

  return {
    query,
    setQuery,
    results,
    selectedIndex,
    setSelectedIndex,
    isSearching,
    mode,
    commandParser,
    filterCategory,
    filterPrefixLength,
    filterGhostText,
    refreshSettings,
  };
}

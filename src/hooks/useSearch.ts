import { useState, useEffect, useMemo, useRef } from "react";
import type { CommandEntry } from "../lib/types";
import { search, searchStatic } from "../lib/tauri";
import { useCommandParser } from "./useCommandParser";

export type SearchMode = "search" | "command";

/** Lower number = higher priority in results list. */
const CATEGORY_PRIORITY: Record<string, number> = {
  Extensions: 0,
  System: 1,
  Applications: 2,
  Notion: 3,
  Files: 4,
};

function categoryRank(category: string): number {
  return CATEGORY_PRIORITY[category] ?? 3;
}

/** Maps user-facing prefix to internal category name. */
const FILTER_PREFIXES: Record<string, string> = {
  "notion:": "Notion",
  "file:": "Files",
  "app:": "Applications",
  "system:": "System",
  "ext:": "Extensions",
  "pw:": "PasswordGenerator",
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
        setResults(filtered);
        setSelectedIndex(0);
      }
    });
    return () => {
      cancelled = true;
    };
  }, [query, searchQuery, filterCategory]);

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
  };
}

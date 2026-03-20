import { useCallback, useEffect, useMemo, useState } from "react";
import { SearchInput } from "./components/SearchInput/SearchInput";
import { Marketplace } from "./components/Marketplace/Marketplace";
import { ExtensionDetail } from "./components/Marketplace/ExtensionDetail";
import { UpdateBanner } from "./components/UpdateBanner/UpdateBanner";
import { AppViewRouter } from "./components/AppViewRouter";
import { usePerfMonitor } from "./hooks/usePerfMonitor";
import { useSearch } from "./hooks/useSearch";
import { useKeyboardNav } from "./hooks/useKeyboardNav";
import { useAutoUpdate } from "./hooks/useAutoUpdate";
import { useCommandMode } from "./hooks/useCommandMode";
import {
  executeAction,
  executeCommand,
  getSettings,
  hideWindow,
  rulerOpen,
  saveSettings,
  wmSnapFocused,
} from "./lib/tauri";
import { exit } from "@tauri-apps/plugin-process";
import { listen } from "@tauri-apps/api/event";
import { groupByCategory } from "./lib/groupByCategory";
import type {
  CommandDefinition,
  CommandResult,
  SnapPosition,
} from "./lib/types";
import "./styles/app.css";

type View =
  | "search"
  | "settings"
  | "clipboard"
  | "marketplace"
  | "extension-detail"
  | "notion"
  | "command"
  | "param-wizard"
  | "color-picker"
  | "password-generator"
  | "window-manager"
  | "screenshot"
  | "perf";

const BOOLEAN_SETTINGS = new Set([
  "replace_spotlight",
  "launch_at_login",
  "show_in_dock",
  "check_for_updates",
]);

const SNAP_POSITIONS: Record<string, SnapPosition> = {
  left_half: "LeftHalf", right_half: "RightHalf",
  top_half: "TopHalf", bottom_half: "BottomHalf",
  top_left: "TopLeftQuarter", top_right: "TopRightQuarter",
  bottom_left: "BottomLeftQuarter", bottom_right: "BottomRightQuarter",
  left_third: "LeftThird", center_third: "CenterThird", right_third: "RightThird",
  left_two_thirds: "LeftTwoThirds", right_two_thirds: "RightTwoThirds",
  maximize: "Maximize", center: "Center",
};

export function App() {
  const [view, setView] = useState<View>("search");
  const [searchTrailing, setSearchTrailing] = useState<React.ReactNode>(null);
  const [selectedExtensionId, setSelectedExtensionId] = useState("");
  const {
    query,
    setQuery,
    results,
    selectedIndex,
    setSelectedIndex,
    isSearching,
    mode,
    commandParser,
    filterPrefixLength,
    filterGhostText,
    refreshSettings,
  } = useSearch();

  // Command system state
  const [activeCommand, setActiveCommand] = useState<CommandDefinition | null>(null);
  const [lastResult, setLastResult] = useState<CommandResult | null>(null);
  const [allCommands, setAllCommands] = useState<CommandDefinition[]>([]);
  const [showUndo, setShowUndo] = useState(false);
  const [isCommandExecuting, setIsCommandExecuting] = useState(false);
  const [perfScrollTo, setPerfScrollTo] = useState<string | undefined>();
  const [wizardInitialValues, setWizardInitialValues] = useState<Record<string, unknown>>({});

  // Global Cmd+Q to quit
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "q" && e.metaKey) {
        e.preventDefault();
        exit(0);
      }
    };
    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, []);

  // Listen for navigate-view events from global shortcuts
  useEffect(() => {
    const unlisten = listen<string>("navigate-view", (event) => {
      if (event.payload === "perf") {
        setQuery("");
        setPerfScrollTo(undefined);
        setView((prev) => (prev === "perf" ? "search" : "perf"));
      }
    });
    return () => { unlisten.then((fn) => fn()); };
  }, [setQuery]);

  // ── View navigation ──────────────────────────────────────────────────────

  const handleExecute = useCallback(
    async (id: string) => {
      if (id.startsWith("setting:")) {
        const key = id.replace("setting:", "");
        if (BOOLEAN_SETTINGS.has(key)) {
          const current = await getSettings();
          const updated = { ...current, [key]: !current[key as keyof typeof current] };
          await saveSettings(updated);
          refreshSettings();
          return;
        }
        setQuery("");
        setView("settings");
        return;
      }

      const result = await executeCommand(id);
      if (result.startsWith("view:")) {
        setQuery("");
        if (result === "view:perf" || result.startsWith("view:perf:")) {
          setPerfScrollTo(result.includes(":") ? result.split(":").pop() : undefined);
          setView("perf");
        } else {
          const viewMap: Record<string, View> = {
            "view:clipboard": "clipboard",
            "view:marketplace": "marketplace",
            "view:notion": "notion",
            "view:color-picker": "color-picker",
            "view:password-generator": "password-generator",
            "view:settings": "settings",
            "view:window-manager": "window-manager",
            "view:screenshot": "screenshot",
          };
          const target = viewMap[result];
          if (target) setView(target);
        }
      } else if (result === "action:ruler") {
        await rulerOpen();
      } else if (result.startsWith("action:wm.snap.")) {
        const snapPos = SNAP_POSITIONS[result.replace("action:wm.snap.", "")];
        if (snapPos) await wmSnapFocused(snapPos);
        await hideWindow();
      } else {
        await hideWindow();
      }
    },
    [setView, setQuery, refreshSettings],
  );

  const handleCommandComplete = useCallback(
    async (_result: CommandResult) => {
      setActiveCommand(null);
      setLastResult(null);
      setQuery("");
      setView("search");
      await hideWindow();
    },
    [setQuery],
  );

  const handleFollowUpSelect = useCallback(
    (command: CommandDefinition) => {
      setLastResult(null);
      setActiveCommand(command);
      setView("param-wizard");
    },
    [],
  );

  const handleCommandSelect = useCallback(
    async (cmd: CommandDefinition, initVals?: Record<string, unknown>) => {
      const vals = initVals ?? {};
      const requiredParams = cmd.params.filter((p) => p.group === "Required");
      const allFilled = requiredParams.every((p) => vals[p.id] !== undefined);

      if (allFilled) {
        setIsCommandExecuting(true);
        try {
          const result = await executeAction(cmd.id, vals);
          handleCommandComplete(result);
        } catch {
          setActiveCommand(cmd);
          setWizardInitialValues(vals);
          setView("param-wizard");
        } finally {
          setIsCommandExecuting(false);
        }
      } else {
        setActiveCommand(cmd);
        setWizardInitialValues(vals);
        setView("param-wizard");
      }
    },
    [handleCommandComplete],
  );

  // ── Command mode hook ────────────────────────────────────────────────────

  const {
    autocompleteOptions,
    autocompleteIndex,
    showAutocomplete,
    handleCommandKeyDown,
    setAutocompleteOptions,
  } = useCommandMode({
    commandParser,
    query,
    setQuery,
    onCommandSelect: handleCommandSelect,
  });

  // ── Keyboard navigation ──────────────────────────────────────────────────

  useKeyboardNav({
    results: mode === "command" ? [] : results,
    selectedIndex,
    setSelectedIndex,
    onExecute: handleExecute,
    disabled:
      view === "clipboard" ||
      view === "notion" ||
      view === "color-picker" ||
      view === "password-generator" ||
      view === "window-manager" ||
      view === "screenshot" ||
      view === "settings" ||
      view === "param-wizard" ||
      view === "perf" ||
      mode === "command",
  });

  const { update, installUpdate, dismissUpdate, checkNow } = useAutoUpdate();
  const { history: perfHistory } = usePerfMonitor();

  const groups = useMemo(() => groupByCategory(results), [results]);

  const handleItemClick = async (globalIndex: number) => {
    if (results[globalIndex]) {
      await handleExecute(results[globalIndex].id);
    }
  };

  const handleBack = () => {
    setQuery("");
    setSearchTrailing(null);
    setActiveCommand(null);
    setLastResult(null);
    setView("search");
  };

  const handleWizardBack = () => {
    setActiveCommand(null);
    setView("search");
    setQuery("/");
  };

  const handleSettingsClick = useCallback(() => {
    setQuery("");
    setView("settings");
  }, [setQuery]);

  const handleMarketplaceClick = useCallback(() => {
    setQuery("");
    setView("marketplace");
  }, [setQuery]);

  // Keyboard handler for search mode: Tab-to-accept ghost text, Backspace to delete filter prefix
  const handleSearchKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLInputElement>) => {
      if (e.key === "Tab" && filterGhostText) {
        e.preventDefault();
        setQuery(query + filterGhostText + " ");
      } else if (e.key === "Backspace" && filterPrefixLength > 0) {
        const input = e.currentTarget;
        const cursorPos = input.selectionStart ?? query.length;
        if (cursorPos <= filterPrefixLength) {
          e.preventDefault();
          setQuery(query.slice(filterPrefixLength));
        }
      }
    },
    [query, filterGhostText, filterPrefixLength, setQuery],
  );

  // ── Render ───────────────────────────────────────────────────────────────

  const isCommandMode = mode === "command" && view !== "param-wizard";
  const placeholder =
    view === "clipboard" ? "Filter clipboard history\u2026"
    : view === "notion" ? "Filter Notion pages\u2026"
    : view === "color-picker" ? "Palette name\u2026"
    : view === "password-generator" ? "Filter history\u2026"
    : view === "window-manager" ? "Search windows\u2026"
    : view === "screenshot" ? "Filter screenshots\u2026"
    : view === "settings" ? "Search settings\u2026"
    : view === "perf" ? "Search metrics\u2026"
    : mode === "command" ? "Type a command..."
    : "Search for apps and commands...";

  return (
    <div className="app-wrapper">
      <div className={`app-shell${view === "perf" ? " app-shell--expanded" : ""}`}>
        {view === "marketplace" ? (
          <Marketplace
            onBack={handleBack}
            onExtensionClick={(id) => {
              setSelectedExtensionId(id);
              setView("extension-detail");
            }}
            filter={query}
          />
        ) : view === "extension-detail" ? (
          <ExtensionDetail
            extensionId={selectedExtensionId}
            onBack={() => setView("marketplace")}
          />
        ) : (
          <>
            {update.available && !update.dismissed && (
              <UpdateBanner
                version={update.version!}
                downloading={update.downloading}
                onUpdate={installUpdate}
                onDismiss={dismissUpdate}
              />
            )}
            <SearchInput
              value={
                view === "param-wizard" && activeCommand
                  ? `${activeCommand.extension_id}: ${activeCommand.name}`
                  : query
              }
              onChange={view === "param-wizard" ? () => {} : setQuery}
              placeholder={placeholder}
              readOnly={view === "param-wizard"}
              onBack={
                view === "clipboard" || view === "notion" || view === "color-picker" || view === "password-generator" || view === "window-manager" || view === "screenshot" || view === "settings" || view === "perf"
                  ? handleBack
                  : view === "param-wizard"
                    ? handleWizardBack
                    : undefined
              }
              trailing={searchTrailing ?? (isSearching || isCommandExecuting ? <span className="search-spinner" /> : null)}
              onKeyDown={isCommandMode ? handleCommandKeyDown : (filterGhostText || filterPrefixLength > 0) ? handleSearchKeyDown : undefined}
              ghostText={isCommandMode ? commandParser.ghostText : filterGhostText || undefined}
              labelRanges={
                isCommandMode ? commandParser.labelRanges
                : filterPrefixLength > 0 ? [{ start: 0, end: filterPrefixLength }]
                : undefined
              }
              cursorPos={isCommandMode ? commandParser.pendingCursorPos : undefined}
              onCursorApplied={isCommandMode ? commandParser.clearPendingCursor : undefined}
            />
            <AppViewRouter
              view={view}
              query={query}
              setQuery={setQuery}
              mode={mode}
              onBack={handleBack}
              onWizardBack={handleWizardBack}
              onSettingsClick={handleSettingsClick}
              onMarketplaceClick={handleMarketplaceClick}
              onCheckForUpdates={checkNow}
              onTrailingChange={setSearchTrailing}
              commandParser={commandParser}
              onCommandSelect={handleCommandSelect}
              autocompleteOptions={autocompleteOptions}
              autocompleteIndex={autocompleteIndex}
              showAutocomplete={showAutocomplete}
              setAutocompleteOptions={setAutocompleteOptions}
              activeCommand={activeCommand}
              lastResult={lastResult}
              allCommands={allCommands}
              showUndo={showUndo}
              onCommandComplete={handleCommandComplete}
              onFollowUpSelect={handleFollowUpSelect}
              onUndoDismiss={() => setShowUndo(false)}
              wizardInitialValues={wizardInitialValues}
              results={results}
              selectedIndex={selectedIndex}
              groups={groups}
              onItemClick={handleItemClick}
              perfHistory={perfHistory}
              perfScrollTo={perfScrollTo}
            />
          </>
        )}
      </div>
    </div>
  );
}

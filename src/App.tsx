import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { SearchInput } from "./components/SearchInput/SearchInput";
import { ResultGroup } from "./components/ResultGroup/ResultGroup";
import { Footer } from "./components/Footer/Footer";
import { EmptyState } from "./components/EmptyState/EmptyState";
import { Settings } from "./components/Settings/Settings";
import { ClipboardManager } from "./components/ClipboardManager/ClipboardManager";
import { ColorPicker } from "./components/ColorPicker/ColorPicker";
import { PasswordGenerator } from "./components/PasswordGenerator/PasswordGeneratorView";
import { Marketplace } from "./components/Marketplace/Marketplace";
import { ExtensionDetail } from "./components/Marketplace/ExtensionDetail";
import { NotionView } from "./components/NotionView/NotionView";
import { CommandMode } from "./components/CommandMode/CommandMode";
import { FollowUpBar } from "./components/CommandMode/FollowUpBar";
import { UndoToast } from "./components/CommandMode/UndoToast";
import { ParamWizard } from "./components/ParamWizard/ParamWizard";
import { UpdateBanner } from "./components/UpdateBanner/UpdateBanner";
import { useSearch } from "./hooks/useSearch";
import { useKeyboardNav } from "./hooks/useKeyboardNav";
import { useAutoUpdate } from "./hooks/useAutoUpdate";
import {
  executeAction,
  executeCommand,
  hideWindow,
  resolveParamOptions,
  searchCommands,
} from "./lib/tauri";
import { groupByCategory } from "./lib/groupByCategory";
import type {
  CommandDefinition,
  CommandResult,
  SelectOption,
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
  | "password-generator";

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
    filterCategory,
    filterPrefixLength,
    filterGhostText,
  } = useSearch();

  // Command system state
  const [activeCommand, setActiveCommand] = useState<CommandDefinition | null>(
    null,
  );
  const [lastResult, setLastResult] = useState<CommandResult | null>(null);
  const [allCommands, setAllCommands] = useState<CommandDefinition[]>([]);
  const [showUndo, setShowUndo] = useState(false);
  const [isCommandExecuting, setIsCommandExecuting] = useState(false);


  // ── Autocomplete for picker params in args phase ──────────────────────
  const [autocompleteOptions, setAutocompleteOptions] = useState<
    SelectOption[]
  >([]);
  const [autocompleteIndex, setAutocompleteIndex] = useState(0);
  const autocompleteDebounceRef = useRef<ReturnType<typeof setTimeout>>(
    undefined,
  );

  const isPickerParam =
    commandParser.currentParam &&
    ["DatabasePicker", "PagePicker", "People", "DynamicSelect"].includes(
      commandParser.currentParam.param_type.type,
    );

  const showAutocomplete =
    commandParser.phase === "args" &&
    isPickerParam &&
    autocompleteOptions.length > 0;

  // Fetch autocomplete options when currentParam or query changes
  useEffect(() => {
    if (!commandParser.lockedCommand || !commandParser.currentParam) {
      setAutocompleteOptions([]);
      return;
    }

    const paramType = commandParser.currentParam.param_type.type;
    if (
      !["DatabasePicker", "PagePicker", "People", "DynamicSelect"].includes(
        paramType,
      )
    ) {
      setAutocompleteOptions([]);
      return;
    }

    if (autocompleteDebounceRef.current)
      clearTimeout(autocompleteDebounceRef.current);

    autocompleteDebounceRef.current = setTimeout(async () => {
      try {
        const options = await resolveParamOptions(
          commandParser.lockedCommand!.id,
          commandParser.currentParam!.id,
          commandParser.currentParamQuery,
        );
        setAutocompleteOptions(options);
        setAutocompleteIndex(0);
      } catch {
        setAutocompleteOptions([]);
      }
    }, 150);

    return () => {
      if (autocompleteDebounceRef.current)
        clearTimeout(autocompleteDebounceRef.current);
    };
  }, [
    commandParser.lockedCommand,
    commandParser.currentParam,
    commandParser.currentParamQuery,
  ]);

  const handleExecute = useCallback(
    async (id: string) => {
      const result = await executeCommand(id);
      if (result === "view:clipboard") {
        setQuery("");
        setView("clipboard");
      } else if (result === "view:marketplace") {
        setQuery("");
        setView("marketplace");
      } else if (result === "view:notion") {
        setQuery("");
        setView("notion");
      } else if (result === "view:color-picker") {
        setQuery("");
        setView("color-picker");
      } else if (result === "view:password-generator") {
        setQuery("");
        setView("password-generator");
      } else {
        await hideWindow();
      }
    },
    [setView, setQuery],
  );

  const handleCommandComplete = useCallback(
    async (_result: CommandResult) => {
      // Reset UI immediately and hide window — notification is sent by backend
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

  // Initial values for wizard (from inline args)
  const [wizardInitialValues, setWizardInitialValues] = useState<
    Record<string, unknown>
  >({});

  // Select a command — execute inline if all required filled, otherwise open wizard
  const handleCommandSelect = useCallback(
    async (cmd: CommandDefinition, initVals?: Record<string, unknown>) => {
      const vals = initVals ?? {};
      const requiredParams = cmd.params.filter((p) => p.group === "Required");
      const allFilled = requiredParams.every((p) => vals[p.id] !== undefined);

      if (allFilled) {
        // Execute inline with spinner — no wizard flash
        setIsCommandExecuting(true);
        try {
          const result = await executeAction(cmd.id, vals);
          handleCommandComplete(result);
        } catch {
          // Fall through to wizard on error
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
      view === "param-wizard" ||
      mode === "command",
  });

  const { update, installUpdate, dismissUpdate } = useAutoUpdate();

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

  // Keyboard handler for command mode
  const handleCommandKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLInputElement>) => {
      if (mode !== "command") return;

      const { commandSuggestions, phase, labelRanges } = commandParser;

      if (phase === "selecting") {
        if (e.key === "Tab" || e.key === "Enter") {
          e.preventDefault();
          const result = commandParser.accept();
          if (result) {
            // accept() returned command + initialValues → open wizard
            handleCommandSelect(result.command, result.initialValues);
          }
          // If null, command was locked — query was updated, stay in input
        } else if (e.key === "Escape") {
          e.preventDefault();
          setQuery("");
        } else if (e.key === "ArrowDown") {
          e.preventDefault();
          commandParser.setSelectedIndex(
            Math.min(
              commandParser.selectedIndex + 1,
              commandSuggestions.length - 1,
            ),
          );
        } else if (e.key === "ArrowUp") {
          e.preventDefault();
          commandParser.setSelectedIndex(
            Math.max(commandParser.selectedIndex - 1, 0),
          );
        }
      } else {
        // Phase "args"

        // Autocomplete keyboard routing
        if (showAutocomplete) {
          if (e.key === "ArrowDown") {
            e.preventDefault();
            setAutocompleteIndex((i) =>
              Math.min(i + 1, autocompleteOptions.length - 1),
            );
            return;
          } else if (e.key === "ArrowUp") {
            e.preventDefault();
            setAutocompleteIndex((i) => Math.max(i - 1, 0));
            return;
          } else if (e.key === "Tab" || e.key === "Enter") {
            e.preventDefault();
            const opt = autocompleteOptions[autocompleteIndex];
            if (opt) {
              commandParser.selectAutocomplete(opt.label, opt.value);
              setAutocompleteOptions([]);
            }
            return;
          }
        }

        // Backspace: delete entire label as a unit
        if (e.key === "Backspace" && labelRanges.length > 0) {
          const cursorPos = query.length;
          for (const range of labelRanges) {
            // If backspacing would land inside a label, delete the whole label
            if (cursorPos > range.start && cursorPos <= range.end) {
              e.preventDefault();
              setQuery(query.slice(0, range.start));
              return;
            }
            // If cursor is right after the label's value region and the value
            // is already empty, delete the label too
          }
        }

        if (e.key === "Enter") {
          e.preventDefault();
          const result = commandParser.accept();
          if (result) {
            handleCommandSelect(result.command, result.initialValues);
          }
        } else if (e.key === "Tab") {
          e.preventDefault();
          commandParser.advanceParam();
        } else if (e.key === "Escape") {
          e.preventDefault();
          setQuery("");
        }
      }
    },
    [
      mode,
      commandParser,
      handleCommandSelect,
      setQuery,
      query,
      showAutocomplete,
      autocompleteOptions,
      autocompleteIndex,
    ],
  );

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

  let offset = 0;

  const isCommandMode = mode === "command" && view !== "param-wizard";
  const placeholder =
    view === "clipboard"
      ? "Filter clipboard history\u2026"
      : view === "notion"
        ? "Filter Notion pages\u2026"
        : view === "color-picker"
          ? "Palette name\u2026"
          : view === "password-generator"
            ? "Filter history\u2026"
            : mode === "command"
              ? "Type a command..."
              : "Search for apps and commands...";

  return (
    <div className="app-wrapper">
      <div className="app-shell">
        {view === "settings" ? (
          <Settings onBack={handleBack} />
        ) : view === "marketplace" ? (
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
                view === "clipboard" || view === "notion" || view === "color-picker" || view === "password-generator"
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
            {view === "param-wizard" && activeCommand ? (
              <ParamWizard
                command={activeCommand}
                onComplete={handleCommandComplete}
                onCancel={handleWizardBack}
                initialValues={wizardInitialValues}
              />
            ) : view === "clipboard" ? (
              <ClipboardManager
                filter={query}
                onBack={handleBack}
                onTrailingChange={setSearchTrailing}
              />
            ) : view === "notion" ? (
              <NotionView
                filter={query}
                onBack={handleBack}
                onTrailingChange={setSearchTrailing}
              />
            ) : view === "color-picker" ? (
              <ColorPicker
                filter={query}
                onBack={handleBack}
                onTrailingChange={setSearchTrailing}
                onQueryChange={setQuery}
              />
            ) : view === "password-generator" ? (
              <PasswordGenerator
                filter={query}
                onBack={handleBack}
                onTrailingChange={setSearchTrailing}
              />
            ) : isCommandMode ? (
              <>
                {commandParser.phase === "selecting" && (
                  <div className="results">
                    <CommandMode
                      commands={commandParser.commandSuggestions}
                      selectedIndex={commandParser.selectedIndex}
                      onItemClick={(i) => {
                        commandParser.setSelectedIndex(i);
                        const cmd = commandParser.commandSuggestions[i];
                        if (cmd) handleCommandSelect(cmd, {});
                      }}
                    />
                  </div>
                )}
                {showAutocomplete && (
                  <div className="args-autocomplete">
                    {autocompleteOptions.map((opt, i) => (
                      <div
                        key={opt.value}
                        className={`args-autocomplete-item ${i === autocompleteIndex ? "selected" : ""}`}
                        onClick={() => {
                          commandParser.selectAutocomplete(
                            opt.label,
                            opt.value,
                          );
                          setAutocompleteOptions([]);
                        }}
                      >
                        {opt.label}
                      </div>
                    ))}
                  </div>
                )}
                {lastResult &&
                  lastResult.follow_ups.length > 0 && (
                    <FollowUpBar
                      followUpIds={lastResult.follow_ups}
                      allCommands={allCommands}
                      onSelect={handleFollowUpSelect}
                    />
                  )}
                {showUndo && lastResult && (
                  <UndoToast
                    message={lastResult.message}
                    onDismiss={() => setShowUndo(false)}
                  />
                )}
              </>
            ) : (
              <>
                <div className="results">
                  {results.length === 0 ? (
                    <EmptyState />
                  ) : (
                    groups.map(([category, commands]) => {
                      const groupOffset = offset;
                      offset += commands.length;
                      return (
                        <ResultGroup
                          key={category}
                          category={category}
                          commands={commands}
                          selectedIndex={selectedIndex}
                          globalOffset={groupOffset}
                          onItemClick={handleItemClick}
                        />
                      );
                    })
                  )}
                </div>
                <Footer
                  onSettingsClick={() => setView("settings")}
                  onMarketplaceClick={() => {
                    setQuery("");
                    setView("marketplace");
                  }}
                />
              </>
            )}
          </>
        )}
      </div>
    </div>
  );
}

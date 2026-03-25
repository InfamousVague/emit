import { memo } from "react";
import { Settings } from "./Settings/Settings";
import { ClipboardManager } from "./ClipboardManager/ClipboardManager";
import { ColorPicker } from "./ColorPicker/ColorPicker";
import { PasswordGenerator } from "./PasswordGenerator/PasswordGeneratorView";
import { WindowManager } from "./WindowManager/WindowManager";
import { Screenshot } from "./Screenshot/Screenshot";
import { NotionView } from "./NotionView/NotionView";
import { ParamWizard } from "./ParamWizard/ParamWizard";
import { PerfDashboard } from "./PerfMonitor/PerfDashboard";
import { PortPilot } from "./PortPilot/PortPilot";
import { EnvVault } from "./EnvVault/EnvVault";
import { Bitwarden } from "./Bitwarden/Bitwarden";
import { CommandMode } from "./CommandMode/CommandMode";
import { FollowUpBar } from "./CommandMode/FollowUpBar";
import { UndoToast } from "./CommandMode/UndoToast";
import { ResultGroup } from "./ResultGroup/ResultGroup";
import { Footer } from "./Footer/Footer";
import { EmptyState } from "./EmptyState/EmptyState";
import type {
  CommandDefinition,
  CommandEntry,
  CommandResult,
  MetricSnapshot,
  SelectOption,
} from "../lib/types";
import type { CommandParserState } from "../hooks/useCommandParser";

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
  | "perf"
  | "port-pilot"
  | "env-vault"
  | "bitwarden";

interface AppViewRouterProps {
  view: View;
  query: string;
  setQuery: (q: string) => void;
  mode: string;
  // View handlers
  onBack: () => void;
  onWizardBack: () => void;
  onSettingsClick: () => void;
  onMarketplaceClick: () => void;
  onCheckForUpdates: () => void;
  onTrailingChange: (node: React.ReactNode) => void;
  // Command mode
  commandParser: CommandParserState;
  onCommandSelect: (cmd: CommandDefinition, initVals?: Record<string, unknown>) => void;
  autocompleteOptions: SelectOption[];
  autocompleteIndex: number;
  showAutocomplete: boolean;
  setAutocompleteOptions: (opts: SelectOption[]) => void;
  // Command state
  activeCommand: CommandDefinition | null;
  lastResult: CommandResult | null;
  allCommands: CommandDefinition[];
  showUndo: boolean;
  onCommandComplete: (result: CommandResult) => void;
  onFollowUpSelect: (cmd: CommandDefinition) => void;
  onUndoDismiss: () => void;
  wizardInitialValues: Record<string, unknown>;
  // Search results
  results: CommandEntry[];
  selectedIndex: number;
  groups: [string, CommandEntry[]][];
  onItemClick: (globalIndex: number) => void;
  perfHistory: MetricSnapshot[];
  perfScrollTo?: string;
}

export const AppViewRouter = memo(function AppViewRouter({
  view,
  query,
  setQuery,
  mode,
  onBack,
  onWizardBack,
  onSettingsClick,
  onMarketplaceClick,
  onCheckForUpdates,
  onTrailingChange,
  commandParser,
  onCommandSelect,
  autocompleteOptions,
  autocompleteIndex,
  showAutocomplete,
  setAutocompleteOptions,
  activeCommand,
  lastResult,
  allCommands,
  showUndo,
  onCommandComplete,
  onFollowUpSelect,
  onUndoDismiss,
  wizardInitialValues,
  results,
  selectedIndex,
  groups,
  onItemClick,
  perfHistory,
  perfScrollTo,
}: AppViewRouterProps) {
  const isCommandMode = mode === "command" && view !== "param-wizard";

  if (view === "param-wizard" && activeCommand) {
    return (
      <ParamWizard
        command={activeCommand}
        onComplete={onCommandComplete}
        onCancel={onWizardBack}
        initialValues={wizardInitialValues}
      />
    );
  }

  if (view === "clipboard") {
    return (
      <ClipboardManager
        filter={query}
        onBack={onBack}
        onTrailingChange={onTrailingChange}
      />
    );
  }

  if (view === "notion") {
    return (
      <NotionView
        filter={query}
        onBack={onBack}
        onTrailingChange={onTrailingChange}
      />
    );
  }

  if (view === "color-picker") {
    return (
      <ColorPicker
        filter={query}
        onBack={onBack}
        onTrailingChange={onTrailingChange}
        onQueryChange={setQuery}
      />
    );
  }

  if (view === "password-generator") {
    return (
      <PasswordGenerator
        filter={query}
        onBack={onBack}
        onTrailingChange={onTrailingChange}
      />
    );
  }

  if (view === "window-manager") {
    return (
      <WindowManager
        filter={query}
        onBack={onBack}
        onTrailingChange={onTrailingChange}
        onQueryChange={setQuery}
      />
    );
  }

  if (view === "screenshot") {
    return (
      <Screenshot
        filter={query}
        onBack={onBack}
        onTrailingChange={onTrailingChange}
      />
    );
  }

  if (view === "settings") {
    return (
      <Settings
        filter={query}
        onBack={onBack}
        onCheckForUpdates={onCheckForUpdates}
      />
    );
  }

  if (view === "port-pilot") {
    return (
      <PortPilot
        filter={query}
        onBack={onBack}
        onTrailingChange={onTrailingChange}
      />
    );
  }

  if (view === "env-vault") {
    return (
      <EnvVault
        filter={query}
        onBack={onBack}
        onTrailingChange={onTrailingChange}
      />
    );
  }

  if (view === "bitwarden") {
    return (
      <Bitwarden
        filter={query}
        onBack={onBack}
        onTrailingChange={onTrailingChange}
      />
    );
  }

  if (view === "perf") {
    return (
      <PerfDashboard
        filter={query}
        onBack={onBack}
        onTrailingChange={onTrailingChange}
        scrollToCard={perfScrollTo}
      />
    );
  }

  if (isCommandMode) {
    return (
      <>
        {commandParser.phase === "selecting" && (
          <div className="results">
            <CommandMode
              commands={commandParser.commandSuggestions}
              selectedIndex={commandParser.selectedIndex}
              onItemClick={(i) => {
                commandParser.setSelectedIndex(i);
                const cmd = commandParser.commandSuggestions[i];
                if (cmd) onCommandSelect(cmd, {});
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
                  commandParser.selectAutocomplete(opt.label, opt.value);
                  setAutocompleteOptions([]);
                }}
              >
                {opt.label}
              </div>
            ))}
          </div>
        )}
        {lastResult && lastResult.follow_ups.length > 0 && (
          <FollowUpBar
            followUpIds={lastResult.follow_ups}
            allCommands={allCommands}
            onSelect={onFollowUpSelect}
          />
        )}
        {showUndo && lastResult && (
          <UndoToast
            message={lastResult.message}
            onDismiss={onUndoDismiss}
          />
        )}
      </>
    );
  }

  // Default: search results
  let offset = 0;
  return (
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
                onItemClick={onItemClick}
                perfHistory={category === "Developer Tools" ? perfHistory : undefined}
              />
            );
          })
        )}
      </div>
      <Footer
        onSettingsClick={onSettingsClick}
        onMarketplaceClick={onMarketplaceClick}
      />
    </>
  );
});

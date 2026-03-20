import { useEffect, useMemo, useState } from "react";
import type { Settings as SettingsType } from "../../lib/types";
import { getSettings, saveSettings } from "../../lib/tauri";
import { Toggle, Select, Kbd, HighlightedText } from "../../ui";
import "./Settings.css";

interface SettingsProps {
  filter: string;
  onBack: () => void;
  onCheckForUpdates?: () => void;
}

export interface SettingDef {
  key: string;
  label: string;
  description: string;
}

export const SETTING_DEFS: SettingDef[] = [
  { key: "shortcut", label: "Activation Shortcut", description: "Keyboard shortcut to toggle Emit" },
  { key: "replace_spotlight", label: "Replace Spotlight", description: "Use Cmd+Space instead of Option+Space (disables Spotlight)" },
  { key: "launch_at_login", label: "Launch at Login", description: "Start Emit when you log in" },
  { key: "show_in_dock", label: "Show in Dock", description: "Display Emit icon in the macOS Dock" },
  { key: "check_for_updates", label: "Check for Updates", description: "Automatically check for new versions" },
  { key: "max_results", label: "Max Results", description: "Maximum number of search results to show" },
  { key: "ruler_shortcut", label: "Ruler Shortcut", description: "Global shortcut to activate Pixel Ruler" },
  { key: "ruler_snap_mode", label: "Ruler Snap Mode", description: "Snapping behavior: freehand or edge detection" },
  { key: "ruler_default_unit", label: "Ruler Default Unit", description: "Default measurement unit for the ruler" },
];

function substringIndices(text: string, query: string): number[] {
  if (!query) return [];
  const idx = text.toLowerCase().indexOf(query.toLowerCase());
  if (idx === -1) return [];
  return Array.from({ length: query.length }, (_, i) => idx + i);
}

export function Settings({ filter, onBack, onCheckForUpdates }: SettingsProps) {
  const [settings, setSettings] = useState<SettingsType | null>(null);

  useEffect(() => {
    getSettings().then(setSettings);
  }, []);

  const update = async (patch: Partial<SettingsType>) => {
    if (!settings) return;
    const next = { ...settings, ...patch };
    setSettings(next);
    await saveSettings(next);
  };

  const filtered = useMemo(() => {
    if (!filter) return SETTING_DEFS.map((s) => ({ ...s, labelIndices: [] as number[], descIndices: [] as number[] }));
    const q = filter.toLowerCase();
    return SETTING_DEFS
      .filter((s) => s.label.toLowerCase().includes(q) || s.description.toLowerCase().includes(q))
      .map((s) => ({
        ...s,
        labelIndices: substringIndices(s.label, filter),
        descIndices: substringIndices(s.description, filter),
      }));
  }, [filter]);

  if (!settings) return null;

  const renderControl = (key: string) => {
    switch (key) {
      case "shortcut":
        return <Kbd>{settings.replace_spotlight ? "\u2318 Space" : "\u2325 Space"}</Kbd>;
      case "replace_spotlight":
        return <Toggle checked={settings.replace_spotlight} onChange={(v) => update({ replace_spotlight: v })} />;
      case "launch_at_login":
        return <Toggle checked={settings.launch_at_login} onChange={(v) => update({ launch_at_login: v })} />;
      case "show_in_dock":
        return <Toggle checked={settings.show_in_dock} onChange={(v) => update({ show_in_dock: v })} />;
      case "check_for_updates":
        return (
          <div style={{ display: "flex", alignItems: "center", gap: "var(--space-sm)" }}>
            {onCheckForUpdates && (
              <button className="settings-action-btn" onClick={onCheckForUpdates}>
                Check Now
              </button>
            )}
            <Toggle checked={settings.check_for_updates} onChange={(v) => update({ check_for_updates: v })} />
          </div>
        );
      case "max_results":
        return (
          <Select
            value={settings.max_results}
            options={[10, 15, 20, 30, 50].map((n) => ({ value: n, label: String(n) }))}
            onChange={(v) => update({ max_results: Number(v) })}
          />
        );
      case "ruler_shortcut":
        return <Kbd>{settings.ruler_shortcut || "Shift+Cmd+R"}</Kbd>;
      case "ruler_snap_mode":
        return (
          <Select
            value={settings.ruler_snap_mode}
            options={[
              { value: "freehand", label: "Freehand" },
              { value: "edge", label: "Edge Detection" },
            ]}
            onChange={(v) => update({ ruler_snap_mode: String(v) })}
          />
        );
      case "ruler_default_unit":
        return (
          <Select
            value={settings.ruler_default_unit}
            options={[
              { value: "px", label: "Pixels (px)" },
              { value: "pt", label: "Points (pt)" },
              { value: "inches", label: "Inches" },
              { value: "rem", label: "REM" },
            ]}
            onChange={(v) => update({ ruler_default_unit: String(v) })}
          />
        );
      default:
        return null;
    }
  };

  return (
    <div className="settings">
      <div className="settings-body">
        {filtered.length === 0 ? (
          <div className="settings-empty">No matching settings</div>
        ) : (
          filtered.map((def) => (
            <div key={def.key} className="setting-row">
              <div className="setting-info">
                <span className="setting-label">
                  <HighlightedText text={def.label} indices={def.labelIndices} />
                </span>
                <span className="setting-desc">
                  <HighlightedText text={def.description} indices={def.descIndices} />
                </span>
              </div>
              <div className="setting-control">{renderControl(def.key)}</div>
            </div>
          ))
        )}
      </div>
    </div>
  );
}

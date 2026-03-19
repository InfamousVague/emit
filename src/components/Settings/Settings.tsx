import { useEffect, useMemo, useState } from "react";
import type { Settings as SettingsType } from "../../lib/types";
import { getSettings, saveSettings } from "../../lib/tauri";
import { Toggle, Select, Kbd, HighlightedText } from "../../ui";
import "./Settings.css";

interface SettingsProps {
  filter: string;
  onBack: () => void;
}

interface SettingDef {
  key: string;
  label: string;
  description: string;
}

const SETTING_DEFS: SettingDef[] = [
  { key: "shortcut", label: "Activation Shortcut", description: "Keyboard shortcut to toggle Emit" },
  { key: "launch_at_login", label: "Launch at Login", description: "Start Emit when you log in" },
  { key: "show_in_dock", label: "Show in Dock", description: "Display Emit icon in the macOS Dock" },
  { key: "check_for_updates", label: "Check for Updates", description: "Automatically check for new versions" },
  { key: "max_results", label: "Max Results", description: "Maximum number of search results to show" },
];

function substringIndices(text: string, query: string): number[] {
  if (!query) return [];
  const idx = text.toLowerCase().indexOf(query.toLowerCase());
  if (idx === -1) return [];
  return Array.from({ length: query.length }, (_, i) => idx + i);
}

export function Settings({ filter, onBack }: SettingsProps) {
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
        return <Kbd>{settings.shortcut}</Kbd>;
      case "launch_at_login":
        return <Toggle checked={settings.launch_at_login} onChange={(v) => update({ launch_at_login: v })} />;
      case "show_in_dock":
        return <Toggle checked={settings.show_in_dock} onChange={(v) => update({ show_in_dock: v })} />;
      case "check_for_updates":
        return <Toggle checked={settings.check_for_updates} onChange={(v) => update({ check_for_updates: v })} />;
      case "max_results":
        return (
          <Select
            value={settings.max_results}
            options={[10, 15, 20, 30, 50].map((n) => ({ value: n, label: String(n) }))}
            onChange={(v) => update({ max_results: Number(v) })}
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

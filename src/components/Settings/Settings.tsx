import { useEffect, useState } from "react";
import { ArrowLeft } from "@phosphor-icons/react";
import type { Settings as SettingsType } from "../../lib/types";
import { getSettings, saveSettings } from "../../lib/tauri";
import { Toggle, Select, Kbd } from "../../ui";
import "./Settings.css";

interface SettingsProps {
  onBack: () => void;
}

export function Settings({ onBack }: SettingsProps) {
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

  if (!settings) return null;

  return (
    <div className="settings">
      <div className="settings-header">
        <button className="view-back" onClick={onBack} aria-label="Back">
          <ArrowLeft size={18} weight="regular" />
        </button>
        <h2 className="settings-title">Settings</h2>
      </div>

      <div className="settings-body">
        <SettingRow
          label="Activation Shortcut"
          description="Keyboard shortcut to toggle Emit"
        >
          <Kbd>{settings.shortcut}</Kbd>
        </SettingRow>

        <SettingRow
          label="Launch at Login"
          description="Start Emit when you log in"
        >
          <Toggle
            checked={settings.launch_at_login}
            onChange={(v) => update({ launch_at_login: v })}
          />
        </SettingRow>

        <SettingRow
          label="Show in Dock"
          description="Display Emit icon in the macOS Dock"
        >
          <Toggle
            checked={settings.show_in_dock}
            onChange={(v) => update({ show_in_dock: v })}
          />
        </SettingRow>

        <SettingRow
          label="Check for Updates"
          description="Automatically check for new versions"
        >
          <Toggle
            checked={settings.check_for_updates}
            onChange={(v) => update({ check_for_updates: v })}
          />
        </SettingRow>

        <SettingRow
          label="Max Results"
          description="Maximum number of search results to show"
        >
          <Select
            value={settings.max_results}
            options={[10, 15, 20, 30, 50].map((n) => ({
              value: n,
              label: String(n),
            }))}
            onChange={(v) => update({ max_results: Number(v) })}
          />
        </SettingRow>
      </div>
    </div>
  );
}

/* ── Setting row layout primitive ── */

function SettingRow({
  label,
  description,
  children,
}: {
  label: string;
  description: string;
  children: React.ReactNode;
}) {
  return (
    <div className="setting-row">
      <div className="setting-info">
        <span className="setting-label">{label}</span>
        <span className="setting-desc">{description}</span>
      </div>
      <div className="setting-control">{children}</div>
    </div>
  );
}

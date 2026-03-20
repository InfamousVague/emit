import { useEffect, useMemo, useState } from "react";
import type { Settings as SettingsType, ShortcutBinding } from "../../lib/types";
import { getSettings, saveSettings, getShortcuts, rebindShortcut } from "../../lib/tauri";
import { Button, Toggle, Select, Kbd, HighlightedText, SectionHeader, ViewContainer } from "../../ui";
import { EmptyState } from "../EmptyState/EmptyState";
import { substringMatchIndices } from "../../lib/search";
import { ShortcutRecorder } from "./ShortcutRecorder";
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
  { key: "ruler_snap_mode", label: "Ruler Snap Mode", description: "Snapping behavior: freehand or edge detection" },
  { key: "ruler_default_unit", label: "Ruler Default Unit", description: "Default measurement unit for the ruler" },
];

export function Settings({ filter, onBack, onCheckForUpdates }: SettingsProps) {
  const [settings, setSettings] = useState<SettingsType | null>(null);
  const [shortcuts, setShortcuts] = useState<ShortcutBinding[]>([]);

  useEffect(() => {
    getSettings().then(setSettings);
    getShortcuts().then(setShortcuts);
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
        labelIndices: substringMatchIndices(s.label, filter),
        descIndices: substringMatchIndices(s.description, filter),
      }));
  }, [filter]);

  const handleRebind = async (id: string, keys: string) => {
    await rebindShortcut(id, keys);
    const updated = await getShortcuts();
    setShortcuts(updated);
  };

  // Filter shortcuts by search query
  const filteredShortcuts = useMemo(() => {
    if (!filter) return shortcuts;
    const q = filter.toLowerCase();
    return shortcuts.filter(
      (s) => s.label.toLowerCase().includes(q) || s.keys.toLowerCase().includes(q),
    );
  }, [filter, shortcuts]);

  // Check for conflicts between shortcuts
  const conflictMap = useMemo(() => {
    const map: Record<string, string> = {};
    const keyToLabel: Record<string, string> = {};
    for (const s of shortcuts) {
      if (keyToLabel[s.keys] && keyToLabel[s.keys] !== s.label) {
        map[s.id] = keyToLabel[s.keys];
      }
      keyToLabel[s.keys] = s.label;
    }
    return map;
  }, [shortcuts]);

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
              <Button size="sm" onClick={onCheckForUpdates}>
                Check Now
              </Button>
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

  const showShortcuts = filteredShortcuts.length > 0;
  const showSettings = filtered.length > 0;

  return (
    <ViewContainer>
      <ViewContainer.Body>
        {!showSettings && !showShortcuts ? (
          <EmptyState message="No matching settings" />
        ) : (
          <>
            {showSettings &&
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
              ))}
            {showShortcuts && (
              <>
                <SectionHeader className="settings-section-header">Keyboard Shortcuts</SectionHeader>
                {filteredShortcuts.map((s) => (
                  <div key={s.id} className="setting-row">
                    <div className="setting-info">
                      <span className="setting-label">{s.label}</span>
                      <span className="setting-desc">{s.extension_id}</span>
                    </div>
                    <div className="setting-control">
                      <ShortcutRecorder
                        value={s.keys}
                        onChange={(keys) => handleRebind(s.id, keys)}
                        conflict={conflictMap[s.id]}
                      />
                    </div>
                  </div>
                ))}
              </>
            )}
          </>
        )}
      </ViewContainer.Body>
    </ViewContainer>
  );
}

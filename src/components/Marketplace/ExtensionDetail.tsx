import { useEffect, useState } from "react";
import { Icon } from "@base/primitives/icon/Icon";
import { arrowLeft } from "../../lib/icons";
import {
  getExtensions,
  getExtensionSettings,
  saveExtensionSettings,
  setExtensionEnabled,
} from "../../lib/tauri";
import type { ExtensionInfo } from "../../lib/types";
import { Toggle } from "../../ui";
import "./ExtensionDetail.css";

interface ExtensionDetailProps {
  extensionId: string;
  onBack: () => void;
}

export function ExtensionDetail({ extensionId, onBack }: ExtensionDetailProps) {
  const [ext, setExt] = useState<ExtensionInfo | null>(null);
  const [settings, setSettings] = useState<Record<string, unknown>>({});

  useEffect(() => {
    getExtensions().then((exts) => {
      setExt(exts.find((e) => e.id === extensionId) ?? null);
    });
    getExtensionSettings(extensionId).then((s) =>
      setSettings(s as Record<string, unknown>),
    );
  }, [extensionId]);

  const handleToggle = async (enabled: boolean) => {
    await setExtensionEnabled(extensionId, enabled);
    setExt((prev) => (prev ? { ...prev, enabled } : null));
  };

  const updateSetting = async (key: string, value: unknown) => {
    const next = { ...settings, [key]: value };
    setSettings(next);
    await saveExtensionSettings(extensionId, next);
  };

  if (!ext) return null;

  return (
    <div className="ext-detail">
      <div className="ext-detail-header">
        <button className="view-back" onClick={onBack} aria-label="Back">
          <Icon icon={arrowLeft} size="sm" />
        </button>
        <h2 className="ext-detail-title">{ext.name}</h2>
      </div>

      <div className="ext-detail-body">
        <p className="ext-detail-desc">{ext.description}</p>

        <div className="setting-row">
          <div className="setting-info">
            <span className="setting-label">Enabled</span>
            <span className="setting-desc">
              {ext.enabled ? "Extension is active" : "Extension is disabled"}
            </span>
          </div>
          <div className="setting-control">
            <Toggle checked={ext.enabled} onChange={handleToggle} />
          </div>
        </div>

        {extensionId === "notion" && (
          <NotionSettings settings={settings} onUpdate={updateSetting} />
        )}
      </div>
    </div>
  );
}

function NotionSettings({
  settings,
  onUpdate,
}: {
  settings: Record<string, unknown>;
  onUpdate: (key: string, value: unknown) => void;
}) {
  const apiKey = (settings.api_key as string) ?? "";
  const defaultDb = (settings.default_database_id as string) ?? "";
  const savedFilters = (settings.saved_filters as Array<{
    name: string;
    status: string;
    assignee: string;
  }>) ?? [];

  return (
    <>
      <div className="ext-detail-section">
        <div className="ext-detail-section-title">Authentication</div>
        <div className="ext-input-row">
          <label className="ext-input-label">Internal Integration Token</label>
          <input
            className="ext-input"
            type="password"
            placeholder="secret_..."
            value={apiKey}
            onChange={(e) => onUpdate("api_key", e.target.value)}
          />
        </div>
      </div>

      <div className="ext-detail-section">
        <div className="ext-detail-section-title">Configuration</div>
        <div className="ext-input-row">
          <label className="ext-input-label">
            Default Database ID (optional)
          </label>
          <input
            className="ext-input"
            type="text"
            placeholder="Paste database ID..."
            value={defaultDb}
            onChange={(e) => onUpdate("default_database_id", e.target.value)}
          />
        </div>
      </div>

      {savedFilters.length > 0 && (
        <div className="ext-detail-section">
          <div className="ext-detail-section-title">Saved Filters</div>
          <div className="ext-saved-filters">
            {savedFilters.map((f, i) => (
              <div key={i} className="ext-filter-item">
                <span>{f.name}</span>
                <span className="ext-filter-item-meta">
                  {[f.status, f.assignee].filter(Boolean).join(" / ") ||
                    "No filters"}
                </span>
              </div>
            ))}
          </div>
        </div>
      )}
    </>
  );
}

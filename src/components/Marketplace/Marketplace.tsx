import { useEffect, useState } from "react";
import { Icon } from "@base/primitives/icon/Icon";
import { arrowLeft, puzzle } from "../../lib/icons";
import type { ExtensionInfo } from "../../lib/types";
import { getExtensions, setExtensionEnabled } from "../../lib/tauri";
import { Button, Toggle } from "../../ui";
import { EXTENSION_ICONS } from "../../assets/extension-icons";
import "./Marketplace.css";

interface MarketplaceProps {
  onBack: () => void;
  onExtensionClick: (id: string) => void;
  filter?: string;
}

export function Marketplace({
  onBack,
  onExtensionClick,
  filter = "",
}: MarketplaceProps) {
  const [extensions, setExtensions] = useState<ExtensionInfo[]>([]);

  useEffect(() => {
    getExtensions().then(setExtensions);
  }, []);

  const filtered = filter
    ? extensions.filter(
        (e) =>
          e.name.toLowerCase().includes(filter.toLowerCase()) ||
          e.description.toLowerCase().includes(filter.toLowerCase()),
      )
    : extensions;

  const handleToggle = async (id: string, enabled: boolean) => {
    await setExtensionEnabled(id, enabled);
    setExtensions((prev) =>
      prev.map((e) => (e.id === id ? { ...e, enabled } : e)),
    );
  };

  return (
    <div className="marketplace">
      <div className="marketplace-header">
        <Button variant="ghost" iconOnly icon={arrowLeft} aria-label="Back" onClick={onBack} />
        <h2 className="marketplace-title">Extensions</h2>
      </div>

      <div className="marketplace-body">
        {filtered.map((ext) => {
          const iconSrc = EXTENSION_ICONS[ext.id];
          return (
            <div
              key={ext.id}
              className="ext-card"
              onClick={() => onExtensionClick(ext.id)}
            >
              <div className="ext-card-icon">
                {iconSrc ? (
                  <img src={iconSrc} alt={ext.name} width={24} height={24} />
                ) : (
                  <Icon icon={puzzle} size="base" />
                )}
              </div>
              <div className="ext-card-info">
                <div className="ext-card-name">{ext.name}</div>
                <div className="ext-card-desc">{ext.description}</div>
              </div>
              <span className="ext-card-badge">{ext.category}</span>
              <div
                className="ext-card-toggle"
                onClick={(e) => e.stopPropagation()}
              >
                <Toggle
                  checked={ext.enabled}
                  onChange={(v) => handleToggle(ext.id, v)}
                />
              </div>
            </div>
          );
        })}
        {filtered.length === 0 && (
          <div style={{ color: "var(--color-text-tertiary)", padding: "var(--space-lg)", textAlign: "center" }}>
            No extensions found
          </div>
        )}
      </div>
    </div>
  );
}

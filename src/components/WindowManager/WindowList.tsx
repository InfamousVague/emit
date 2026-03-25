import { Icon } from "@base/primitives/icon/Icon";
import { appWindow } from "../../lib/icons";
import { useEffect, useState } from "react";
import type { WindowInfo } from "../../lib/types";
import { wmGetAppIcon } from "../../lib/tauri";

interface Props {
  windows: WindowInfo[];
  selectedId: number | null;
  filter: string;
  onSelect: (window: WindowInfo) => void;
}

export function WindowList({ windows, selectedId, filter, onSelect }: Props) {
  const [icons, setIcons] = useState<Record<string, string | null>>({});

  useEffect(() => {
    const appNames = [...new Set(windows.map((w) => w.app_name))];
    for (const name of appNames) {
      if (name in icons) continue;
      wmGetAppIcon(name).then((uri) => {
        setIcons((prev) => ({ ...prev, [name]: uri }));
      });
    }
  }, [windows]);

  const filtered = filter
    ? windows.filter(
        (w) =>
          w.app_name.toLowerCase().includes(filter.toLowerCase()) ||
          w.title.toLowerCase().includes(filter.toLowerCase()),
      )
    : windows;

  if (filtered.length === 0) {
    return (
      <div className="wm-list-empty">
        {filter ? "No matching windows" : "No windows found"}
      </div>
    );
  }

  return (
    <div className="wm-window-list">
      {filtered.map((w) => {
        const iconUri = icons[w.app_name] ?? null;
        return (
          <button
            key={w.window_id}
            className={`wm-window-item ${selectedId === w.window_id ? "selected" : ""}`}
            onClick={() => onSelect(w)}
          >
            <div className="wm-window-icon">
              {iconUri ? (
                <img src={iconUri} alt="" width={20} height={20} />
              ) : (
                <Icon icon={appWindow} size="base" />
              )}
            </div>
            <div className="wm-window-info">
              <span className="wm-window-app">{w.app_name}</span>
              {w.title && <span className="wm-window-title">{w.title}</span>}
            </div>
          </button>
        );
      })}
    </div>
  );
}

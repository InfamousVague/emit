import { useEffect, useRef, useState } from "react";
import { Kbd } from "./Kbd";

export interface Action {
  id: string;
  label: string;
  icon: React.ReactNode;
  shortcut: string[];
  action: () => void;
}

interface ActionBarProps {
  actions: Action[];
  open: boolean;
  onClose: () => void;
}

export function ActionBar({ actions, open, onClose }: ActionBarProps) {
  const [selectedIndex, setSelectedIndex] = useState(0);
  const popoverRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    setSelectedIndex(0);
  }, [open]);

  useEffect(() => {
    if (!open) return;

    const handler = (e: MouseEvent) => {
      if (
        popoverRef.current &&
        !popoverRef.current.contains(e.target as Node)
      ) {
        onClose();
      }
    };

    requestAnimationFrame(() => {
      document.addEventListener("mousedown", handler);
    });
    return () => document.removeEventListener("mousedown", handler);
  }, [open, onClose]);

  useEffect(() => {
    if (!open) return;

    const handler = (e: KeyboardEvent) => {
      switch (e.key) {
        case "ArrowDown":
          e.preventDefault();
          e.stopPropagation();
          setSelectedIndex((i) => Math.min(i + 1, actions.length - 1));
          break;
        case "ArrowUp":
          e.preventDefault();
          e.stopPropagation();
          setSelectedIndex((i) => Math.max(i - 1, 0));
          break;
        case "Enter":
          e.preventDefault();
          e.stopPropagation();
          actions[selectedIndex]?.action();
          onClose();
          break;
        case "Escape":
          e.preventDefault();
          e.stopPropagation();
          onClose();
          break;
      }
    };

    document.addEventListener("keydown", handler, true);
    return () => document.removeEventListener("keydown", handler, true);
  }, [open, actions, selectedIndex, onClose]);

  if (!open || actions.length === 0) return null;

  return (
    <div className="action-bar-popover" ref={popoverRef}>
      <div className="action-bar-list">
        {actions.map((action, i) => (
          <div
            key={action.id}
            className={`action-bar-item ${i === selectedIndex ? "selected" : ""}`}
            onClick={() => {
              action.action();
              onClose();
            }}
            onMouseEnter={() => setSelectedIndex(i)}
          >
            <span className="action-bar-icon">{action.icon}</span>
            <span className="action-bar-label">{action.label}</span>
            <span className="action-bar-shortcut">
              {action.shortcut.map((k, j) => (
                <Kbd key={j}>{k}</Kbd>
              ))}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}

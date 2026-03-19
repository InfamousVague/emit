import { useCallback, useEffect, useState } from "react";
import { AccessibilityPrompt } from "./AccessibilityPrompt";
import { LayoutGrid } from "./LayoutGrid";
import { WindowList } from "./WindowList";
import {
  hideWindow,
  wmCheckAccessibility,
  wmGetScreenInfo,
  wmListWindows,
  wmSnapFocused,
  wmSnapWindow,
} from "../../lib/tauri";
import type { ScreenInfo, SnapPosition, WindowInfo } from "../../lib/types";
import { TabNav } from "../../ui";
import type { Tab } from "../../ui";
import "./WindowManager.css";

type Mode = "grid" | "picker";

const WM_TABS: Tab[] = [
  { id: "grid", label: "Quick Snap" },
  { id: "picker", label: "Window Picker" },
];

interface Props {
  filter: string;
  onBack: () => void;
  onTrailingChange: (node: React.ReactNode) => void;
  onQueryChange: (q: string) => void;
}

export function WindowManager({ filter, onBack, onTrailingChange, onQueryChange }: Props) {
  const [accessible, setAccessible] = useState<boolean | null>(null);
  const [mode, setMode] = useState<Mode>("grid");
  const [windows, setWindows] = useState<WindowInfo[]>([]);
  const [selectedWindow, setSelectedWindow] = useState<WindowInfo | null>(null);
  const [screen, setScreen] = useState<ScreenInfo | null>(null);

  useEffect(() => {
    wmCheckAccessibility().then(setAccessible);
  }, []);

  useEffect(() => {
    if (!accessible) return;
    wmListWindows().then(setWindows);
    wmGetScreenInfo().then(setScreen);
  }, [accessible]);

  useEffect(() => {
    onTrailingChange(null);
    return () => onTrailingChange(null);
  }, [onTrailingChange]);

  const handleSnapGrid = useCallback(
    async (position: SnapPosition) => {
      await wmSnapFocused(position);
      await hideWindow();
    },
    [],
  );

  const handleSnapPicker = useCallback(
    async (position: SnapPosition) => {
      if (!selectedWindow) return;
      await wmSnapWindow(selectedWindow.window_id, position);
      await hideWindow();
    },
    [selectedWindow],
  );

  if (accessible === null) return null;

  if (!accessible) {
    return (
      <div className="window-manager">
        <AccessibilityPrompt onGranted={() => setAccessible(true)} />
      </div>
    );
  }

  return (
    <div className="window-manager">
      <TabNav tabs={WM_TABS} activeId={mode} onChange={(id) => setMode(id as Mode)} />

      {mode === "grid" && (
        <div className="wm-body-full">
          <LayoutGrid onSnap={handleSnapGrid} screen={screen} />
        </div>
      )}

      {mode === "picker" && (
        <div className="wm-picker">
          <div className="wm-picker-left">
            <WindowList
              windows={windows}
              selectedId={selectedWindow?.window_id ?? null}
              filter={filter}
              onSelect={setSelectedWindow}
            />
          </div>
          <div className="wm-picker-right">
            <LayoutGrid
              onSnap={handleSnapPicker}
              selectedPosition={null}
              screen={screen}
            />
          </div>
        </div>
      )}
    </div>
  );
}

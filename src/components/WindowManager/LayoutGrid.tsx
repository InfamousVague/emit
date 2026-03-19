import type { ScreenInfo, SnapPosition } from "../../lib/types";

interface SnapCell {
  position: SnapPosition;
  label: string;
  zone: { x: number; y: number; w: number; h: number };
}

const GRID_ROWS: SnapCell[][] = [
  [
    { position: "LeftHalf", label: "Left Half", zone: { x: 0, y: 0, w: 0.5, h: 1 } },
    { position: "RightHalf", label: "Right Half", zone: { x: 0.5, y: 0, w: 0.5, h: 1 } },
    { position: "TopHalf", label: "Top Half", zone: { x: 0, y: 0, w: 1, h: 0.5 } },
    { position: "BottomHalf", label: "Bottom Half", zone: { x: 0, y: 0.5, w: 1, h: 0.5 } },
  ],
  [
    { position: "TopLeftQuarter", label: "Top Left", zone: { x: 0, y: 0, w: 0.5, h: 0.5 } },
    { position: "TopRightQuarter", label: "Top Right", zone: { x: 0.5, y: 0, w: 0.5, h: 0.5 } },
    { position: "BottomLeftQuarter", label: "Bottom Left", zone: { x: 0, y: 0.5, w: 0.5, h: 0.5 } },
    { position: "BottomRightQuarter", label: "Bottom Right", zone: { x: 0.5, y: 0.5, w: 0.5, h: 0.5 } },
  ],
  [
    { position: "LeftThird", label: "Left Third", zone: { x: 0, y: 0, w: 1 / 3, h: 1 } },
    { position: "CenterThird", label: "Center Third", zone: { x: 1 / 3, y: 0, w: 1 / 3, h: 1 } },
    { position: "RightThird", label: "Right Third", zone: { x: 2 / 3, y: 0, w: 1 / 3, h: 1 } },
    { position: "Maximize", label: "Maximize", zone: { x: 0, y: 0, w: 1, h: 1 } },
  ],
  [
    { position: "LeftTwoThirds", label: "Left 2/3", zone: { x: 0, y: 0, w: 2 / 3, h: 1 } },
    { position: "RightTwoThirds", label: "Right 2/3", zone: { x: 1 / 3, y: 0, w: 2 / 3, h: 1 } },
    { position: "Center", label: "Center", zone: { x: 0.15, y: 0.15, w: 0.7, h: 0.7 } },
  ],
];

interface Props {
  onSnap: (position: SnapPosition) => void;
  selectedPosition?: SnapPosition | null;
  screen?: ScreenInfo | null;
}

export function LayoutGrid({ onSnap, selectedPosition, screen }: Props) {
  // Insets as percentages to account for menu bar and dock
  const menuInset = screen?.is_primary ? 10 : 0; // % from top
  const dockSize = 8; // % for dock thickness
  const dockInset = screen?.dock_position
    ? { bottom: screen.dock_position === "bottom" ? dockSize : 0,
        left: screen.dock_position === "left" ? dockSize : 0,
        right: screen.dock_position === "right" ? dockSize : 0 }
    : { bottom: 0, left: 0, right: 0 };

  return (
    <div className="wm-grid">
      {GRID_ROWS.map((row, ri) => (
        <div key={ri} className="wm-grid-row">
          {row.map((cell) => {
            const isSelected = selectedPosition === cell.position;
            // Map zone coordinates into the usable area (excluding menu bar + dock)
            const usableLeft = dockInset.left;
            const usableTop = menuInset;
            const usableWidth = 100 - dockInset.left - dockInset.right;
            const usableHeight = 100 - menuInset - dockInset.bottom;

            const zoneLeft = usableLeft + cell.zone.x * usableWidth;
            const zoneTop = usableTop + cell.zone.y * usableHeight;
            const zoneWidth = cell.zone.w * usableWidth;
            const zoneHeight = cell.zone.h * usableHeight;

            return (
              <button
                key={cell.position}
                className={`wm-grid-cell ${isSelected ? "selected" : ""}`}
                onClick={() => onSnap(cell.position)}
                title={cell.label}
              >
                <div className="wm-screen">
                  {screen?.is_primary && <div className="wm-menubar" />}
                  {screen?.dock_position && (
                    <div className={`wm-dock wm-dock--${screen.dock_position}`} />
                  )}
                  <div
                    className="wm-zone"
                    style={{
                      left: `calc(${zoneLeft}% + 2px)`,
                      top: `calc(${zoneTop}% + 2px)`,
                      width: `calc(${zoneWidth}% - 4px)`,
                      height: `calc(${zoneHeight}% - 4px)`,
                    }}
                  />
                </div>
                <span className="wm-grid-label">{cell.label}</span>
              </button>
            );
          })}
        </div>
      ))}
    </div>
  );
}

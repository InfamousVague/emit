import { invoke } from "@tauri-apps/api/core";
import type { Measurement, Unit } from "./types";
import { formatLabel } from "./geometry";

interface ControlPanelProps {
  measurements: Measurement[];
  unit: Unit;
  onUnitChange: (unit: Unit) => void;
  onClearAll: () => void;
}

const UNITS: Unit[] = ["px", "pt", "inches", "rem"];

export function ControlPanel({
  measurements,
  unit,
  onUnitChange,
  onClearAll,
}: ControlPanelProps) {
  async function handleCopy() {
    const text = measurements
      .map((m) => formatLabel(m.start, m.end, unit))
      .join("\n");
    await invoke("ruler_copy_measurements", { data: text });
  }

  async function handleScreenshot() {
    await invoke("ruler_screenshot_overlay");
  }

  return (
    <div className="ruler-control-wrapper">
      <div className="ruler-control-panel">
        <div className="ruler-control-row">
          <span className="ruler-control-count">
            {measurements.length} measurement{measurements.length !== 1 ? "s" : ""}
          </span>
          <select
            className="ruler-control-select"
            value={unit}
            onChange={(e) => onUnitChange(e.target.value as Unit)}
          >
            {UNITS.map((u) => (
              <option key={u} value={u}>
                {u}
              </option>
            ))}
          </select>
        </div>
        <div className="ruler-control-row">
          {measurements.length > 0 && (
            <>
              <button className="ruler-control-btn" onClick={handleCopy}>
                Copy
              </button>
              <button className="ruler-control-btn" onClick={handleScreenshot}>
                Screenshot
              </button>
              <button
                className="ruler-control-btn ruler-control-btn-danger"
                onClick={onClearAll}
              >
                Clear All
              </button>
            </>
          )}
        </div>
        <div className="ruler-control-hint">
          Shift: constrain · Esc: cancel · Esc×2: close
        </div>
      </div>
    </div>
  );
}

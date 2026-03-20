import type { TimeRange } from "../../lib/types";
import "./PerfDashboard.css";

const RANGES: TimeRange[] = ["1m", "5m", "15m", "1hr"];

interface Props {
  value: TimeRange;
  onChange: (range: TimeRange) => void;
}

export function TimeRangeSelector({ value, onChange }: Props) {
  return (
    <div className="time-range-selector">
      {RANGES.map((r) => (
        <button
          key={r}
          className={`time-range-btn${r === value ? " time-range-btn--active" : ""}`}
          onClick={() => onChange(r)}
        >
          {r}
        </button>
      ))}
    </div>
  );
}

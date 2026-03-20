import type { MetricSnapshot } from "../../../lib/types";
import { thresholdColor } from "../utils";

interface Props {
  snapshot: MetricSnapshot | null;
  history: MetricSnapshot[];
}

export function BatteryCard({ snapshot }: Props) {
  const battery = snapshot?.battery;

  if (!battery) {
    return (
      <>
        <div className="perf-card__header">
          <span className="perf-card__title">Battery</span>
        </div>
        <span className="perf-card__empty">No battery</span>
      </>
    );
  }

  const chargeColor = thresholdColor(100 - battery.charge_percent);
  const icon = battery.is_charging ? "\u26A1" : "\u{1F50B}";

  return (
    <>
      <div className="perf-card__header">
        <span className="perf-card__title">
          {icon} Battery
        </span>
        <span className="perf-card__value" style={{ color: chargeColor }}>
          {battery.charge_percent.toFixed(0)}%
        </span>
      </div>

      <div className="perf-battery-grid">
        <div className="perf-battery-stat">
          <span className="perf-battery-stat__label">Health</span>
          <span className="perf-battery-stat__value">{battery.health_percent.toFixed(0)}%</span>
        </div>
        <div className="perf-battery-stat">
          <span className="perf-battery-stat__label">Temp</span>
          <span className="perf-battery-stat__value">{battery.temperature.toFixed(1)}&deg;C</span>
        </div>
        <div className="perf-battery-stat">
          <span className="perf-battery-stat__label">Cycles</span>
          <span className="perf-battery-stat__value">{battery.cycle_count}</span>
        </div>
        <div className="perf-battery-stat">
          <span className="perf-battery-stat__label">Power</span>
          <span className="perf-battery-stat__value">{battery.power_draw.toFixed(1)}W</span>
        </div>
      </div>
    </>
  );
}

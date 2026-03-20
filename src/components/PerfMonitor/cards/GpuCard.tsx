import { memo } from "react";
import type { MetricSnapshot } from "../../../lib/types";
import { thresholdColor } from "../utils";

interface Props {
  snapshot: MetricSnapshot | null;
  history: MetricSnapshot[];
}

export const GpuCard = memo(function GpuCard({ snapshot }: Props) {
  const gpu = snapshot?.gpu;

  if (!gpu) {
    return (
      <>
        <div className="perf-card__header">
          <span className="perf-card__title">GPU</span>
        </div>
        <span className="perf-card__empty">No GPU data</span>
      </>
    );
  }

  const color = thresholdColor(gpu.utilization);

  return (
    <>
      <div className="perf-card__header">
        <span className="perf-card__title">GPU</span>
        <span className="perf-card__value" style={{ color }}>
          {gpu.utilization.toFixed(0)}%
        </span>
      </div>

      <div className="perf-card__subtitle">{gpu.name}</div>

      <div className="perf-progress">
        <div
          className="perf-progress__fill"
          style={{ width: `${gpu.utilization}%`, backgroundColor: color }}
        />
      </div>
    </>
  );
});

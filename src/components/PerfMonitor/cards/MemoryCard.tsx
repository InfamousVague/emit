import type { MetricSnapshot } from "../../../lib/types";
import { formatBytes, thresholdColor } from "../utils";

interface Props {
  snapshot: MetricSnapshot | null;
  history: MetricSnapshot[];
}

export function MemoryCard({ snapshot }: Props) {
  const mem = snapshot?.memory;
  if (!mem) {
    return (
      <>
        <div className="perf-card__header">
          <span className="perf-card__title">Memory</span>
        </div>
        <span className="perf-card__empty">Waiting for data...</span>
      </>
    );
  }

  const usedPercent = (mem.used / mem.total) * 100;
  const color = thresholdColor(usedPercent);

  return (
    <>
      <div className="perf-card__header">
        <span className="perf-card__title">Memory</span>
        <span className="perf-card__value" style={{ color }}>
          {usedPercent.toFixed(1)}%
        </span>
      </div>

      <div className="perf-progress">
        <div
          className="perf-progress__fill"
          style={{ width: `${usedPercent}%`, backgroundColor: color }}
        />
      </div>

      <div className="perf-card__subtitle">
        {formatBytes(mem.used)} / {formatBytes(mem.total)}
      </div>

      <div className="perf-card__breakdown">
        <span>App: {formatBytes(mem.app_memory)}</span>
        <span>Wired: {formatBytes(mem.wired)}</span>
        <span>Compressed: {formatBytes(mem.compressed)}</span>
      </div>
    </>
  );
}

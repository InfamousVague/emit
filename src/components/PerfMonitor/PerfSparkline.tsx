import type { MetricSnapshot } from "../../lib/types";
import { thresholdColor } from "./utils";

interface Props {
  history: MetricSnapshot[];
  metric: string;
  width?: number;
  height?: number;
}

function extractValues(history: MetricSnapshot[], metric: string): number[] {
  switch (metric) {
    case "cpu":
      return history.map((s) => s.cpu.total_usage);
    case "memory":
      return history.map((s) =>
        s.memory.total > 0 ? (s.memory.used / s.memory.total) * 100 : 0,
      );
    case "gpu":
      return history.map((s) => s.gpu?.utilization ?? 0);
    case "network":
      return history.map((s) => s.network.download_speed);
    case "battery":
      return history.map((s) => s.battery?.charge_percent ?? 0);
    case "disk":
      return history.map((s) => {
        const d = s.disks[0];
        return d && d.total > 0 ? (d.used / d.total) * 100 : 0;
      });
    default:
      return [];
  }
}

export function PerfSparkline({
  history,
  metric,
  width = 80,
  height = 24,
}: Props) {
  const values = extractValues(history, metric);
  if (values.length < 2) return null;

  // Take last 60 points max
  const data = values.slice(-60);
  const max = Math.max(...data, 1);
  const min = Math.min(...data, 0);
  const range = max - min || 1;

  const padding = 1;
  const chartW = width - padding * 2;
  const chartH = height - padding * 2;

  const points = data
    .map((v, i) => {
      const x = padding + (i / (data.length - 1)) * chartW;
      const y = padding + chartH - ((v - min) / range) * chartH;
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(" ");

  // Current value determines color
  const current = data[data.length - 1];
  const color = metric === "network" ? "var(--perf-green)" : thresholdColor(current);

  // Fill path: close to bottom
  const fillPoints =
    `${padding},${padding + chartH} ` +
    points +
    ` ${padding + chartW},${padding + chartH}`;

  return (
    <svg
      width={width}
      height={height}
      viewBox={`0 0 ${width} ${height}`}
      className="perf-sparkline"
    >
      <polygon
        points={fillPoints}
        fill={color}
        fillOpacity={0.15}
      />
      <polyline
        points={points}
        fill="none"
        stroke={color}
        strokeWidth={1.5}
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

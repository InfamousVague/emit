import {
  Area,
  AreaChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import type { MetricSnapshot } from "../../../lib/types";
import { thresholdColor } from "../utils";

interface Props {
  snapshot: MetricSnapshot | null;
  history: MetricSnapshot[];
}

export function CpuCard({ snapshot, history }: Props) {
  const totalUsage = snapshot?.cpu.total_usage ?? 0;
  const cores = snapshot?.cpu.per_core ?? [];
  const color = thresholdColor(totalUsage);

  const chartData = history.map((s) => ({
    time: s.timestamp,
    cpu: s.cpu.total_usage,
  }));

  return (
    <>
      <div className="perf-card__header">
        <span className="perf-card__title">CPU</span>
        <span className="perf-card__value" style={{ color }}>
          {totalUsage.toFixed(1)}%
        </span>
      </div>

      <div className="perf-card__chart">
        <ResponsiveContainer width="100%" height={120}>
          <AreaChart data={chartData} margin={{ top: 4, right: 4, bottom: 0, left: -20 }}>
            <defs>
              <linearGradient id="cpuGrad" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%" stopColor={color} stopOpacity={0.3} />
                <stop offset="100%" stopColor={color} stopOpacity={0} />
              </linearGradient>
            </defs>
            <XAxis dataKey="time" hide />
            <YAxis
              domain={[0, 100]}
              tick={{ fill: "rgba(255,255,255,0.5)", fontSize: 10 }}
              tickLine={false}
              axisLine={false}
            />
            <Tooltip
              contentStyle={{
                background: "rgba(0,0,0,0.85)",
                border: "1px solid rgba(255,255,255,0.1)",
                borderRadius: 6,
                fontSize: 11,
              }}
              labelFormatter={() => ""}
              formatter={(v) => [`${Number(v).toFixed(1)}%`, "CPU"]}
            />
            <Area
              type="monotone"
              dataKey="cpu"
              stroke={color}
              strokeWidth={1.5}
              fill="url(#cpuGrad)"
              isAnimationActive={false}
            />
          </AreaChart>
        </ResponsiveContainer>
      </div>

      <div className="perf-core-grid">
        {cores.map((usage, i) => {
          const coreColor = thresholdColor(usage);
          return (
            <div key={i} className="perf-core-bar">
              <div
                className="perf-core-bar__fill"
                style={{
                  height: `${usage}%`,
                  backgroundColor: coreColor,
                }}
              />
              <span className="perf-core-bar__label">{i}</span>
            </div>
          );
        })}
      </div>
    </>
  );
}

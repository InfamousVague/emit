import { useEffect, useRef } from "react";
import { usePerfMonitor } from "../../hooks/usePerfMonitor";
import { TimeRangeSelector } from "./TimeRangeSelector";
import { CpuCard } from "./cards/CpuCard";
import { MemoryCard } from "./cards/MemoryCard";
import { NetworkCard } from "./cards/NetworkCard";
import { GpuCard } from "./cards/GpuCard";
import { BatteryCard } from "./cards/BatteryCard";
import { DiskCard } from "./cards/DiskCard";
import { ProcessCard } from "./cards/ProcessCard";
import { formatUptime } from "./utils";
import "./PerfDashboard.css";

interface Props {
  filter: string;
  onBack: () => void;
  onTrailingChange: (node: React.ReactNode) => void;
  scrollToCard?: string;
}

export function PerfDashboard({
  onTrailingChange,
  scrollToCard,
}: Props) {
  const { snapshot, history, timeRange, setTimeRange } = usePerfMonitor();
  const gridRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    onTrailingChange(null);
    return () => onTrailingChange(null);
  }, [onTrailingChange]);

  useEffect(() => {
    if (!scrollToCard || !gridRef.current) return;
    const el = gridRef.current.querySelector(
      `[data-card="${scrollToCard}"]`,
    );
    el?.scrollIntoView({ behavior: "smooth", block: "center" });
  }, [scrollToCard]);

  const uptime = snapshot?.system.uptime_secs;
  const loadAvg = snapshot?.cpu;
  const cardProps = { snapshot, history };

  return (
    <div className="perf-dashboard">
      <div className="perf-header">
        <div className="perf-header__left">
          {uptime != null && (
            <span className="perf-header__uptime">
              Uptime: {formatUptime(uptime)}
            </span>
          )}
          {loadAvg && (
            <span className="perf-header__load">
              Load: {loadAvg.load_avg_1.toFixed(2)}{" "}
              {loadAvg.load_avg_5.toFixed(2)}{" "}
              {loadAvg.load_avg_15.toFixed(2)}
            </span>
          )}
        </div>
        <TimeRangeSelector value={timeRange} onChange={setTimeRange} />
      </div>

      <div className="perf-grid" ref={gridRef}>
        {/* CpuCard: 2x2 */}
        <div className="perf-card perf-card--2x2" data-card="cpu">
          <CpuCard {...cardProps} />
        </div>
        {/* MemoryCard: 2x1 */}
        <div className="perf-card perf-card--2x1" data-card="memory">
          <MemoryCard {...cardProps} />
        </div>
        {/* NetworkCard: 1x1 */}
        <div className="perf-card" data-card="network">
          <NetworkCard {...cardProps} />
        </div>
        {/* GpuCard: 1x1 */}
        <div className="perf-card" data-card="gpu">
          <GpuCard {...cardProps} />
        </div>
        {/* BatteryCard: 2x1 */}
        <div className="perf-card perf-card--2x1" data-card="battery">
          <BatteryCard {...cardProps} />
        </div>
        {/* DiskCard: full width */}
        <div className="perf-card perf-card--wide" data-card="disk">
          <DiskCard {...cardProps} />
        </div>
        {/* ProcessCard: full width */}
        <div className="perf-card perf-card--wide" data-card="processes">
          <ProcessCard {...cardProps} />
        </div>
      </div>
    </div>
  );
}

import type { CommandEntry, MetricSnapshot } from "../../lib/types";
import { ResultItem } from "../ResultItem/ResultItem";
import { PerfSparkline } from "../PerfMonitor/PerfSparkline";
import "./ResultGroup.css";

const PERF_METRIC_MAP: Record<string, string> = {
  "perf.cpu": "cpu",
  "perf.memory": "memory",
  "perf.disk": "disk",
  "perf.network": "network",
  "perf.gpu": "gpu",
  "perf.battery": "battery",
};

interface ResultGroupProps {
  category: string;
  commands: CommandEntry[];
  selectedIndex: number;
  globalOffset: number;
  onItemClick: (index: number) => void;
  perfHistory?: MetricSnapshot[];
}

export function ResultGroup({
  category,
  commands,
  selectedIndex,
  globalOffset,
  onItemClick,
  perfHistory,
}: ResultGroupProps) {
  return (
    <div className="result-group">
      <div className="result-section">{category}</div>
      {commands.map((cmd, i) => {
        const globalIndex = globalOffset + i;
        const metric = PERF_METRIC_MAP[cmd.id];
        const trailing =
          metric && perfHistory && perfHistory.length >= 2 ? (
            <PerfSparkline history={perfHistory} metric={metric} />
          ) : undefined;
        return (
          <ResultItem
            key={cmd.id}
            command={cmd}
            isSelected={globalIndex === selectedIndex}
            onClick={() => onItemClick(globalIndex)}
            trailing={trailing}
          />
        );
      })}
    </div>
  );
}

import { useCallback, useEffect, useState } from "react";
import { perfGetProcesses } from "../../../lib/tauri";
import type { MetricSnapshot, ProcessInfo } from "../../../lib/types";
import { formatBytes } from "../utils";

type SortField = "cpu" | "memory" | "pid" | "name";

interface Props {
  snapshot: MetricSnapshot | null;
  history: MetricSnapshot[];
}

export function ProcessCard({ snapshot }: Props) {
  const [processes, setProcesses] = useState<ProcessInfo[]>([]);
  const [sortField, setSortField] = useState<SortField>("cpu");
  const [sortAsc, setSortAsc] = useState(false);

  const fetchProcesses = useCallback(async () => {
    try {
      const sortBy = sortField === "memory" ? "memory" : "cpu";
      const data = await perfGetProcesses(sortBy, 10);
      setProcesses(data);
    } catch {
      /* backend may not be ready */
    }
  }, [sortField]);

  useEffect(() => {
    fetchProcesses();
  }, [fetchProcesses, snapshot?.timestamp]);

  const sorted = [...processes].sort((a, b) => {
    const dir = sortAsc ? 1 : -1;
    switch (sortField) {
      case "pid":
        return (a.pid - b.pid) * dir;
      case "name":
        return a.name.localeCompare(b.name) * dir;
      case "cpu":
        return (a.cpu_usage - b.cpu_usage) * dir;
      case "memory":
        return (a.memory_bytes - b.memory_bytes) * dir;
      default:
        return 0;
    }
  });

  const handleSort = (field: SortField) => {
    if (field === sortField) {
      setSortAsc((prev) => !prev);
    } else {
      setSortField(field);
      setSortAsc(false);
    }
  };

  const sortIndicator = (field: SortField) => {
    if (field !== sortField) return null;
    return sortAsc ? " \u25B2" : " \u25BC";
  };

  return (
    <>
      <div className="perf-card__header">
        <span className="perf-card__title">Processes</span>
      </div>

      <table className="perf-process-table">
        <thead>
          <tr>
            <th onClick={() => handleSort("pid")}>
              PID{sortIndicator("pid")}
            </th>
            <th onClick={() => handleSort("name")}>
              Name{sortIndicator("name")}
            </th>
            <th onClick={() => handleSort("cpu")}>
              CPU%{sortIndicator("cpu")}
            </th>
            <th onClick={() => handleSort("memory")}>
              RAM{sortIndicator("memory")}
            </th>
          </tr>
        </thead>
        <tbody>
          {sorted.map((proc) => (
            <tr key={proc.pid}>
              <td>{proc.pid}</td>
              <td className="perf-process-name">{proc.name}</td>
              <td>{proc.cpu_usage.toFixed(1)}%</td>
              <td>{formatBytes(proc.memory_bytes)}</td>
            </tr>
          ))}
          {sorted.length === 0 && (
            <tr>
              <td colSpan={4} className="perf-card__empty">
                Waiting for data...
              </td>
            </tr>
          )}
        </tbody>
      </table>
    </>
  );
}

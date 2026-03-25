import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type { MetricSnapshot, TimeRange } from "../lib/types";

const TIME_RANGE_SECONDS: Record<TimeRange, number> = {
  "1m": 60,
  "5m": 300,
  "15m": 900,
  "1hr": 3600,
};

export function usePerfMonitor() {
  const [snapshot, setSnapshot] = useState<MetricSnapshot | null>(null);
  const [history, setHistory] = useState<MetricSnapshot[]>([]);
  const [timeRange, setTimeRange] = useState<TimeRange>("1m");
  const timeRangeRef = useRef(timeRange);

  useEffect(() => {
    timeRangeRef.current = timeRange;
  }, [timeRange]);

  const trimHistory = useCallback(
    (items: MetricSnapshot[]): MetricSnapshot[] => {
      const cutoffMs =
        Date.now() - TIME_RANGE_SECONDS[timeRangeRef.current] * 1000;
      const filtered = items.filter((s) => s.timestamp >= cutoffMs);
      // Hard cap to prevent unbounded growth regardless of time filtering
      const MAX_HISTORY = 1800; // 15 min at 2/sec
      if (filtered.length > MAX_HISTORY) {
        return filtered.slice(filtered.length - MAX_HISTORY);
      }
      return filtered;
    },
    [],
  );

  useEffect(() => {
    const unlisten = listen<MetricSnapshot>("perf-update", (event) => {
      const data = event.payload;
      setSnapshot(data);
      setHistory((prev) => trimHistory([...prev, data]));
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [trimHistory]);

  useEffect(() => {
    setHistory((prev) => trimHistory(prev));
  }, [timeRange, trimHistory]);

  return { snapshot, history, timeRange, setTimeRange };
}

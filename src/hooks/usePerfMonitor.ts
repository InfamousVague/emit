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
      const cutoff =
        Date.now() / 1000 - TIME_RANGE_SECONDS[timeRangeRef.current];
      return items.filter((s) => s.timestamp >= cutoff);
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

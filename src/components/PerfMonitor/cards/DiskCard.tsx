import { memo } from "react";
import type { MetricSnapshot } from "../../../lib/types";
import { formatBytes, thresholdColor } from "../utils";

interface Props {
  snapshot: MetricSnapshot | null;
  history: MetricSnapshot[];
}

export const DiskCard = memo(function DiskCard({ snapshot }: Props) {
  const disks = snapshot?.disks ?? [];

  if (disks.length === 0) {
    return (
      <>
        <div className="perf-card__header">
          <span className="perf-card__title">Disks</span>
        </div>
        <span className="perf-card__empty">Waiting for data...</span>
      </>
    );
  }

  return (
    <>
      <div className="perf-card__header">
        <span className="perf-card__title">Disks</span>
      </div>

      <div className="perf-disk-list">
        {disks.map((disk) => {
          const usedPercent = (disk.used / disk.total) * 100;
          const color = thresholdColor(usedPercent);

          return (
            <div key={disk.mount_point} className="perf-disk-row">
              <div className="perf-disk-info">
                <span className="perf-disk-name">{disk.name}</span>
                <span className="perf-disk-mount">{disk.mount_point}</span>
              </div>
              <div className="perf-disk-bar-wrapper">
                <div className="perf-progress">
                  <div
                    className="perf-progress__fill"
                    style={{ width: `${usedPercent}%`, backgroundColor: color }}
                  />
                </div>
              </div>
              <span className="perf-disk-usage">
                {formatBytes(disk.used)} / {formatBytes(disk.total)}
              </span>
            </div>
          );
        })}
      </div>
    </>
  );
});

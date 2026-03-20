import { memo } from "react";
import type { MetricSnapshot } from "../../../lib/types";
import { formatBytes, formatBytesPerSec } from "../utils";

interface Props {
  snapshot: MetricSnapshot | null;
  history: MetricSnapshot[];
}

export const NetworkCard = memo(function NetworkCard({ snapshot }: Props) {
  const net = snapshot?.network;
  if (!net) {
    return (
      <>
        <div className="perf-card__header">
          <span className="perf-card__title">Network</span>
        </div>
        <span className="perf-card__empty">Waiting for data...</span>
      </>
    );
  }

  return (
    <>
      <div className="perf-card__header">
        <span className="perf-card__title">Network</span>
      </div>

      <div className="perf-net-speeds">
        <div className="perf-net-speed">
          <span className="perf-net-arrow perf-net-arrow--up">&#9650;</span>
          <span>{formatBytesPerSec(net.upload_speed)}</span>
        </div>
        <div className="perf-net-speed">
          <span className="perf-net-arrow perf-net-arrow--down">&#9660;</span>
          <span>{formatBytesPerSec(net.download_speed)}</span>
        </div>
      </div>

      <div className="perf-card__subtitle" style={{ marginTop: 6 }}>
        Total: {formatBytes(net.total_uploaded)} up / {formatBytes(net.total_downloaded)} down
      </div>

      {net.interfaces.length > 0 && (
        <div className="perf-card__breakdown">
          {net.interfaces.map((iface) => (
            <span key={iface.name}>{iface.name}</span>
          ))}
        </div>
      )}
    </>
  );
});

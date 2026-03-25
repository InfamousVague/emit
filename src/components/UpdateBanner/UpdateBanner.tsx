import { Icon } from "@base/primitives/icon/Icon";
import { x, arrowUpCircle, refreshCw, checkCircle } from "../../lib/icons";
import "./UpdateBanner.css";

type UpdatePhase = "idle" | "available" | "downloading" | "ready" | "error";

interface UpdateBannerProps {
  version: string;
  phase: UpdatePhase;
  progress: number;
  onCancel: () => void;
  onRelaunch: () => void;
  onDismiss: () => void;
}

export function UpdateBanner({
  version,
  phase,
  progress,
  onCancel,
  onRelaunch,
  onDismiss,
}: UpdateBannerProps) {
  return (
    <div className={`update-banner ${phase === "ready" ? "update-banner--ready" : ""}`}>
      <div className="update-banner-content">
        {phase === "ready" ? (
          <Icon icon={checkCircle} size="sm" />
        ) : (
          <Icon icon={arrowUpCircle} size="sm" />
        )}
        <span className="update-banner-text">
          {phase === "downloading" && (
            <>Downloading update <strong>v{version}</strong>…</>
          )}
          {phase === "ready" && (
            <>Update installed! Relaunch Emit to complete the update.</>
          )}
          {phase === "error" && (
            <>Update failed — try again later</>
          )}
          {phase === "available" && (
            <>Update available: <strong>v{version}</strong></>
          )}
        </span>
      </div>

      <div className="update-banner-actions">
        {phase === "downloading" && (
          <>
            <span className="update-banner-pct">{Math.round(progress)}%</span>
            <button className="update-banner-dismiss" onClick={onCancel} title="Cancel download">
              <Icon icon={x} size="sm" />
            </button>
          </>
        )}
        {phase === "ready" && (
          <button className="update-banner-btn update-banner-btn--relaunch" onClick={onRelaunch}>
            <Icon icon={refreshCw} size="sm" />
            Relaunch
          </button>
        )}
        {(phase === "available" || phase === "error") && (
          <button className="update-banner-dismiss" onClick={onDismiss}>
            <Icon icon={x} size="sm" />
          </button>
        )}
      </div>

      {phase === "downloading" && (
        <div className="update-banner-progress">
          <div className="update-banner-progress-bar" style={{ width: `${progress}%` }} />
        </div>
      )}
    </div>
  );
}

import { X, ArrowCircleUp } from "@phosphor-icons/react";
import "./UpdateBanner.css";

interface UpdateBannerProps {
  version: string;
  downloading: boolean;
  onUpdate: () => void;
  onDismiss: () => void;
}

export function UpdateBanner({ version, downloading, onUpdate, onDismiss }: UpdateBannerProps) {
  return (
    <div className="update-banner">
      <div className="update-banner-content">
        <ArrowCircleUp size={14} weight="fill" className="update-banner-icon" />
        <span className="update-banner-text">
          Update available: <strong>v{version}</strong>
        </span>
      </div>
      <div className="update-banner-actions">
        <button
          className="update-banner-btn"
          onClick={onUpdate}
          disabled={downloading}
        >
          {downloading ? "Updating…" : "Update"}
        </button>
        <button className="update-banner-dismiss" onClick={onDismiss}>
          <X size={12} weight="bold" />
        </button>
      </div>
    </div>
  );
}

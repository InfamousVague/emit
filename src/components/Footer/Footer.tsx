import { Gear, PuzzlePiece } from "@phosphor-icons/react";
import { Button, Kbd } from "../../ui";
import "./Footer.css";

interface FooterProps {
  onSettingsClick?: () => void;
  onMarketplaceClick?: () => void;
}

export function Footer({ onSettingsClick, onMarketplaceClick }: FooterProps) {
  return (
    <div className="footer">
      <div className="footer-left">
        <span className="footer-label">Emit</span>
        {onSettingsClick && (
          <Button variant="ghost" size="sm" onClick={onSettingsClick}>
            <Gear size={12} weight="regular" /> Settings
          </Button>
        )}
        {onMarketplaceClick && (
          <Button variant="ghost" size="sm" onClick={onMarketplaceClick}>
            <PuzzlePiece size={12} weight="regular" /> Extensions
          </Button>
        )}
      </div>
      <div className="footer-actions">
        <Kbd>{"\u2191\u2193"}</Kbd> <span>Navigate</span>
        <Kbd>{"\u21B5"}</Kbd> <span>Open</span>
      </div>
    </div>
  );
}

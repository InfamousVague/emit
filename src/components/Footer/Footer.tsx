import { Gear, PuzzlePiece, Power } from "@phosphor-icons/react";
import { exit } from "@tauri-apps/plugin-process";
import { Button, Kbd } from "../../ui";
import emitIcon from "../../assets/emit-icon.png";
import "./Footer.css";

interface FooterProps {
  onSettingsClick?: () => void;
  onMarketplaceClick?: () => void;
}

export function Footer({ onSettingsClick, onMarketplaceClick }: FooterProps) {
  return (
    <div className="footer">
      <div className="footer-left">
        <img src={emitIcon} alt="Emit" className="footer-icon" />
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
        <Button variant="ghost" size="sm" onClick={() => exit(0)}>
          <Power size={12} weight="regular" /> Quit
        </Button>
      </div>
      <div className="footer-actions">
        <Kbd>{"\u2191\u2193"}</Kbd> <span>Navigate</span>
        <Kbd>{"\u21B5"}</Kbd> <span>Open</span>
      </div>
    </div>
  );
}

import { exit } from "@tauri-apps/plugin-process";
import { Icon } from "@base/primitives/icon/Icon";
import { settings, puzzle, power } from "../../lib/icons";
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
            <Icon icon={settings} size="sm" /> Settings
          </Button>
        )}
        {onMarketplaceClick && (
          <Button variant="ghost" size="sm" onClick={onMarketplaceClick}>
            <Icon icon={puzzle} size="sm" /> Extensions
          </Button>
        )}
        <Button variant="ghost" size="sm" onClick={() => exit(0)}>
          <Icon icon={power} size="sm" /> Quit
        </Button>
      </div>
      <div className="footer-actions">
        <Kbd>{"\u2191\u2193"}</Kbd> <span>Navigate</span>
        <Kbd>{"\u21B5"}</Kbd> <span>Open</span>
      </div>
    </div>
  );
}

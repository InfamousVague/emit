import { Icon } from "@base/primitives/icon/Icon";
import {
  moon,
  lock,
  rotateCcw,
  power,
  logOut,
  trash,
  notebookPen,
  gauge,
  appWindow,
  settings,
  file,
  search,
  puzzle,
  command,
} from "../lib/icons";
import { COMMAND_EXTENSION_ICONS } from "../assets/extension-icons";

const ID_ICON_MAP: Record<string, string> = {
  "system.sleep": moon,
  "system.lock": lock,
  "system.restart": rotateCcw,
  "system.shutdown": power,
  "system.logout": logOut,
  "system.trash": trash,
  "notion.open": notebookPen,
  "perf-monitor.open": gauge,
};

const CATEGORY_ICON_MAP: Record<string, string> = {
  Applications: appWindow,
  System: settings,
  Files: file,
  Search: search,
  Extensions: puzzle,
  Notion: notebookPen,
};

const FALLBACK_ICON = command;

interface CommandIconProps {
  id: string;
  category: string;
  iconDataUri: string | null;
  size?: number;
}

export function CommandIcon({
  id,
  category,
  iconDataUri,
  size = 20,
}: CommandIconProps) {
  if (iconDataUri) {
    return (
      <div className="cmd-icon">
        <img src={iconDataUri} alt="" width={size} height={size} />
      </div>
    );
  }

  const extensionIcon = COMMAND_EXTENSION_ICONS[id];
  if (extensionIcon) {
    return (
      <div className="cmd-icon">
        <img src={extensionIcon} alt="" width={size} height={size} />
      </div>
    );
  }

  const icon = ID_ICON_MAP[id] ?? CATEGORY_ICON_MAP[category] ?? FALLBACK_ICON;

  return (
    <div className="cmd-icon">
      <Icon icon={icon} size="base" />
    </div>
  );
}

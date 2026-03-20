import {
  Moon,
  LockSimple,
  ArrowCounterClockwise,
  Power,
  SignOut,
  Trash,
  MagnifyingGlass,
  Gear,
  File,
  AppWindow,
  Command,
  PuzzlePiece,
  NotePencil,
  Gauge,
} from "@phosphor-icons/react";
import type { IconProps } from "@phosphor-icons/react";
import { COMMAND_EXTENSION_ICONS } from "../assets/extension-icons";
import "./CommandIcon.css";

type PhosphorIcon = React.ComponentType<IconProps>;

/** Map specific command IDs to Phosphor icons (fallback when no custom icon). */
const ID_ICON_MAP: Record<string, PhosphorIcon> = {
  "system.sleep": Moon,
  "system.lock": LockSimple,
  "system.restart": ArrowCounterClockwise,
  "system.shutdown": Power,
  "system.logout": SignOut,
  "system.trash": Trash,
  "notion.open": NotePencil,
  "perf-monitor.open": Gauge,
};

/** Fallback icons by category. */
const CATEGORY_ICON_MAP: Record<string, PhosphorIcon> = {
  Applications: AppWindow,
  System: Gear,
  Files: File,
  Search: MagnifyingGlass,
  Extensions: PuzzlePiece,
  Notion: NotePencil,
};

const FALLBACK_ICON: PhosphorIcon = Command;

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
  // Real app icon from Rust extraction — render as image
  if (iconDataUri) {
    return (
      <div className="emit-cmd-icon">
        <img src={iconDataUri} alt="" width={size} height={size} />
      </div>
    );
  }

  // Custom extension icon — render as image
  const extensionIcon = COMMAND_EXTENSION_ICONS[id];
  if (extensionIcon) {
    return (
      <div className="emit-cmd-icon">
        <img src={extensionIcon} alt="" width={size} height={size} />
      </div>
    );
  }

  // Phosphor icon by ID, then category, then fallback
  const Icon = ID_ICON_MAP[id] ?? CATEGORY_ICON_MAP[category] ?? FALLBACK_ICON;

  return (
    <div className="emit-cmd-icon">
      <Icon size={size} weight="regular" />
    </div>
  );
}

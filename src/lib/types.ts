export interface CommandEntry {
  id: string;
  name: string;
  description: string;
  category: string;
  icon: string | null;
  match_indices: number[];
  score: number;
}

export interface Settings {
  shortcut: string;
  launch_at_login: boolean;
  show_in_dock: boolean;
  max_results: number;
  check_for_updates: boolean;
  replace_spotlight: boolean;
  ruler_shortcut: string;
  ruler_snap_mode: string;
  ruler_default_unit: string;
  shortcuts: Record<string, string>;
}

export interface ClipboardMetadata {
  width: number;
  height: number;
  size_bytes: number;
  source_app: string | null;
}

export interface ClipboardItem {
  id: string;
  content: string;
  content_type: string;
  timestamp: number;
  preview: string;
  image_path: string | null;
  metadata: ClipboardMetadata | null;
}

export interface ExtensionInfo {
  id: string;
  name: string;
  description: string;
  icon: string;
  category: string;
  bundled: boolean;
  enabled: boolean;
  configured: boolean;
  has_settings: boolean;
}

export interface NotionFilter {
  name: string;
  status: string;
  assignee: string;
}

export interface NotionDatabase {
  id: string;
  title: string;
}

export interface NotionPage {
  id: string;
  title: string;
  status: string;
  assignee: string;
  url: string;
}

// --- Color Picker Types ---

export interface PickedColor {
  hex: string;
  rgb: { r: number; g: number; b: number };
}

export interface ColorPalette {
  id: string;
  name: string;
  colors: PickedColor[];
  created_at: number;
}

// --- Password Generator Types ---

export interface PasswordHistoryEntry {
  id: string;
  password: string;
  generated_at: number;
  label: string | null;
  mode: string;
  length: number;
}

export interface GeneratePasswordOpts {
  length: number;
  uppercase: boolean;
  lowercase: boolean;
  numbers: boolean;
  symbols: boolean;
  passphrase: boolean;
  word_count?: number;
  separator?: string;
  label?: string;
}

// --- Window Management Types ---

export interface WindowBounds {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface WindowInfo {
  window_id: number;
  app_name: string;
  title: string;
  bundle_id: string;
  bounds: WindowBounds;
  is_on_screen: boolean;
  pid: number;
}

export interface ScreenInfo {
  x: number;
  y: number;
  width: number;
  height: number;
  visible_x: number;
  visible_y: number;
  visible_width: number;
  visible_height: number;
  dock_position: "left" | "bottom" | "right" | null;
  is_primary: boolean;
  menu_bar_height: number;
}

export type SnapPosition =
  | "LeftHalf"
  | "RightHalf"
  | "TopHalf"
  | "BottomHalf"
  | "TopLeftQuarter"
  | "TopRightQuarter"
  | "BottomLeftQuarter"
  | "BottomRightQuarter"
  | "LeftThird"
  | "CenterThird"
  | "RightThird"
  | "LeftTwoThirds"
  | "RightTwoThirds"
  | "Maximize"
  | "Center";

// --- Screenshot Types ---

export interface ScreenshotItem {
  id: string;
  path: string;
  thumbnail_path: string;
  timestamp: number;
  width: number;
  height: number;
  source_app: string | null;
}

// --- Command System Types ---

export type CommandCategory = "Read" | "Write";
export type ParamGroup = "Required" | "Advanced";

export interface SelectOption {
  value: string;
  label: string;
  color: string | null;
}

export type ParamType =
  | { type: "Text" }
  | { type: "RichText" }
  | { type: "Number" }
  | { type: "Boolean" }
  | { type: "Date" }
  | { type: "Select"; options: SelectOption[] }
  | { type: "MultiSelect"; options: SelectOption[] }
  | { type: "DatabasePicker" }
  | { type: "PagePicker"; database_id: string | null }
  | { type: "People" }
  | { type: "Url" }
  | { type: "DynamicSelect"; resolver: string };

export interface ParamDefinition {
  id: string;
  name: string;
  param_type: ParamType;
  required: boolean;
  default_value: unknown;
  placeholder: string | null;
  group: ParamGroup;
}

export interface CommandDefinition {
  id: string;
  extension_id: string;
  name: string;
  description: string;
  icon: string | null;
  category: CommandCategory;
  requires_confirmation: boolean;
  shortcut: string | null;
  follow_ups: string[];
  params: ParamDefinition[];
  undoable: boolean;
}

export interface CommandResult {
  success: boolean;
  message: string;
  action_id: string | null;
  data: unknown;
  follow_ups: string[];
  undo_data: unknown;
}

// --- Ruler Types ---

export interface RulerEdgePoint {
  x: number;
  y: number;
  direction: string;
}

// --- Performance Monitor Types ---

export interface MetricSnapshot {
  timestamp: number;
  cpu: CpuMetrics;
  memory: MemoryMetrics;
  disks: DiskMetrics[];
  network: NetworkMetrics;
  gpu: GpuMetrics | null;
  battery: BatteryMetrics | null;
  system: SystemMetrics;
}

export interface CpuMetrics {
  total_usage: number;
  per_core: number[];
  load_avg_1: number;
  load_avg_5: number;
  load_avg_15: number;
}

export interface MemoryMetrics {
  total: number;
  used: number;
  available: number;
  app_memory: number;
  wired: number;
  compressed: number;
}

export interface DiskMetrics {
  name: string;
  mount_point: string;
  total: number;
  used: number;
  available: number;
  fs_type: string;
}

export interface NetworkMetrics {
  upload_speed: number;
  download_speed: number;
  total_uploaded: number;
  total_downloaded: number;
  interfaces: NetworkInterface[];
}

export interface NetworkInterface {
  name: string;
  ip: string;
  is_wifi: boolean;
}

export interface GpuMetrics {
  utilization: number;
  name: string;
}

export interface BatteryMetrics {
  charge_percent: number;
  health_percent: number;
  temperature: number;
  cycle_count: number;
  power_draw: number;
  is_charging: boolean;
  time_remaining: number | null;
}

export interface SystemMetrics {
  uptime_secs: number;
  hostname: string;
}

export interface ProcessInfo {
  pid: number;
  name: string;
  cpu_usage: number;
  memory_bytes: number;
}

export interface AlertThreshold {
  metric: string;
  threshold: number;
  enabled: boolean;
}

export interface AlertConfig {
  thresholds: AlertThreshold[];
  cooldown_secs: number;
}

export type TimeRange = "1m" | "5m" | "15m" | "1hr";

// --- Shortcut Types ---

export interface ShortcutBinding {
  id: string;
  label: string;
  default_keys: string;
  keys: string;
  extension_id: string;
}

import { invoke } from "@tauri-apps/api/core";
import type {
  ClipboardItem,
  CommandDefinition,
  CommandEntry,
  CommandResult,
  ExtensionInfo,
  GeneratePasswordOpts,
  NotionDatabase,
  NotionPage,
  PasswordHistoryEntry,
  SelectOption,
  Settings,
  ScreenInfo,
  SnapPosition,
  WindowInfo,
} from "./types";

export async function search(query: string): Promise<CommandEntry[]> {
  return invoke("search", { query });
}

/** Fast static-only search — no network/IO, returns immediately. */
export async function searchStatic(query: string): Promise<CommandEntry[]> {
  return invoke("search_static", { query });
}

export async function executeCommand(id: string): Promise<string> {
  return invoke("execute_command", { id });
}

export async function hideWindow(): Promise<void> {
  return invoke("hide_window");
}

/** Execute a command and dismiss the launcher window. */
export async function executeAndDismiss(id: string): Promise<void> {
  await executeCommand(id);
  await hideWindow();
}

export async function getSettings(): Promise<Settings> {
  return invoke("get_settings");
}

export async function saveSettings(settings: Settings): Promise<void> {
  return invoke("save_settings", { settings });
}

export async function getClipboardHistory(): Promise<ClipboardItem[]> {
  return invoke("get_clipboard_history");
}

export async function clipboardCopy(id: string): Promise<void> {
  return invoke("clipboard_copy", { id });
}

export async function clipboardDelete(id: string): Promise<void> {
  return invoke("clipboard_delete", { id });
}

export async function clipboardClear(): Promise<void> {
  return invoke("clipboard_clear");
}

export async function clipboardGetImage(id: string): Promise<string> {
  return invoke("clipboard_get_image", { id });
}

// --- Extension commands ---

export async function getExtensions(): Promise<ExtensionInfo[]> {
  return invoke("get_extensions");
}

export async function setExtensionEnabled(
  id: string,
  enabled: boolean,
): Promise<void> {
  return invoke("set_extension_enabled", { id, enabled });
}

export async function getExtensionSettings(
  id: string,
): Promise<Record<string, unknown>> {
  return invoke("get_extension_settings", { id });
}

export async function saveExtensionSettings(
  id: string,
  settings: Record<string, unknown>,
): Promise<void> {
  return invoke("save_extension_settings", { id, settings });
}

// --- Notion commands ---

export async function notionGetDatabases(): Promise<NotionDatabase[]> {
  return invoke("notion_get_databases");
}

export async function notionQueryDatabase(
  databaseId: string,
  filters: Record<string, string>,
): Promise<NotionPage[]> {
  return invoke("notion_query_database", { databaseId, filters });
}

// --- Slash command system ---

export async function searchCommands(
  query: string,
): Promise<CommandDefinition[]> {
  return invoke("search_commands", { query });
}

export async function executeAction(
  commandId: string,
  params: Record<string, unknown>,
): Promise<CommandResult> {
  return invoke("execute_action", { commandId, params });
}

export async function resolveParamOptions(
  commandId: string,
  paramId: string,
  query: string,
): Promise<SelectOption[]> {
  return invoke("resolve_param_options", { commandId, paramId, query });
}

export async function undoLastAction(): Promise<CommandResult> {
  return invoke("undo_last_action");
}

// --- Notion CRUD ---

export async function notionGetDatabaseSchema(
  databaseId: string,
): Promise<Record<string, unknown>> {
  return invoke("notion_get_database_schema", { databaseId });
}

export async function notionCreatePage(
  databaseId: string,
  properties: Record<string, unknown>,
): Promise<Record<string, unknown>> {
  return invoke("notion_create_page", { databaseId, properties });
}

export async function notionUpdatePage(
  pageId: string,
  properties: Record<string, unknown>,
): Promise<Record<string, unknown>> {
  return invoke("notion_update_page", { pageId, properties });
}

export async function notionArchivePage(
  pageId: string,
  archive: boolean,
): Promise<void> {
  return invoke("notion_archive_page", { pageId, archive });
}

export async function notionAddComment(
  pageId: string,
  content: string,
): Promise<void> {
  return invoke("notion_add_comment", { pageId, content });
}

export async function notionSearchPages(
  query: string,
  databaseId?: string,
): Promise<Array<{ id: string; title: string; url: string }>> {
  return invoke("notion_search_pages", { query, databaseId });
}

// --- Color Picker ---

export async function colorPickerSampleScreen(): Promise<void> {
  return invoke("color_picker_sample_screen");
}

export async function colorPickerSavePalettes(
  palettes: import("./types").ColorPalette[],
): Promise<void> {
  return invoke("color_picker_save_palettes", { palettes });
}

export async function colorPickerLoadPalettes(): Promise<
  import("./types").ColorPalette[]
> {
  return invoke("color_picker_load_palettes");
}

// --- Password Generator ---

export async function pwgenHasVault(): Promise<boolean> {
  return invoke("pwgen_has_vault");
}

export async function pwgenSetup(password: string): Promise<void> {
  return invoke("pwgen_setup", { password });
}

export async function pwgenUnlock(password: string): Promise<void> {
  return invoke("pwgen_unlock", { password });
}

export async function pwgenLock(): Promise<void> {
  return invoke("pwgen_lock");
}

export async function pwgenIsUnlocked(): Promise<boolean> {
  return invoke("pwgen_is_unlocked");
}

export async function pwgenGenerate(
  opts: GeneratePasswordOpts,
): Promise<string> {
  return invoke("pwgen_generate", { opts });
}

export async function pwgenSaveToHistory(
  password: string,
  mode: string,
  length: number,
  label?: string,
): Promise<PasswordHistoryEntry> {
  return invoke("pwgen_save_to_history", { password, mode, length, label });
}

export async function pwgenGetHistory(): Promise<PasswordHistoryEntry[]> {
  return invoke("pwgen_get_history");
}

export async function pwgenDeleteHistoryEntry(id: string): Promise<void> {
  return invoke("pwgen_delete_history_entry", { id });
}

export async function pwgenClearHistory(): Promise<void> {
  return invoke("pwgen_clear_history");
}

export async function pwgenCopyPassword(id: string): Promise<string> {
  return invoke("pwgen_copy_password", { id });
}

export async function pwgenSetLockTimeout(secs: number): Promise<void> {
  return invoke("pwgen_set_lock_timeout", { seconds: secs });
}

export async function pwgenGetLockTimeout(): Promise<number> {
  return invoke("pwgen_get_lock_timeout");
}

// --- Window Management ---

export async function wmCheckAccessibility(): Promise<boolean> {
  return invoke("wm_check_accessibility");
}

export async function wmRequestAccessibility(): Promise<void> {
  return invoke("wm_request_accessibility");
}

export async function wmListWindows(): Promise<WindowInfo[]> {
  return invoke("wm_list_windows");
}

export async function wmSnapWindow(
  windowId: number,
  position: SnapPosition,
): Promise<void> {
  return invoke("wm_snap_window", { windowId, position });
}

export async function wmSnapFocused(position: SnapPosition): Promise<void> {
  return invoke("wm_snap_focused", { position });
}

export async function wmGetAppIcon(
  appName: string,
): Promise<string | null> {
  return invoke("wm_get_app_icon", { appName });
}

export async function wmGetScreenInfo(): Promise<ScreenInfo> {
  return invoke("wm_get_screen_info");
}

// --- Screenshot ---

export async function screenshotCaptureRegion(): Promise<void> {
  return invoke("screenshot_capture_region");
}

export async function screenshotCaptureWindow(): Promise<void> {
  return invoke("screenshot_capture_window");
}

export async function screenshotCaptureScreen(): Promise<void> {
  return invoke("screenshot_capture_screen");
}

export async function screenshotList(): Promise<
  import("./types").ScreenshotItem[]
> {
  return invoke("screenshot_list");
}

export async function screenshotDelete(id: string): Promise<void> {
  return invoke("screenshot_delete", { id });
}

export async function screenshotCopy(id: string): Promise<void> {
  return invoke("screenshot_copy", { id });
}

export async function screenshotGetImage(path: string): Promise<string> {
  return invoke("screenshot_get_image", { path });
}

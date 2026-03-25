import clipboardIcon from "./clipboard.png";
import colorPickerIcon from "./color-picker.png";
import marketplaceIcon from "./marketplace.png";
import passwordGeneratorIcon from "./password-generator.png";
import performanceMonitorIcon from "./performance-monitor.png";
import windowManagementIcon from "./window-management.png";
import rulerIcon from "./ruler.jpg";
import portPilotIcon from "./port-pilot.jpg";
import screenshotIcon from "./screenshot.jpg";

/** Maps extension IDs to their custom icon asset URLs. */
export const EXTENSION_ICONS: Record<string, string> = {
  "clipboard": clipboardIcon,
  "color-picker": colorPickerIcon,
  "marketplace": marketplaceIcon,
  "password-generator": passwordGeneratorIcon,
  "perf-monitor": performanceMonitorIcon,
  "port-pilot": portPilotIcon,
  "screenshot": screenshotIcon,
  "ruler": rulerIcon,
  "window-management": windowManagementIcon,
};

/** Maps command IDs to their extension's custom icon. */
export const COMMAND_EXTENSION_ICONS: Record<string, string> = {
  "clipboard.open": clipboardIcon,
  "color_picker.open": colorPickerIcon,
  "system.marketplace": marketplaceIcon,
  "pwgen.open": passwordGeneratorIcon,
  "perf.open": performanceMonitorIcon,
  "perf.cpu": performanceMonitorIcon,
  "perf.memory": performanceMonitorIcon,
  "perf.disk": performanceMonitorIcon,
  "perf.network": performanceMonitorIcon,
  "perf.gpu": performanceMonitorIcon,
  "perf.battery": performanceMonitorIcon,
  "perf.uptime": performanceMonitorIcon,
  "perf.dashboard": performanceMonitorIcon,
  "port-pilot.dashboard": portPilotIcon,
  "screenshot.open": screenshotIcon,
  "ruler.open": rulerIcon,
  "wm.open": windowManagementIcon,
  "wm.snap.left_half": windowManagementIcon,
  "wm.snap.right_half": windowManagementIcon,
  "wm.snap.top_half": windowManagementIcon,
  "wm.snap.bottom_half": windowManagementIcon,
  "wm.snap.top_left": windowManagementIcon,
  "wm.snap.top_right": windowManagementIcon,
  "wm.snap.bottom_left": windowManagementIcon,
  "wm.snap.bottom_right": windowManagementIcon,
  "wm.snap.left_third": windowManagementIcon,
  "wm.snap.center_third": windowManagementIcon,
  "wm.snap.right_third": windowManagementIcon,
  "wm.snap.left_two_thirds": windowManagementIcon,
  "wm.snap.right_two_thirds": windowManagementIcon,
  "wm.snap.maximize": windowManagementIcon,
  "wm.snap.center": windowManagementIcon,
};

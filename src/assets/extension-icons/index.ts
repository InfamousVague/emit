import clipboardIcon from "./clipboard.png";
import colorPickerIcon from "./color-picker.png";
import marketplaceIcon from "./marketplace.png";
import passwordGeneratorIcon from "./password-generator.png";
import windowManagementIcon from "./window-management.png";

/** Maps extension IDs to their custom icon asset URLs. */
export const EXTENSION_ICONS: Record<string, string> = {
  "clipboard": clipboardIcon,
  "color-picker": colorPickerIcon,
  "marketplace": marketplaceIcon,
  "password-generator": passwordGeneratorIcon,
  "window-management": windowManagementIcon,
};

/** Maps command IDs to their extension's custom icon. */
export const COMMAND_EXTENSION_ICONS: Record<string, string> = {
  "clipboard.open": clipboardIcon,
  "color_picker.open": colorPickerIcon,
  "system.marketplace": marketplaceIcon,
  "pwgen.open": passwordGeneratorIcon,
};

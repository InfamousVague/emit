const HEX6 = /^#([0-9a-f]{6})$/i;
const HEX3 = /^#([0-9a-f]{3})$/i;
const HEX8 = /^#([0-9a-f]{8})$/i;
const RGB = /^rgba?\(\s*(\d{1,3})\s*,\s*(\d{1,3})\s*,\s*(\d{1,3})\s*(?:,\s*[\d.]+\s*)?\)$/i;
const HSL = /^hsla?\(\s*(\d{1,3})\s*,\s*(\d{1,3})%\s*,\s*(\d{1,3})%\s*(?:,\s*[\d.]+\s*)?\)$/i;

/**
 * Detect if a clipboard string is a color value.
 * Returns a CSS-compatible color string for rendering, or null.
 */
export function detectColor(text: string): string | null {
  const trimmed = text.trim();

  if (HEX6.test(trimmed) || HEX3.test(trimmed) || HEX8.test(trimmed)) {
    return trimmed;
  }

  if (RGB.test(trimmed)) {
    return trimmed;
  }

  if (HSL.test(trimmed)) {
    return trimmed;
  }

  return null;
}

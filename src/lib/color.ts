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

/** Convert RGB (0-255 per channel) to HSL (h: 0-360, s/l: 0-100). */
export function rgbToHsl(
  r: number,
  g: number,
  b: number,
): { h: number; s: number; l: number } {
  r /= 255;
  g /= 255;
  b /= 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const l = (max + min) / 2;
  if (max === min) return { h: 0, s: 0, l: Math.round(l * 100) };
  const d = max - min;
  const s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
  let h = 0;
  if (max === r) h = ((g - b) / d + (g < b ? 6 : 0)) / 6;
  else if (max === g) h = ((b - r) / d + 2) / 6;
  else h = ((r - g) / d + 4) / 6;
  return {
    h: Math.round(h * 360),
    s: Math.round(s * 100),
    l: Math.round(l * 100),
  };
}

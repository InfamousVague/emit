import type { Point, Unit } from "./types";

/**
 * Euclidean distance between two points.
 * @param a - First point.
 * @param b - Second point.
 * @returns Distance in pixels.
 */
export function distance(a: Point, b: Point): number {
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  return Math.sqrt(dx * dx + dy * dy);
}

/**
 * Snap the end point to the nearest 45-degree increment relative to start.
 * Used when the user holds Shift to constrain measurement lines to
 * horizontal, vertical, or diagonal axes.
 * @param start - The fixed origin point.
 * @param end   - The unconstrained cursor position.
 * @returns A new point at the same distance from `start` but snapped to the
 *          nearest 45-degree angle.
 */
export function constrainAngle(start: Point, end: Point): Point {
  const dx = end.x - start.x;
  const dy = end.y - start.y;
  const angle = Math.atan2(dy, dx);
  const snapped = Math.round(angle / (Math.PI / 4)) * (Math.PI / 4);
  const dist = Math.sqrt(dx * dx + dy * dy);
  return {
    x: start.x + Math.cos(snapped) * dist,
    y: start.y + Math.sin(snapped) * dist,
  };
}

/**
 * Convert a pixel measurement to the target display unit.
 * @param px   - Value in CSS pixels.
 * @param unit - Target unit (`"px"`, `"pt"`, `"inches"`, or `"rem"`).
 * @returns The converted numeric value.
 */
export function convertUnit(px: number, unit: Unit): number {
  switch (unit) {
    case "px":
      return px;
    case "pt":
      return px * 0.75;
    case "inches":
      return px / 96;
    case "rem":
      return px / 16;
  }
}

/**
 * Format a human-readable measurement label showing both endpoint coordinates
 * and the converted distance, e.g. `"(10,20) -> (110,20) = 100px"`.
 * @param start - Start point of the measurement.
 * @param end   - End point of the measurement.
 * @param unit  - Display unit for the distance value.
 * @returns Formatted string for rendering inside a badge.
 */
export function formatLabel(start: Point, end: Point, unit: Unit): string {
  const dist = distance(start, end);
  const value = convertUnit(dist, unit);
  const decimals = unit === "px" ? 0 : 1;
  return `(${Math.round(start.x)},${Math.round(start.y)}) → (${Math.round(end.x)},${Math.round(end.y)}) = ${value.toFixed(decimals)}${unit}`;
}

/**
 * Angle in radians from start to end (measured from the positive x-axis).
 * @param start - Origin point.
 * @param end   - Target point.
 * @returns Angle in radians, range (-PI, PI].
 */
export function angle(start: Point, end: Point): number {
  return Math.atan2(end.y - start.y, end.x - start.x);
}

/**
 * Midpoint between two points.
 * @param a - First point.
 * @param b - Second point.
 * @returns The point equidistant from `a` and `b`.
 */
export function midpoint(a: Point, b: Point): Point {
  return { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 };
}

import type { Point, Unit } from "./types";

/** Euclidean distance between two points. */
export function distance(a: Point, b: Point): number {
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  return Math.sqrt(dx * dx + dy * dy);
}

/** Snap the end point to the nearest 45° angle from start. */
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

/** Convert pixels to the target unit. */
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

/** Format a measurement label with full coordinates. */
export function formatLabel(start: Point, end: Point, unit: Unit): string {
  const dist = distance(start, end);
  const value = convertUnit(dist, unit);
  const decimals = unit === "px" ? 0 : 1;
  return `(${Math.round(start.x)},${Math.round(start.y)}) → (${Math.round(end.x)},${Math.round(end.y)}) = ${value.toFixed(decimals)}${unit}`;
}

/** Angle in radians from start to end. */
export function angle(start: Point, end: Point): number {
  return Math.atan2(end.y - start.y, end.x - start.x);
}

/** Midpoint between two points. */
export function midpoint(a: Point, b: Point): Point {
  return { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 };
}

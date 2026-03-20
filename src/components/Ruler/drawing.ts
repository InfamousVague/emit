import type { Measurement, Point, Unit, ClickTarget } from "./types";
import { formatLabel, angle, midpoint } from "./geometry";

const ACCENT = "#FF6B35";
const BADGE_BG = "rgba(0, 0, 0, 0.75)";
const BADGE_TEXT = "#ffffff";
const DISMISS_SIZE = 14;
const ENDPOINT_RADIUS = 4;

/** Draw a measurement line with label and dismiss button. Returns the dismiss click target. */
export function drawMeasurement(
  ctx: CanvasRenderingContext2D,
  m: Measurement,
  unit: Unit,
  onDismiss: () => void,
): ClickTarget {
  drawLine(ctx, m.start, m.end);
  drawEndpoint(ctx, m.start);
  drawEndpoint(ctx, m.end);
  const label = formatLabel(m.start, m.end, unit);
  const mid = midpoint(m.start, m.end);
  const lineAngle = angle(m.start, m.end);
  const badgeBounds = drawBadge(ctx, label, mid, lineAngle);
  return drawDismissButton(ctx, badgeBounds, onDismiss);
}

/** Draw the currently-active draw in progress. */
export function drawActiveDraw(
  ctx: CanvasRenderingContext2D,
  start: Point,
  current: Point,
  unit: Unit,
): void {
  drawLine(ctx, start, current);
  drawEndpoint(ctx, start);
  drawEndpoint(ctx, current);

  const label = formatLabel(start, current, unit);
  const mid = midpoint(start, current);
  const lineAngle = angle(start, current);
  drawBadge(ctx, label, mid, lineAngle);
}

/** Draw a simple 1px accent line — pixel-aligned, no outline or glow. */
function drawLine(ctx: CanvasRenderingContext2D, a: Point, b: Point): void {
  ctx.strokeStyle = ACCENT;
  ctx.lineWidth = 1;
  ctx.lineCap = "butt";
  ctx.beginPath();
  ctx.moveTo(Math.round(a.x) + 0.5, Math.round(a.y) + 0.5);
  ctx.lineTo(Math.round(b.x) + 0.5, Math.round(b.y) + 0.5);
  ctx.stroke();
}

/** Draw a hollow circle at a measurement endpoint. */
function drawEndpoint(ctx: CanvasRenderingContext2D, point: Point): void {
  ctx.strokeStyle = ACCENT;
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.arc(point.x, point.y, ENDPOINT_RADIUS, 0, Math.PI * 2);
  ctx.stroke();
}

/**
 * Determine if the line is more vertical than horizontal.
 * Within 45 degrees of straight up/down → vertical text.
 */
function isVerticalLine(lineAngle: number): boolean {
  const abs = Math.abs(lineAngle);
  return abs > Math.PI / 4 && abs < (3 * Math.PI) / 4;
}

/** Draw a label badge, rotated to match the line direction. Returns bounds in screen space. */
function drawBadge(
  ctx: CanvasRenderingContext2D,
  text: string,
  position: Point,
  lineAngle: number,
): { x: number; y: number; width: number; height: number } {
  ctx.font = "12px -apple-system, BlinkMacSystemFont, sans-serif";
  const metrics = ctx.measureText(text);
  const padX = 8;
  const padY = 5;
  const w = metrics.width + padX * 2;
  const h = 20 + padY * 2;

  const vertical = isVerticalLine(lineAngle);

  ctx.save();
  ctx.translate(position.x, position.y);

  if (vertical) {
    // Rotate -90° so text reads bottom-to-top along vertical lines
    ctx.rotate(-Math.PI / 2);
    const offsetX = -w / 2;
    const offsetY = -h - 8;

    ctx.fillStyle = BADGE_BG;
    roundRect(ctx, offsetX, offsetY, w, h, 6);
    ctx.fill();

    ctx.fillStyle = BADGE_TEXT;
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    ctx.fillText(text, 0, offsetY + h / 2);

    ctx.restore();

    // Screen-space bounds for hit-testing (width/height swapped due to rotation)
    return {
      x: position.x - h / 2 - 8,
      y: position.y - w / 2,
      width: h,
      height: w,
    };
  } else {
    // Horizontal — render above the line
    const offsetX = -w / 2;
    const offsetY = -h - 8;

    ctx.fillStyle = BADGE_BG;
    roundRect(ctx, offsetX, offsetY, w, h, 6);
    ctx.fill();

    ctx.fillStyle = BADGE_TEXT;
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    ctx.fillText(text, 0, offsetY + h / 2);

    ctx.restore();

    return {
      x: position.x + offsetX,
      y: position.y + offsetY,
      width: w,
      height: h,
    };
  }
}

/** Draw a small X dismiss button adjacent to a badge. Returns click target. */
function drawDismissButton(
  ctx: CanvasRenderingContext2D,
  badge: { x: number; y: number; width: number; height: number },
  onDismiss: () => void,
): ClickTarget {
  const cx = badge.x + badge.width + DISMISS_SIZE / 2 + 4;
  const cy = badge.y + badge.height / 2;
  const r = DISMISS_SIZE / 2;

  ctx.fillStyle = "rgba(255, 60, 60, 0.8)";
  ctx.beginPath();
  ctx.arc(cx, cy, r, 0, Math.PI * 2);
  ctx.fill();

  ctx.strokeStyle = "#fff";
  ctx.lineWidth = 1.5;
  const offset = 3.5;
  ctx.beginPath();
  ctx.moveTo(cx - offset, cy - offset);
  ctx.lineTo(cx + offset, cy + offset);
  ctx.moveTo(cx + offset, cy - offset);
  ctx.lineTo(cx - offset, cy + offset);
  ctx.stroke();

  return {
    x: cx - r,
    y: cy - r,
    width: DISMISS_SIZE,
    height: DISMISS_SIZE,
    action: onDismiss,
  };
}

/** Helper to draw a rounded rectangle path. */
function roundRect(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  h: number,
  r: number,
): void {
  ctx.beginPath();
  ctx.moveTo(x + r, y);
  ctx.lineTo(x + w - r, y);
  ctx.quadraticCurveTo(x + w, y, x + w, y + r);
  ctx.lineTo(x + w, y + h - r);
  ctx.quadraticCurveTo(x + w, y + h, x + w - r, y + h);
  ctx.lineTo(x + r, y + h);
  ctx.quadraticCurveTo(x, y + h, x, y + h - r);
  ctx.lineTo(x, y + r);
  ctx.quadraticCurveTo(x, y, x + r, y);
  ctx.closePath();
}

/** Draw a precise pixel-aligned crosshair with a clear center gap so the target pixel is visible. */
export function drawCrosshair(
  ctx: CanvasRenderingContext2D,
  pos: Point,
): void {
  const x = Math.round(pos.x) + 0.5;
  const y = Math.round(pos.y) + 0.5;
  const gap = 6;
  const armLen = 24;

  // Dark outline for contrast on bright backgrounds
  ctx.strokeStyle = "rgba(0, 0, 0, 0.4)";
  ctx.lineWidth = 3;
  ctx.beginPath();
  ctx.moveTo(x - armLen, y);
  ctx.lineTo(x - gap, y);
  ctx.moveTo(x + gap, y);
  ctx.lineTo(x + armLen, y);
  ctx.moveTo(x, y - armLen);
  ctx.lineTo(x, y - gap);
  ctx.moveTo(x, y + gap);
  ctx.lineTo(x, y + armLen);
  ctx.stroke();

  // Bright crisp arms
  ctx.strokeStyle = "rgba(255, 255, 255, 0.9)";
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.moveTo(x - armLen, y);
  ctx.lineTo(x - gap, y);
  ctx.moveTo(x + gap, y);
  ctx.lineTo(x + armLen, y);
  ctx.moveTo(x, y - armLen);
  ctx.lineTo(x, y - gap);
  ctx.moveTo(x, y + gap);
  ctx.lineTo(x, y + armLen);
  ctx.stroke();
}

/** Check if a point hits any click target. */
export function hitTest(
  targets: ClickTarget[],
  point: Point,
): ClickTarget | null {
  for (const t of targets) {
    if (
      point.x >= t.x &&
      point.x <= t.x + t.width &&
      point.y >= t.y &&
      point.y <= t.y + t.height
    ) {
      return t;
    }
  }
  return null;
}

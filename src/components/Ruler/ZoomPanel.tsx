import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Point } from "./types";

interface ZoomPanelProps {
  cursorPos: Point;
  zoomLevel: number;
}

export function ZoomPanel({ cursorPos, zoomLevel }: ZoomPanelProps) {
  const [imageSrc, setImageSrc] = useState<string | null>(null);
  const [hexColor, setHexColor] = useState("#000000");
  const lastCapture = useRef(0);
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const now = Date.now();
    if (now - lastCapture.current < 66) return; // ~15fps
    lastCapture.current = now;

    const captureRadius = Math.round(100 / zoomLevel);
    const x = cursorPos.x - captureRadius;
    const y = cursorPos.y - captureRadius;
    const size = captureRadius * 2;

    invoke<string>("ruler_capture_region", {
      x,
      y,
      width: size,
      height: size,
    })
      .then((dataUrl) => {
        setImageSrc(dataUrl);
        extractCenterColor(dataUrl);
      })
      .catch(() => {});
  }, [cursorPos.x, cursorPos.y, zoomLevel]);

  function extractCenterColor(dataUrl: string) {
    const img = new Image();
    img.onload = () => {
      const c = canvasRef.current;
      if (!c) return;
      const ctx = c.getContext("2d");
      if (!ctx) return;
      c.width = img.width;
      c.height = img.height;
      ctx.drawImage(img, 0, 0);
      const cx = Math.floor(img.width / 2);
      const cy = Math.floor(img.height / 2);
      const pixel = ctx.getImageData(cx, cy, 1, 1).data;
      const hex = `#${pixel[0].toString(16).padStart(2, "0")}${pixel[1].toString(16).padStart(2, "0")}${pixel[2].toString(16).padStart(2, "0")}`;
      setHexColor(hex);
    };
    img.src = dataUrl;
  }

  return (
    <div className="ruler-zoom-panel">
      <div className="ruler-zoom-viewport">
        {imageSrc && (
          <img
            src={imageSrc}
            alt="Zoom"
            className="ruler-zoom-image"
            draggable={false}
          />
        )}
        <div className="ruler-zoom-crosshair-h" />
        <div className="ruler-zoom-crosshair-v" />
      </div>
      <div className="ruler-zoom-info">
        <span>
          ({Math.round(cursorPos.x)}, {Math.round(cursorPos.y)})
        </span>
        <span style={{ display: "flex", alignItems: "center", gap: 4 }}>
          <span
            className="ruler-zoom-swatch"
            style={{ background: hexColor }}
          />
          {hexColor}
        </span>
        <span className="ruler-zoom-level">{zoomLevel}x</span>
      </div>
      <canvas ref={canvasRef} style={{ display: "none" }} />
    </div>
  );
}

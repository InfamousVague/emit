import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Measurement, Point, Unit, ClickTarget } from "./types";
import { constrainAngle } from "./geometry";
import {
  drawMeasurement,
  drawActiveDraw,
  drawCrosshair,
  hitTest,
} from "./drawing";
import { ZoomPanel } from "./ZoomPanel";
import { ControlPanel } from "./ControlPanel";
import "./RulerApp.css";

export function RulerApp() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [measurements, setMeasurements] = useState<Measurement[]>([]);
  const [activeDraw, setActiveDraw] = useState<{
    start: Point;
    current: Point;
  } | null>(null);
  const [cursorPos, setCursorPos] = useState<Point>({ x: 0, y: 0 });
  const [unit, setUnit] = useState<Unit>("px");
  const [zoomLevel, setZoomLevel] = useState(4);
  const shiftRef = useRef(false);
  const clickTargetsRef = useRef<ClickTarget[]>([]);
  const lastEscRef = useRef(0);
  const dirtyRef = useRef(true);
  const activeDrawRef = useRef(activeDraw);
  const measurementsRef = useRef(measurements);

  // Keep refs in sync
  activeDrawRef.current = activeDraw;
  measurementsRef.current = measurements;

  // Resize canvas to window
  useEffect(() => {
    function resize() {
      const canvas = canvasRef.current;
      if (!canvas) return;
      canvas.width = window.innerWidth * window.devicePixelRatio;
      canvas.height = window.innerHeight * window.devicePixelRatio;
      canvas.style.width = `${window.innerWidth}px`;
      canvas.style.height = `${window.innerHeight}px`;
      dirtyRef.current = true;
    }
    resize();
    window.addEventListener("resize", resize);
    return () => window.removeEventListener("resize", resize);
  }, []);

  // Render loop
  useEffect(() => {
    let raf: number;

    function render() {
      raf = requestAnimationFrame(render);
      if (!dirtyRef.current) return;
      dirtyRef.current = false;

      const canvas = canvasRef.current;
      if (!canvas) return;
      const ctx = canvas.getContext("2d");
      if (!ctx) return;

      const dpr = window.devicePixelRatio;
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      ctx.clearRect(0, 0, canvas.width / dpr, canvas.height / dpr);

      const targets: ClickTarget[] = [];

      // Draw completed measurements
      for (const m of measurementsRef.current) {
        const target = drawMeasurement(ctx, m, unit, () => {
          setMeasurements((prev) => prev.filter((item) => item.id !== m.id));
          dirtyRef.current = true;
        });
        targets.push(target);
      }

      // Draw active measurement
      if (activeDrawRef.current) {
        drawActiveDraw(
          ctx,
          activeDrawRef.current.start,
          activeDrawRef.current.current,
          unit,
        );
      }

      // Draw crosshair at cursor
      drawCrosshair(ctx, cursorPos);

      clickTargetsRef.current = targets;
    }

    raf = requestAnimationFrame(render);
    return () => cancelAnimationFrame(raf);
  }, [measurements, activeDraw, cursorPos, unit]);

  // Mouse handlers — click to start, click to end
  const handleClick = useCallback(
    (e: React.MouseEvent) => {
      const pos = { x: e.clientX, y: e.clientY };

      // Check if clicking a dismiss button
      const hit = hitTest(clickTargetsRef.current, pos);
      if (hit) {
        hit.action();
        return;
      }

      if (!activeDrawRef.current) {
        // First click — start drawing
        setActiveDraw({ start: pos, current: pos });
        dirtyRef.current = true;
      } else {
        // Second click — finalize measurement
        const { start } = activeDrawRef.current;
        const endPoint = shiftRef.current
          ? constrainAngle(start, pos)
          : pos;
        const dx = endPoint.x - start.x;
        const dy = endPoint.y - start.y;

        if (Math.sqrt(dx * dx + dy * dy) > 3) {
          const id = `m_${Date.now()}_${Math.random().toString(36).slice(2, 6)}`;
          setMeasurements((prev) => [...prev, { id, start, end: endPoint }]);
        }

        setActiveDraw(null);
        dirtyRef.current = true;
      }
    },
    [],
  );

  const handleMouseMove = useCallback(
    (e: React.MouseEvent) => {
      const pos = { x: e.clientX, y: e.clientY };
      setCursorPos(pos);
      dirtyRef.current = true;

      if (activeDrawRef.current) {
        const endPoint = shiftRef.current
          ? constrainAngle(activeDrawRef.current.start, pos)
          : pos;
        setActiveDraw({
          start: activeDrawRef.current.start,
          current: endPoint,
        });
      }
    },
    [],
  );

  // Keyboard handlers
  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === "Shift") {
        shiftRef.current = true;
      }
      if (e.key === "Escape") {
        if (activeDrawRef.current) {
          // Cancel active draw
          setActiveDraw(null);
          dirtyRef.current = true;
        } else {
          const now = Date.now();
          if (now - lastEscRef.current < 500) {
            // Double escape — close overlay
            invoke("ruler_close");
          }
          lastEscRef.current = now;
        }
      }
    }
    function onKeyUp(e: KeyboardEvent) {
      if (e.key === "Shift") {
        shiftRef.current = false;
      }
    }
    document.addEventListener("keydown", onKeyDown);
    document.addEventListener("keyup", onKeyUp);
    return () => {
      document.removeEventListener("keydown", onKeyDown);
      document.removeEventListener("keyup", onKeyUp);
    };
  }, []);

  // Scroll wheel for zoom
  useEffect(() => {
    function onWheel(e: WheelEvent) {
      e.preventDefault();
      setZoomLevel((prev) => {
        if (e.deltaY < 0) return Math.min(16, prev * 2);
        return Math.max(2, prev / 2);
      });
    }
    window.addEventListener("wheel", onWheel, { passive: false });
    return () => window.removeEventListener("wheel", onWheel);
  }, []);

  return (
    <div className="ruler-overlay">
      <canvas
        ref={canvasRef}
        className="ruler-canvas"
        onClick={handleClick}
        onMouseMove={handleMouseMove}
      />
      <ZoomPanel cursorPos={cursorPos} zoomLevel={zoomLevel} />
      <ControlPanel
        measurements={measurements}
        unit={unit}
        onUnitChange={setUnit}
        onClearAll={() => {
          setMeasurements([]);
          dirtyRef.current = true;
        }}
      />
    </div>
  );
}

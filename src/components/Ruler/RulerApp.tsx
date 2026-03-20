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

const SNAP_DISTANCE = 8;

/** Snap a point to the nearest detected edge within SNAP_DISTANCE pixels. */
function snapToEdge(
  pos: Point,
  edges: Array<{ x: number; y: number; direction: string }>,
): Point {
  if (edges.length === 0) return pos;

  let closest: { dist: number; snapped: Point } | null = null;

  for (const edge of edges) {
    const isHoriz = edge.direction === "left" || edge.direction === "right";
    // Snap along the axis the edge was detected on
    const dist = isHoriz
      ? Math.abs(pos.x - edge.x)
      : Math.abs(pos.y - edge.y);

    if (dist < SNAP_DISTANCE && (!closest || dist < closest.dist)) {
      closest = {
        dist,
        snapped: isHoriz
          ? { x: edge.x, y: pos.y }
          : { x: pos.x, y: edge.y },
      };
    }
  }

  return closest ? closest.snapped : pos;
}

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
  const [edges, setEdges] = useState<Array<{ x: number; y: number; direction: string }>>([]);
  const shiftRef = useRef(false);
  const edgesRef = useRef(edges);
  const clickTargetsRef = useRef<ClickTarget[]>([]);
  const lastEscRef = useRef(0);
  const dirtyRef = useRef(true);
  const activeDrawRef = useRef(activeDraw);
  const measurementsRef = useRef(measurements);

  // Keep refs in sync
  activeDrawRef.current = activeDraw;
  measurementsRef.current = measurements;
  edgesRef.current = edges;

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
  }, [measurements, activeDraw, cursorPos, unit, edges]);

  // Mouse handlers
  const handleMouseDown = useCallback(
    (e: React.MouseEvent) => {
      const raw = { x: e.clientX, y: e.clientY };
      const pos = snapToEdge(raw, edgesRef.current);

      // Check if clicking a dismiss button
      const hit = hitTest(clickTargetsRef.current, raw);
      if (hit) {
        hit.action();
        return;
      }

      setActiveDraw({ start: pos, current: pos });
      dirtyRef.current = true;
    },
    [],
  );

  const handleMouseMove = useCallback(
    (e: React.MouseEvent) => {
      const raw = { x: e.clientX, y: e.clientY };
      const snapped = snapToEdge(raw, edgesRef.current);
      setCursorPos(snapped);
      dirtyRef.current = true;

      if (activeDrawRef.current) {
        const endPoint = shiftRef.current
          ? constrainAngle(activeDrawRef.current.start, snapped)
          : snapped;
        setActiveDraw({
          start: activeDrawRef.current.start,
          current: endPoint,
        });
      }
    },
    [],
  );

  const handleMouseUp = useCallback(() => {
    if (!activeDrawRef.current) return;

    const { start, current } = activeDrawRef.current;
    const dx = current.x - start.x;
    const dy = current.y - start.y;

    // Only create measurement if dragged more than 3px
    if (Math.sqrt(dx * dx + dy * dy) > 3) {
      const id = `m_${Date.now()}_${Math.random().toString(36).slice(2, 6)}`;
      setMeasurements((prev) => [...prev, { id, start, end: current }]);
    }

    setActiveDraw(null);
    dirtyRef.current = true;
  }, []);

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

  // Edge detection — fetch detected edges near cursor (throttled)
  const edgeFetchRef = useRef(0);
  useEffect(() => {
    const now = Date.now();
    if (now - edgeFetchRef.current < 100) return; // ~10fps
    edgeFetchRef.current = now;

    invoke<Array<{ x: number; y: number; direction: string }>>(
      "ruler_detect_edges",
      { x: cursorPos.x, y: cursorPos.y, radius: 50 },
    )
      .then(setEdges)
      .catch(() => setEdges([]));
  }, [cursorPos.x, cursorPos.y]);

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
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
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

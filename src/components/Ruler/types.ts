export interface Point {
  x: number;
  y: number;
}

export interface Measurement {
  id: string;
  start: Point;
  end: Point;
}

export type Unit = "px" | "pt" | "inches" | "rem";

export interface EdgePoint {
  x: number;
  y: number;
  direction: string;
}

export interface ClickTarget {
  x: number;
  y: number;
  width: number;
  height: number;
  action: () => void;
}

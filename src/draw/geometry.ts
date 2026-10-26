import type { Element, Point, Shape } from "./model";

const dist = (a: { x: number; y: number }, b: { x: number; y: number }) => Math.hypot(a.x - b.x, a.y - b.y);

/** Distance from p to the segment ab */
const segDist = (p: Point, a: Point, b: Point) => {
  const dx = b.x - a.x, dy = b.y - a.y;
  const len = dx * dx + dy * dy;
  const t = len ? Math.max(0, Math.min(1, ((p.x - a.x) * dx + (p.y - a.y) * dy) / len)) : 0;
  return Math.hypot(p.x - (a.x + t * dx), p.y - (a.y + t * dy));
};

/** Constrains a shape drag with Shift: lines snap to 45°, boxes become squares */
export const constrain = (kind: Shape, a: Point, b: Point): Point => {
  const dx = b.x - a.x, dy = b.y - a.y;
  if (kind === "rect" || kind === "ellipse") {
    const s = Math.max(Math.abs(dx), Math.abs(dy));
    return { x: a.x + Math.sign(dx || 1) * s, y: a.y + Math.sign(dy || 1) * s, p: b.p };
  }
  const angle = Math.round(Math.atan2(dy, dx) / (Math.PI / 4)) * (Math.PI / 4);
  const len = Math.hypot(dx, dy);
  return { x: a.x + Math.cos(angle) * len, y: a.y + Math.sin(angle) * len, p: b.p };
};

/**
 * Smart shapes: when the pen is held still at the end of a stroke, a rough
 * scribble becomes a clean line, rectangle or ellipse. Returns null when the
 * stroke doesn't look like any of them.
 */
export const recognize = (points: Point[]): { kind: Shape; a: Point; b: Point } | null => {
  if (points.length < 4) return null;
  const first = points[0], last = points[points.length - 1];
  let length = 0;
  for (let i = 1; i < points.length; i++) length += dist(points[i - 1], points[i]);
  if (length < 30) return null;

  // Straight line: nothing strays far from the first→last segment
  const chord = dist(first, last);
  const maxDev = Math.max(...points.map((p) => segDist(p, first, last)));
  if (chord > length * 0.85 && maxDev < Math.max(8, chord * 0.08)) return { kind: "line", a: first, b: last };

  // Closed shape: the end comes back near the start
  if (chord > length * 0.22) return null;
  const xs = points.map((p) => p.x), ys = points.map((p) => p.y);
  const a = { x: Math.min(...xs), y: Math.min(...ys), p: 0.5 };
  const b = { x: Math.max(...xs), y: Math.max(...ys), p: 0.5 };
  const w = b.x - a.x, h = b.y - a.y;
  if (w < 15 || h < 15) return null;
  // Shoelace area against the bounding box: ~0.79 for an ellipse, ~1 for a box
  let area = 0;
  for (let i = 0; i < points.length; i++) {
    const p = points[i], q = points[(i + 1) % points.length];
    area += p.x * q.y - q.x * p.y;
  }
  const fill = Math.abs(area / 2) / (w * h);
  if (fill < 0.55) return null;
  return { kind: fill > 0.88 ? "rect" : "ellipse", a, b };
};

/** Whether the eraser at p (radius r) touches the element */
export const hits = (el: Element, p: Point, r: number, measure: (el: Element) => { w: number; h: number }): boolean => {
  const reach = r + el.size / 2;
  switch (el.kind) {
    case "pen":
    case "highlighter":
      for (let i = 1; i < el.points.length; i++) if (segDist(p, el.points[i - 1], el.points[i]) < reach) return true;
      return el.points.length === 1 && dist(p, el.points[0]) < reach;
    case "line":
    case "arrow":
      return segDist(p, el.a, el.b) < reach;
    case "rect": {
      const c = [el.a, { x: el.b.x, y: el.a.y, p: 0 }, el.b, { x: el.a.x, y: el.b.y, p: 0 }];
      return c.some((q, i) => segDist(p, q, c[(i + 1) % 4]) < reach);
    }
    case "ellipse": {
      const cx = (el.a.x + el.b.x) / 2, cy = (el.a.y + el.b.y) / 2;
      const rx = Math.abs(el.b.x - el.a.x) / 2 || 1, ry = Math.abs(el.b.y - el.a.y) / 2 || 1;
      // Approximate distance to the outline via the normalized radius
      const k = Math.hypot((p.x - cx) / rx, (p.y - cy) / ry);
      return Math.abs(k - 1) * Math.min(rx, ry) < reach;
    }
    case "text": {
      const { w, h } = measure(el);
      return p.x > el.at.x - r && p.x < el.at.x + w + r && p.y > el.at.y - r && p.y < el.at.y + h + r;
    }
    case "number":
      return dist(p, el.at) < el.size * 2.2 + 10 + r;
  }
};

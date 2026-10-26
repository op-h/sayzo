import { getStroke } from "perfect-freehand";
import type { Element, Point } from "./model";

export const FONT = "Inter, 'Segoe UI', system-ui, sans-serif";
export const textSize = (size: number) => 18 + size * 4;
export const badgeRadius = (size: number) => 12 + size * 1.6;

/** Smooth, pressure-sensitive outline from perfect-freehand */
const outline = (points: Point[], size: number, highlighter: boolean) =>
  getStroke(
    points.map((p) => [p.x, p.y, p.p]),
    highlighter
      ? { size: size * 4, thinning: 0, smoothing: 0.6, streamline: 0.5, simulatePressure: false }
      : { size: size * 1.6, thinning: 0.55, smoothing: 0.5, streamline: 0.45, simulatePressure: points.every((p) => p.p === 0.5) }
  );

const fillOutline = (ctx: CanvasRenderingContext2D, poly: number[][]) => {
  if (!poly.length) return;
  ctx.beginPath();
  ctx.moveTo(poly[0][0], poly[0][1]);
  for (let i = 1; i < poly.length; i++) {
    // Quadratic curves through midpoints keep the edge smooth
    const [x0, y0] = poly[i - 1], [x1, y1] = poly[i];
    ctx.quadraticCurveTo(x0, y0, (x0 + x1) / 2, (y0 + y1) / 2);
  }
  ctx.closePath();
  ctx.fill();
};

const arrowHead = (ctx: CanvasRenderingContext2D, a: Point, b: Point, size: number) => {
  const angle = Math.atan2(b.y - a.y, b.x - a.x);
  const len = Math.min(10 + size * 3.2, Math.hypot(b.x - a.x, b.y - a.y) * 0.6);
  ctx.beginPath();
  ctx.moveTo(b.x, b.y);
  ctx.lineTo(b.x - len * Math.cos(angle - Math.PI / 7), b.y - len * Math.sin(angle - Math.PI / 7));
  ctx.lineTo(b.x - len * Math.cos(angle + Math.PI / 7), b.y - len * Math.sin(angle + Math.PI / 7));
  ctx.closePath();
  ctx.fill();
};

/** Black or white, whichever reads better on the given colour */
export const contrastText = (hex: string) => {
  const n = parseInt(hex.slice(1), 16);
  const lum = 0.299 * ((n >> 16) & 255) + 0.587 * ((n >> 8) & 255) + 0.114 * (n & 255);
  return lum > 160 ? "#111827" : "#ffffff";
};

export const drawElement = (ctx: CanvasRenderingContext2D, el: Element, now: number) => {
  ctx.save();
  let alpha = 1;
  if (el.fadeAt) {
    // Fully visible until the last second, then fades out
    alpha = Math.max(0, Math.min(1, (el.fadeAt - now) / 1000));
  }
  ctx.globalAlpha = alpha * (el.kind === "highlighter" ? 0.38 : 1);
  ctx.fillStyle = el.color;
  ctx.strokeStyle = el.color;
  ctx.lineWidth = el.size;
  ctx.lineCap = "round";
  ctx.lineJoin = "round";
  switch (el.kind) {
    case "pen":
    case "highlighter":
      fillOutline(ctx, outline(el.points, el.size, el.kind === "highlighter"));
      break;
    case "line":
    case "arrow":
      ctx.beginPath();
      ctx.moveTo(el.a.x, el.a.y);
      ctx.lineTo(el.b.x, el.b.y);
      ctx.stroke();
      if (el.kind === "arrow") arrowHead(ctx, el.a, el.b, el.size);
      break;
    case "rect": {
      const x = Math.min(el.a.x, el.b.x), y = Math.min(el.a.y, el.b.y);
      ctx.beginPath();
      ctx.roundRect(x, y, Math.abs(el.b.x - el.a.x), Math.abs(el.b.y - el.a.y), Math.min(8, el.size * 1.5));
      ctx.stroke();
      break;
    }
    case "ellipse":
      ctx.beginPath();
      ctx.ellipse((el.a.x + el.b.x) / 2, (el.a.y + el.b.y) / 2, Math.abs(el.b.x - el.a.x) / 2, Math.abs(el.b.y - el.a.y) / 2, 0, 0, Math.PI * 2);
      ctx.stroke();
      break;
    case "text": {
      const px = textSize(el.size);
      ctx.font = `600 ${px}px ${FONT}`;
      ctx.textBaseline = "top";
      // A soft outline keeps text readable on any background
      ctx.lineWidth = Math.max(2, px / 7);
      ctx.strokeStyle = contrastText(el.color) === "#ffffff" ? "rgba(255,255,255,0.85)" : "rgba(0,0,0,0.55)";
      el.text.split("\n").forEach((line, i) => {
        ctx.strokeText(line, el.at.x, el.at.y + i * px * 1.25);
        ctx.fillText(line, el.at.x, el.at.y + i * px * 1.25);
      });
      break;
    }
    case "number": {
      const r = badgeRadius(el.size);
      ctx.beginPath();
      ctx.arc(el.at.x, el.at.y, r, 0, Math.PI * 2);
      ctx.shadowColor = "rgba(0,0,0,0.35)";
      ctx.shadowBlur = 6;
      ctx.fill();
      ctx.shadowBlur = 0;
      ctx.fillStyle = contrastText(el.color);
      ctx.font = `700 ${r * 1.1}px ${FONT}`;
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      ctx.fillText(String(el.n), el.at.x, el.at.y + 1);
      break;
    }
  }
  ctx.restore();
};

export interface LaserPoint {
  x: number;
  y: number;
  t: number;
}

/** How long each part of the laser trail lives */
export const LASER_MS = 650;

/** Glowing trail that shrinks and fades behind the cursor */
export const drawLaser = (ctx: CanvasRenderingContext2D, trail: LaserPoint[], color: string, now: number) => {
  if (trail.length < 2) return;
  ctx.save();
  ctx.lineCap = "round";
  ctx.lineJoin = "round";
  ctx.shadowColor = color;
  ctx.shadowBlur = 14;
  for (let i = 1; i < trail.length; i++) {
    const life = 1 - (now - trail[i].t) / LASER_MS;
    if (life <= 0) continue;
    ctx.globalAlpha = life;
    ctx.strokeStyle = color;
    ctx.lineWidth = 2 + 6 * life;
    ctx.beginPath();
    ctx.moveTo(trail[i - 1].x, trail[i - 1].y);
    ctx.lineTo(trail[i].x, trail[i].y);
    ctx.stroke();
  }
  ctx.restore();
};

/** Darkens everything except a soft circle around the cursor */
export const drawSpotlight = (ctx: CanvasRenderingContext2D, w: number, h: number, at: { x: number; y: number }) => {
  ctx.save();
  ctx.fillStyle = "rgba(0,0,0,0.62)";
  ctx.fillRect(0, 0, w, h);
  ctx.globalCompositeOperation = "destination-out";
  const g = ctx.createRadialGradient(at.x, at.y, 110, at.x, at.y, 150);
  g.addColorStop(0, "rgba(0,0,0,1)");
  g.addColorStop(1, "rgba(0,0,0,0)");
  ctx.fillStyle = g;
  ctx.beginPath();
  ctx.arc(at.x, at.y, 150, 0, Math.PI * 2);
  ctx.fill();
  ctx.restore();
};

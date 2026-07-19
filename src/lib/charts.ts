/**
 * Pure chart math for the Dashboard. No DOM, no stores — typed functions the
 * chart components call to turn data into path strings / dasharrays / scales.
 */

export interface Point {
  x: number;
  y: number;
}

export function normalizeChartIndex(current: number, length: number): number | null {
  if (!Number.isInteger(length) || length <= 0 || !Number.isInteger(current)) return null;
  return Math.min(Math.max(current, 0), length - 1);
}

export function nextChartIndex(current: number, key: string, length: number): number | null {
  const normalized = normalizeChartIndex(current, length);
  if (normalized === null) return null;

  switch (key) {
    case "ArrowRight":
      return (normalized + 1) % length;
    case "ArrowLeft":
      return (normalized - 1 + length) % length;
    case "Home":
      return 0;
    case "End":
      return length - 1;
    default:
      return null;
  }
}

/**
 * Build a smooth cubic-bezier path through the given points using a
 * Catmull-Rom -> Bezier conversion (monotone-ish smoothing). Returns an SVG
 * path `d` string starting with `M`.
 */
export function smoothPath(points: Point[]): string {
  if (points.length === 0) return "";
  if (points.length === 1) return `M ${points[0].x} ${points[0].y}`;
  if (points.length === 2) {
    return `M ${points[0].x} ${points[0].y} L ${points[1].x} ${points[1].y}`;
  }

  const d: string[] = [`M ${points[0].x} ${points[0].y}`];
  const tension = 6; // Catmull-Rom -> Bezier divisor (standard 1/6 tangent scale)

  for (let i = 0; i < points.length - 1; i++) {
    const p0 = points[i - 1] ?? points[i];
    const p1 = points[i];
    const p2 = points[i + 1];
    const p3 = points[i + 2] ?? p2;

    const cp1x = p1.x + (p2.x - p0.x) / tension;
    const cp1y = p1.y + (p2.y - p0.y) / tension;
    const cp2x = p2.x - (p3.x - p1.x) / tension;
    const cp2y = p2.y - (p3.y - p1.y) / tension;

    d.push(`C ${cp1x} ${cp1y}, ${cp2x} ${cp2y}, ${p2.x} ${p2.y}`);
  }

  return d.join(" ");
}

export interface DonutSegment {
  dasharray: string;
  dashoffset: number;
  frac: number;
  /** True when the arc is short enough that round caps would overlap — use butt caps instead. */
  butt: boolean;
}

export interface DonutOpts {
  /** Circle radius in px. */
  r: number;
  /** Visual gap between adjacent segments, in px of arc length. */
  gapPx: number;
  /** Stroke width in px — used to derive the round-cap allowance (capPx = strokeWidth / 2 per end). */
  capPx: number;
}

/**
 * Compute per-segment dasharray/dashoffset for a ring built from stacked
 * `<circle>` strokes sharing one circumference. Each segment's visible arc is
 * shrunk by the gap AND by the round-cap allowance (capPx per end) so
 * adjacent round-capped strokes don't visually overlap. If a segment's raw
 * arc length is smaller than 2x the cap allowance, `butt: true` is returned
 * so the caller can switch that segment to butt caps (a round cap would
 * otherwise balloon past the segment's own bounds).
 */
export function donutSegments(values: number[], opts: DonutOpts): DonutSegment[] {
  const { r, gapPx, capPx } = opts;
  const circumference = 2 * Math.PI * r;
  const total = values.reduce((a, b) => a + b, 0);

  if (total <= 0 || values.length === 0) return [];

  let cursor = 0;
  return values.map((v): DonutSegment => {
    const frac = v / total;
    const rawArc = frac * circumference;
    const butt = rawArc < 2 * capPx;
    const capAllowance = butt ? 0 : 2 * capPx;
    const visibleArc = Math.max(0, rawArc - gapPx - capAllowance);
    const dasharray = `${visibleArc} ${circumference - visibleArc}`;
    // Offset so this segment starts where the previous one ended, rotated to
    // start at 12 o'clock (handled by the caller's -90deg transform); half
    // the cap allowance shifts the start inward to keep the segment centered
    // in its slot once caps are added back visually.
    const dashoffset = -(cursor + capAllowance / 2) - gapPx / 2;
    cursor += rawArc;
    return { dasharray, dashoffset, frac, butt };
  });
}

/**
 * Round a value up to a "nice" headroom max for a bar/line y-axis, so the
 * highest data point never touches the top gridline. Picks from a 1/2/5/10
 * step sequence scaled to the value's magnitude.
 */
export function niceMax(v: number): number {
  if (!Number.isFinite(v) || v <= 0) return 1;
  const withHeadroom = v * 1.15;
  const magnitude = Math.pow(10, Math.floor(Math.log10(withHeadroom)));
  const normalized = withHeadroom / magnitude;

  let step: number;
  if (normalized <= 1) step = 1;
  else if (normalized <= 2) step = 2;
  else if (normalized <= 5) step = 5;
  else step = 10;

  return step * magnitude;
}

/** A horizontal run of same-coloured pixels in 1-bit art. */
export interface Run {
  x: number;
  y: number;
  width: number;
  pixel: string;
}

/** Collapses art rows into runs, skipping `.`, so a sprite renders as a few
    dozen SVG rects instead of one per pixel. */
export function pixelRuns(rows: readonly string[]): Run[] {
  const out: Run[] = [];
  rows.forEach((row, y) => {
    let x = 0;
    while (x < row.length) {
      const pixel = row.charAt(x);
      if (pixel === ".") {
        x += 1;
        continue;
      }
      let end = x + 1;
      while (row.charAt(end) === pixel) end += 1;
      out.push({ x, y, width: end - x, pixel });
      x = end;
    }
  });
  return out;
}

/** The theme colour of each pixel code; anything else is ink. */
export const FILLS: Readonly<Record<string, string>> = {
  r: "var(--red)",
  p: "var(--potion)",
};

export function fillOf(pixel: string): string {
  return FILLS[pixel] ?? "var(--ink)";
}

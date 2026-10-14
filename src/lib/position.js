/**
 * Places a floating element next to an anchor inside the viewport.
 * Prefers below; flips above when there isn't room. Returns fixed
 * coordinates in px.
 *
 * @param {DOMRect} anchor
 * @param {{ width: number, height: number }} floating
 * @param {{ align?: "start" | "center" | "end", gap?: number, margin?: number }} [options]
 */
export function placeFloating(anchor, floating, options = {}) {
  const { align = "start", gap = 4, margin = 8 } = options;
  const vw = window.innerWidth;
  const vh = window.innerHeight;

  let top = anchor.bottom + gap;
  if (
    top + floating.height > vh - margin &&
    anchor.top - gap - floating.height >= margin
  ) {
    top = anchor.top - gap - floating.height;
  }

  let left =
    align === "center"
      ? anchor.left + anchor.width / 2 - floating.width / 2
      : align === "end"
        ? anchor.right - floating.width
        : anchor.left;
  left = Math.min(Math.max(left, margin), vw - margin - floating.width);

  return { top: Math.round(top), left: Math.round(left) };
}

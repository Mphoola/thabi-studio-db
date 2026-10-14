/**
 * Small fuzzy matcher for the command palette. Every query character must
 * appear in order. Consecutive matches and matches at word starts score
 * higher. Returns -1 when the text doesn't match.
 *
 * @param {string} query
 * @param {string} text
 */
export function fuzzyScore(query, text) {
  const q = query.trim().toLowerCase();
  if (!q) return 0;
  const t = text.toLowerCase();
  let score = 0;
  let ti = 0;
  let previous = -2;
  for (const ch of q) {
    if (ch === " ") continue;
    const found = t.indexOf(ch, ti);
    if (found === -1) return -1;
    const atWordStart = found === 0 || /[\s:/._-]/.test(t[found - 1]);
    score += 1 + (found === previous + 1 ? 3 : 0) + (atWordStart ? 2 : 0);
    previous = found;
    ti = found + 1;
  }
  // Prefer shorter texts when scores tie.
  return score - t.length * 0.01;
}

/**
 * @template T
 * @param {string} query
 * @param {T[]} items
 * @param {(item: T) => string} getText
 * @returns {T[]}
 */
export function fuzzyFilter(query, items, getText) {
  if (!query.trim()) return items;
  return items
    .map((item) => ({ item, score: fuzzyScore(query, getText(item)) }))
    .filter((entry) => entry.score >= 0)
    .sort((a, b) => b.score - a.score)
    .map((entry) => entry.item);
}

/**
 * How a tab is presented in the tab bar.
 *
 * Out of the component because a tab is not always about one database object:
 * a query tab has no node, and a diagram tab has no node either. Anything
 * reading `tab.node` without checking the type first will throw on the first
 * tab kind anybody adds, which is the wrong moment to find out.
 */
import { Eye, Network, SquareTerminal, Table2 } from "lucide-vue-next";

/** True for tabs that stand for one object in the explorer. */
export const isObjectTab = (tab) => tab?.type === "object";

/** The icon a tab is shown with. */
export function tabIcon(tab) {
  if (isObjectTab(tab)) return tab.node?.kind === "table" ? Table2 : Eye;
  // Checked by type rather than by the presence of a node, so a tab that has
  // no node cannot fall through to a line that assumes it has one.
  if (tab?.type === "diagram") return Network;
  return SquareTerminal;
}

/** The title, falling back to something rather than rendering nothing. */
export function tabTitle(tab) {
  return tab?.title?.trim() || "Untitled";
}

/**
 * How saved connections are arranged on the connections page and in the
 * switcher.
 *
 * A connection with a group sits under that user-named group; one without
 * falls back to its environment, so the page still reads sensibly before
 * anyone creates a group. Rust lists profiles grouped and ordered already
 * (`(group_name = ''), group_name, sort_order, name`), and this keeps that
 * order inside each section.
 */

import { environmentFor } from "./environments.js";

/** Production first: it's the one to notice. */
const ENVIRONMENT_ORDER = [
  "production",
  "staging",
  "testing",
  "development",
  "local",
];

/** The tag tokens a connection can be coloured with (see tokens.css). */
export const COLOURS = [
  { value: "", label: "Environment colour" },
  { value: "red", label: "Red" },
  { value: "orange", label: "Orange" },
  { value: "yellow", label: "Yellow" },
  { value: "green", label: "Green" },
  { value: "teal", label: "Teal" },
  { value: "blue", label: "Blue" },
  { value: "purple", label: "Purple" },
  { value: "gray", label: "Gray" },
];

export function isColour(value) {
  return COLOURS.some((colour) => colour.value === value);
}

/** The dot class for a connection: its custom colour, or the environment's. */
export function colourDotClass(profile) {
  const colour = profile?.colour || "";
  return colour
    ? `bg-tag-${colour}`
    : environmentFor(profile?.environment).dotClass;
}

/** The left border of a group's card, matching its dot. */
export function groupEdgeClass(group) {
  return group.user
    ? "border-l-tag-gray"
    : `border-l-tag-${environmentFor(group.key).token}`;
}

/**
 * The sections to show, user groups first (alphabetical), then the
 * environment sections for anything ungrouped.
 *
 * @param {object[]} profiles in the order Rust returned them
 * @param {(profile: object) => boolean} [matches] search filter
 */
export function groupConnections(profiles, matches = () => true) {
  const visible = profiles.filter(matches);
  /** @type {{ key: string, label: string, user: boolean, profiles: object[] }[]} */
  const groups = [];
  const byName = new Map();

  for (const profile of visible) {
    const name = (profile.groupName ?? "").trim();
    if (!name) continue;
    let group = byName.get(name.toLowerCase());
    if (!group) {
      group = { key: name, label: name, user: true, profiles: [] };
      byName.set(name.toLowerCase(), group);
      groups.push(group);
    }
    group.profiles.push(profile);
  }

  const loose = visible.filter((p) => !(p.groupName ?? "").trim());
  const byEnvironment = new Map();
  for (const profile of loose) {
    let group = byEnvironment.get(profile.environment);
    if (!group) {
      const environment = environmentFor(profile.environment);
      group = {
        key: environment.value,
        label: environment.label,
        user: false,
        profiles: [],
      };
      byEnvironment.set(profile.environment, group);
    }
    group.profiles.push(profile);
  }

  const environmentGroups = ENVIRONMENT_ORDER.map((value) =>
    byEnvironment.get(value),
  ).filter(Boolean);
  // Environments the list doesn't know about still get a section.
  for (const group of byEnvironment.values()) {
    if (!environmentGroups.includes(group)) environmentGroups.push(group);
  }

  return [...groups, ...environmentGroups];
}

/**
 * The same order Rust lists profiles in: user groups first, then the manual
 * order inside each, then the name. Used after a save, before the next
 * `list_connection_profiles` round trip.
 */
export function sortProfiles(profiles) {
  return [...profiles].sort((a, b) => {
    const groupA = (a.groupName ?? "").trim().toLowerCase();
    const groupB = (b.groupName ?? "").trim().toLowerCase();
    if (!groupA !== !groupB) return groupA ? -1 : 1;
    if (groupA !== groupB) return groupA.localeCompare(groupB);
    if ((a.sortOrder ?? 0) !== (b.sortOrder ?? 0))
      return (a.sortOrder ?? 0) - (b.sortOrder ?? 0);
    return a.name.localeCompare(b.name);
  });
}

/**
 * Where a dragged profile landed, as the new order of one group's ids.
 * `targetId` is the row it was dropped on; dropping on itself changes nothing.
 *
 * @returns {string[]} ids in their new order
 */
export function reorderWithin(ids, draggedId, targetId) {
  const from = ids.indexOf(draggedId);
  if (from < 0) return [...ids];
  // Take the row out first, then drop it where the target now sits, so it
  // lands on the target's place instead of one row past it.
  const order = [...ids];
  const [moved] = order.splice(from, 1);
  const to = order.indexOf(targetId);
  if (to < 0) return [...ids];
  order.splice(to, 0, moved);
  return order;
}

/** Groups are folded away by name; environment sections always start open. */
export function isCollapsed(collapsed, group) {
  return group.user && (collapsed ?? []).includes(group.key);
}

/**
 * The list with `profile` in it: replacing the saved copy when it is already
 * there (an edit), adding it when it is new (a create), in display order.
 */
export function upsertProfile(profiles, profile) {
  const exists = profiles.some((p) => p.id === profile.id);
  return sortProfiles(
    exists
      ? profiles.map((p) => (p.id === profile.id ? profile : p))
      : [...profiles, profile],
  );
}

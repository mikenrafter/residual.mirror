// Pure state-merging layer for the import modal (Phase 6 of
// residual/iterations/view-live-editing-plan.md): takes PendingItems already
// parsed by import-parser.ts's `parseImportText` and applies them onto an
// existing `PendingState` by delegating to the existing reducers in
// actions.ts wherever the shapes line up. No DOM here — see import-modal.ts
// for the DOM-wiring layer that calls this.
//
// Design notes (decisions made beyond the task sketch, documented per
// instructions):
//
// - `mergeImportedItems` returns `{ state, unmergeable }` rather than
//   throwing or silently dropping anything it can't apply. Per
//   web/generated/cli-schema.json + actions.ts, there is currently no
//   generic "update this attractor/component/term/persona's fields" reducer
//   (only `updateForceField` exists, and it's forces-only) — so
//   `{ kind: "update", type: "attractor" | "component" | "term" | "persona" }`
//   items are parsed successfully upstream but have nothing to apply here.
//   Rather than crash or drop them invisibly, they come back in
//   `unmergeable` so the DOM layer can surface them (e.g. re-show as an
//   error) instead of pretending the import fully succeeded.
// - `{ kind: "add", type: "stressor" | "purpose" }` fields map onto
//   `updateForceField`'s field names: fields.description -> "description",
//   fields["attractor-shortname"] -> "attractorId", fields["naive-change"] ->
//   "naiveChangeOrFeature" (cli-schema.json uses "naive-change" for BOTH
//   `add stressor` and `add purpose` — there is no separate "feature" flag
//   name at the CLI layer, despite the plan prose using "feature" for
//   purposes; ground truth is cli-schema.json, mirroring model.ts's existing
//   documented policy of trusting the schema over plan prose),
//   fields.outcomes -> "outcomes", fields.shortname -> "shortname".
// - `add stressor`/`add purpose` have no `--add-component` flag in
//   cli-schema.json (only `update stressor`/`update purpose` do), so
//   `multipleFields["add-component"]` can never actually be populated by
//   import-parser.ts for an "add" item — parseImportText only ever
//   populates multipleFields for flags the schema marks `multiple: true`
//   for that exact subcommand. The defensive handling below (toggle it on
//   if somehow present) is therefore unreachable in practice and
//   deliberately untested per the task's own carve-out; it exists only so a
//   malformed/future PendingItem can't crash this function.
// - Toggle-guard for `update`'s `add-component`/`remove-component`: naively
//   calling `toggleComponent` unconditionally is WRONG for
//   already-matching state — toggling "add" a component that's already
//   coupled would flip it OFF, and toggling "remove" a component that's
//   already absent would flip it ON. Both are the opposite of user intent.
//   `applyComponentToggles` below only calls `toggleComponent` when doing so
//   would actually change the coupling state, checked via
//   `currentComponentsFor` (which reads the added-force's list directly, or
//   merges a base force with its pending update — mirroring the same merge
//   `toggleComponent`/`computeStateValidity` already perform internally).
// - Synthetic attractor id scheme mirrors forms.ts's `NEW-ATTR-<n>` (max + 1
//   over existing `addedAttractors` ids matching `/^NEW-ATTR-(\d+)$/`) —
//   that helper isn't exported from forms.ts, so it's reimplemented
//   identically here rather than modifying forms.ts (out of scope per the
//   task's file-immutability list).

import { addAttractorOption, addComponentColumn, addForceRow, removeForce, toggleComponent, updateForceField } from "./actions";
import type { PendingState } from "./model";
import type { PendingItem } from "./import-parser";

/** Result of merging a batch of parsed PendingItems into a PendingState. */
export interface MergeResult {
  state: PendingState;
  /** Items that parsed successfully but have no reducer support to apply yet (see module notes). */
  unmergeable: PendingItem[];
}

type ForceFieldName = "description" | "attractorId" | "naiveChangeOrFeature" | "outcomes" | "shortname";

/** Maps cli-schema.json flag names (kebab-case) to actions.ts's ForceUpdate field names, for stressor/purpose add & update. */
const FORCE_FIELD_MAP: Record<string, ForceFieldName> = {
  description: "description",
  "attractor-shortname": "attractorId",
  "naive-change": "naiveChangeOrFeature",
  outcomes: "outcomes",
  shortname: "shortname",
};

/** Next `NEW-ATTR-<n>` synthetic id, mirroring forms.ts's (unexported) scheme. */
function nextAttractorId(state: PendingState): string {
  let max = 0;
  for (const a of state.addedAttractors) {
    const match = /^NEW-ATTR-(\d+)$/.exec(a.id);
    if (match) {
      const n = Number(match[1]);
      if (n > max) max = n;
    }
  }
  return `NEW-ATTR-${max + 1}`;
}

/** The effective (base-merged-with-pending-update, or added-force) component list for `forceKey`. */
function currentComponentsFor(state: PendingState, forceKey: string): string[] {
  const added = state.addedForces.find((f) => f.tempId === forceKey);
  if (added !== undefined) return added.components;
  const base = state.baseForces.find((f) => f.id === forceKey);
  if (base === undefined) return [];
  const update = state.updatedForces[forceKey];
  return update?.components ?? base.components;
}

/** Applies add-component/remove-component multipleFields for an update item, only toggling when it actually changes coupling state. */
function applyComponentToggles(state: PendingState, forceKey: string, item: PendingItem): PendingState {
  let next = state;
  for (const name of item.multipleFields["add-component"] ?? []) {
    if (!currentComponentsFor(next, forceKey).includes(name)) {
      next = toggleComponent(next, forceKey, name);
    }
  }
  for (const name of item.multipleFields["remove-component"] ?? []) {
    if (currentComponentsFor(next, forceKey).includes(name)) {
      next = toggleComponent(next, forceKey, name);
    }
  }
  return next;
}

/** Applies an `add stressor`/`add purpose` item: creates the row, then sets every mapped field present in `item.fields`. */
function applyAddForce(state: PendingState, kind: "stressor" | "purpose", item: PendingItem): PendingState {
  const { state: withRow, tempId } = addForceRow(state, kind);
  let next = withRow;
  for (const [flagName, rawValue] of Object.entries(item.fields)) {
    const field = FORCE_FIELD_MAP[flagName];
    if (field === undefined) continue;
    const value = flagName === "attractor-shortname"
      ? [...next.baseAttractors, ...next.addedAttractors].find((attractor) => attractor.name === rawValue)?.id ?? rawValue
      : rawValue;
    next = updateForceField(next, tempId, field, value);
  }
  // Defensive only — see module notes: add stressor/add purpose have no
  // multiple-valued flags in cli-schema.json, so this is unreachable today.
  next = applyComponentToggles(next, tempId, item);
  return next;
}

/** Applies an `add component` item via addComponentColumn. */
function applyAddComponent(state: PendingState, item: PendingItem): PendingState {
  return addComponentColumn(state, {
    name: item.fields.name ?? "",
    description: item.fields.description ?? "",
    status: (item.fields.status as "proposed" | "actual") ?? "proposed",
    architectureSet: item.fields["architecture-set"] ?? "",
  });
}

/** Applies an `add attractor` item via addAttractorOption, synthesizing a NEW-ATTR-<n> id. */
function applyAddAttractor(state: PendingState, item: PendingItem): PendingState {
  const id = nextAttractorId(state);
  return addAttractorOption(state, {
    id,
    name: item.fields.name ?? "",
    description: item.fields.description ?? "",
    positiveState: item.fields["positive-state"] ?? "",
    negativeState: item.fields["negative-state"] ?? "",
  });
}

/** Applies an `update stressor`/`update purpose` item: field updates by force-id, then guarded component toggles. */
function applyUpdateForce(state: PendingState, item: PendingItem): PendingState {
  const forceKey = item.fields["force-id"];
  if (forceKey === undefined) return state;

  let next = state;
  for (const [flagName, rawValue] of Object.entries(item.fields)) {
    if (flagName === "force-id") continue;
    const field = FORCE_FIELD_MAP[flagName];
    if (field === undefined) continue;
    const value = flagName === "attractor-shortname"
      ? [...next.baseAttractors, ...next.addedAttractors].find((attractor) => attractor.name === rawValue)?.id ?? rawValue
      : rawValue;
    next = updateForceField(next, forceKey, field, value);
  }
  next = applyComponentToggles(next, forceKey, item);
  return next;
}

function applyRemoveForce(state: PendingState, item: PendingItem): PendingState {
  const shortname = item.fields.shortname;
  if (shortname === undefined) return state;
  const force = state.baseForces.find((candidate) => candidate.kind === item.type && candidate.shortname === shortname);
  return force === undefined ? state : removeForce(state, force.id);
}

/** Whether this PendingItem has reducer support to actually apply. */
function isMergeable(item: PendingItem): boolean {
  if (item.kind === "add") {
    return item.type === "stressor" || item.type === "purpose" || item.type === "component" || item.type === "attractor";
  }
  return item.type === "stressor" || item.type === "purpose";
}

/** Applies a single mergeable PendingItem onto state, dispatching by kind/type. */
function applyItem(state: PendingState, item: PendingItem): PendingState {
  if (item.kind === "add") {
    if (item.type === "stressor" || item.type === "purpose") return applyAddForce(state, item.type, item);
    if (item.type === "component") return applyAddComponent(state, item);
    if (item.type === "attractor") return applyAddAttractor(state, item);
  }
  if (item.kind === "remove" && (item.type === "stressor" || item.type === "purpose")) {
    return applyRemoveForce(state, item);
  }
  // item.kind === "update", item.type === "stressor" | "purpose"
  return applyUpdateForce(state, item);
}

/**
 * Applies each parsed `PendingItem` onto `state` in order, delegating to
 * actions.ts's existing reducers. Items with no reducer support yet are
 * collected into `unmergeable` rather than applied, dropped, or thrown.
 */
export function mergeImportedItems(state: PendingState, items: PendingItem[]): MergeResult {
  let next = state;
  const unmergeable: PendingItem[] = [];

  for (const item of items) {
    if (!isMergeable(item)) {
      unmergeable.push(item);
      continue;
    }
    next = applyItem(next, item);
  }

  return { state: next, unmergeable };
}

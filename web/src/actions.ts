// Pure reducer layer for the live-editing DOM to call into: takes a
// PendingState (see model.ts) and a user action's arguments, returns a NEW
// PendingState. No DOM, no mutation of the input state.
//
// Sits between the (future) DOM-wiring layer (double-click handlers, context
// menus, edit forms) and model.ts's pure validity/rendering functions. This
// module owns "how does a user action change the state", not "is the state
// valid" or "how does it render to CLI text" — those stay in model.ts.
//
// Design notes:
//
// - toggleComponent, for a BASE force: toggling a component OFF that
//   restores the update's `components` array to exactly the base force's
//   original `components` list PRUNES the update entry back out of
//   `updatedForces` entirely (rather than leaving a no-op
//   `{ components: [...same as base] }` entry sitting around). This keeps
//   state minimal and keeps `toCommandLines`'s diffing in model.ts from ever
//   emitting a synthetic no-op update line. If the pruned update object would
//   still have OTHER fields set (e.g. a prior description edit), the entry
//   is kept with components removed rather than deleted outright — only
//   `components` is pruned. Pinned by the "toggle a base force's component
//   back to its original state, with no other changes, removes the
//   updatedForces entry" test in actions.test.ts.
// - addForceRow generates tempIds as "NEW-<n>" where n is one more than the
//   count of existing addedForces entries whose tempId already matches the
//   "NEW-<number>" pattern (max + 1, not count + 1, so it can't collide after
//   an intervening removal — no removal action exists yet, but this is
//   future-proof at no cost). Returns { state, tempId } so the DOM layer can
//   open the freshly-created row in edit mode without having to search for
//   it.
// - updateForceField accumulates into the same updatedForces[id]/addedForces
//   entry across repeated calls, only ever touching the one field named.

import type { AddedForce, PendingState, SnapshotAttractor, SnapshotComponent } from "./model";

/** Returns true if two string arrays contain the same elements, ignoring order. */
function sameElements(a: readonly string[], b: readonly string[]): boolean {
  if (a.length !== b.length) return false;
  const sortedA = [...a].sort();
  const sortedB = [...b].sort();
  return sortedA.every((value, index) => value === sortedB[index]);
}

/** Toggles `name` in/out of `components`, appending if absent, filtering out if present. */
function toggled(components: readonly string[], name: string): string[] {
  return components.includes(name) ? components.filter((c) => c !== name) : [...components, name];
}

/** Toggles `componentName` in/out of the effective component list for the force identified by `forceKey` (a base force id or an added force's tempId). */
export function toggleComponent(state: PendingState, forceKey: string, componentName: string): PendingState {
  const addedIndex = state.addedForces.findIndex((f) => f.tempId === forceKey);
  if (addedIndex !== -1) {
    const target = state.addedForces[addedIndex]!;
    const nextForce: AddedForce = { ...target, components: toggled(target.components, componentName) };
    const nextAddedForces = [...state.addedForces];
    nextAddedForces[addedIndex] = nextForce;
    return { ...state, addedForces: nextAddedForces };
  }

  const base = state.baseForces.find((f) => f.id === forceKey);
  if (base === undefined) {
    // Unknown force key — nothing to toggle against; return state unchanged.
    return state;
  }

  const existingUpdate = state.updatedForces[forceKey];
  const effective = existingUpdate?.components ?? base.components;
  const nextComponents = toggled(effective, componentName);

  const nextUpdatedForces = { ...state.updatedForces };

  if (sameElements(nextComponents, base.components)) {
    if (existingUpdate === undefined) {
      // Nothing was set before, and toggling landed back on the base list —
      // no entry needed.
      return state;
    }
    const { components: _components, ...rest } = existingUpdate;
    if (Object.keys(rest).length === 0) {
      delete nextUpdatedForces[forceKey];
    } else {
      nextUpdatedForces[forceKey] = rest;
    }
  } else {
    nextUpdatedForces[forceKey] = { ...existingUpdate, components: nextComponents };
  }

  return { ...state, updatedForces: nextUpdatedForces };
}

/** Appends a new empty AddedForce of the given kind, returning the new state and the freshly generated unique tempId. */
export function addForceRow(
  state: PendingState,
  kind: "stressor" | "purpose",
): { state: PendingState; tempId: string } {
  let maxN = 0;
  for (const f of state.addedForces) {
    const match = /^NEW-(\d+)$/.exec(f.tempId);
    if (match) {
      const n = Number(match[1]);
      if (n > maxN) maxN = n;
    }
  }
  const tempId = `NEW-${maxN + 1}`;

  const newForce: AddedForce = {
    tempId,
    kind,
    description: "",
    attractorId: "",
    naiveChangeOrFeature: "",
    outcomes: "",
    shortname: "",
    components: [],
  };

  return { state: { ...state, addedForces: [...state.addedForces, newForce] }, tempId };
}

/** Sets a single text field on the force identified by `forceKey` (base id or added-force tempId), preserving other already-set fields. */
export function updateForceField(
  state: PendingState,
  forceKey: string,
  field: "description" | "attractorId" | "naiveChangeOrFeature" | "outcomes" | "shortname",
  value: string,
): PendingState {
  const addedIndex = state.addedForces.findIndex((f) => f.tempId === forceKey);
  if (addedIndex !== -1) {
    const target = state.addedForces[addedIndex]!;
    const nextForce: AddedForce = { ...target, [field]: value };
    const nextAddedForces = [...state.addedForces];
    nextAddedForces[addedIndex] = nextForce;
    return { ...state, addedForces: nextAddedForces };
  }

  const existingUpdate = state.updatedForces[forceKey];
  return {
    ...state,
    updatedForces: {
      ...state.updatedForces,
      [forceKey]: { ...existingUpdate, [field]: value },
    },
  };
}

/** Appends a new component column. Throws if `component.name` already exists in baseComponents or addedComponents. */
export function addComponentColumn(state: PendingState, component: SnapshotComponent): PendingState {
  const exists =
    state.baseComponents.some((c) => c.name === component.name) ||
    state.addedComponents.some((c) => c.name === component.name);
  if (exists) {
    throw new Error(`component "${component.name}" already exists`);
  }
  return { ...state, addedComponents: [...state.addedComponents, component] };
}

/** Appends a new attractor option. Throws if `attractor.id` already exists in baseAttractors or addedAttractors. */
export function addAttractorOption(state: PendingState, attractor: SnapshotAttractor): PendingState {
  const exists =
    state.baseAttractors.some((a) => a.id === attractor.id) ||
    state.addedAttractors.some((a) => a.id === attractor.id);
  if (exists) {
    throw new Error(`attractor "${attractor.id}" already exists`);
  }
  return { ...state, addedAttractors: [...state.addedAttractors, attractor] };
}

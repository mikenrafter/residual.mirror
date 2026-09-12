// Pure, DOM-free "what should be visually marked" decision logic for the
// live-editing matrix view (Phase 5 of
// residual/iterations/view-live-editing-plan.md). render.ts (untested,
// DOM-wiring code, closed out via manual browser verification per the plan)
// is the only consumer of this module's output — it mutates the DOM, this
// module never does.
//
// Builds directly on web/src/model.ts (Phase 4, stable): imports
// `PendingState`, `computeStateValidity`, and the snapshot types rather than
// redefining or duplicating any of that.

import { computeStateValidity, type PendingState } from "./model.ts";

/**
 * Which sticky axes (row labels / component-column headers) should be
 * rendered invalid.
 *
 * Scoping decision (documented per instructions): the plan's Phase 5 line
 * ("red state ... propagated to the sticky header cell (column) and sticky
 * label cell (row) for any invalid cell's axis") reads, taken literally, as
 * if individual NKP *cells* can be invalid and blame a specific component
 * column. But the NKP matrix has no per-cell required/optional concept — a
 * cell is only ever "coupled" or "not coupled" (see model.ts's
 * `computeStateValidity`, which validates required *text* fields plus a
 * row-wide "zero components toggled" rule, never a specific force×component
 * pair). The only per-row invalidity that touches the matrix at all is that
 * whole-row "zero components" rule, and it does not implicate any single
 * column over another — every column is equally "missing" for that row.
 *
 * So `invalidComponentColumns` is scoped narrowly: it stays empty for the
 * "zero components toggled" case (there is no literal per-cell reading that
 * applies), and would only ever be populated by a genuine per-cell invalid
 * state, which does not exist in this domain today. This is a deliberate
 * narrowing of the spec, not an oversight — see
 * render-decisions.test.ts's "invalidComponentColumns" describe block for
 * the cases this claim is tested against.
 */
export interface InvalidMarks {
  /** Force keys (real id or tempId) whose sticky row label should be marked invalid. */
  invalidForceKeys: Set<string>;
  /** Component names whose sticky column header should be marked invalid. Always empty today — see module docs. */
  invalidComponentColumns: Set<string>;
}

/**
 * Computes which sticky row labels (and, in principle, column headers) need
 * invalid-marking for the current PendingState. Delegates all validity
 * business rules to `computeStateValidity` from model.ts — this function
 * does not reimplement or re-decide what makes a force invalid, it only
 * reshapes that verdict into a marking decision.
 */
export function computeInvalidMarks(state: PendingState): InvalidMarks {
  const { invalidForceIds } = computeStateValidity(state);
  return {
    invalidForceKeys: new Set(invalidForceIds),
    invalidComponentColumns: new Set(),
  };
}

/** Options controlling which components are visible in the rendered matrix. */
export interface VisibilityOptions {
  /** When false, components with status "proposed" are excluded. */
  showProposed: boolean;
  /** When false, components unrelated to `filteredForceIds` are excluded. */
  showUnrelated: boolean;
  /**
   * Restricts the "related" check to this set of force ids/tempIds.
   * `null` means no filter is active, i.e. "related" is checked against
   * every force (base + added) in the state.
   */
  filteredForceIds: string[] | null;
}

/**
 * Returns the component names that should be rendered as matrix columns,
 * in a stable order, given the current PendingState and visibility toggles.
 * Considers both `state.baseComponents` and `state.addedComponents`, and
 * (for the "unrelated" filter) each force's base-merged-with-pending-update
 * component list — mirroring the same merge model.ts's
 * `computeStateValidity` performs internally, without duplicating its
 * validity business rules.
 */
export function visibleComponents(state: PendingState, options: VisibilityOptions): string[] {
  const all = [...state.baseComponents, ...state.addedComponents];
  const filtered = options.showProposed ? all : all.filter((c) => c.status !== "proposed");

  if (options.showUnrelated) {
    return filtered.map((c) => c.name);
  }

  const relevantBaseForces = state.baseForces.filter(
    (f) => options.filteredForceIds === null || options.filteredForceIds.includes(f.id),
  );
  const relevantAddedForces = state.addedForces.filter(
    (f) => options.filteredForceIds === null || options.filteredForceIds.includes(f.tempId),
  );

  const relatedComponentNames = new Set<string>();
  for (const base of relevantBaseForces) {
    const update = state.updatedForces[base.id];
    const components = update?.components ?? base.components;
    for (const name of components) relatedComponentNames.add(name);
  }
  for (const added of relevantAddedForces) {
    for (const name of added.components) relatedComponentNames.add(name);
  }

  return filtered.filter((c) => relatedComponentNames.has(c.name)).map((c) => c.name);
}

/** One entry in the attractor `<select>` dropdown. */
export interface AttractorOption {
  id: string;
  name: string;
}

/**
 * Merges `state.baseAttractors` with `state.addedAttractors` (attractors
 * added this session, not yet part of the base snapshot) into a single
 * sorted-by-name list of dropdown options.
 */
export function attractorOptions(state: PendingState): AttractorOption[] {
  const all = [...state.baseAttractors, ...state.addedAttractors];
  return all
    .map((a) => ({ id: a.id, name: a.name }))
    .sort((a, b) => a.name.localeCompare(b.name));
}

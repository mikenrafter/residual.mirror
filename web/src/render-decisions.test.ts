import { describe, expect, test } from "bun:test";
import {
  attractorOptions,
  computeInvalidMarks,
  visibleComponents,
  type VisibilityOptions,
} from "./render-decisions";
import type { AddedForce, PendingState, SnapshotAttractor, SnapshotComponent, SnapshotForce } from "./model";

const baseAttractor: SnapshotAttractor = {
  id: "A-01",
  name: "resilience",
  description: "system tolerates partial failure",
  positiveState: "graceful degradation",
  negativeState: "cascading failure",
};

const secondAttractor: SnapshotAttractor = {
  id: "A-02",
  name: "architecture-clarity",
  description: "components stay legible",
  positiveState: "clear boundaries",
  negativeState: "tangled coupling",
};

const cliComponent: SnapshotComponent = {
  name: "cli",
  description: "command-line interface",
  status: "actual",
  architectureSet: "iter1",
};

const proposedComponent: SnapshotComponent = {
  name: "web-view",
  description: "browser-rendered landscape view",
  status: "proposed",
  architectureSet: "iter2",
};

const baseStressor: SnapshotForce = {
  id: "S-01",
  kind: "stressor",
  description: "queue backs up under load",
  attractorId: "A-01",
  naiveChangeOrFeature: "add retry",
  outcomes: "requests drain",
  shortname: "queue-overload",
  components: ["cli"],
};

const secondStressor: SnapshotForce = {
  id: "S-02",
  kind: "stressor",
  description: "operator misreads the matrix",
  attractorId: "A-01",
  naiveChangeOrFeature: "add legend",
  outcomes: "operator trusts matrix",
  shortname: "matrix-misread",
  components: ["web-view"],
};

function emptyState(overrides: Partial<PendingState> = {}): PendingState {
  return {
    baseAttractors: [],
    baseComponents: [],
    baseForces: [],
    basePersonas: [],
    baseTerms: [],
    addedAttractors: [],
    addedComponents: [],
    addedForces: [],
    addedPersonas: [],
    addedTerms: [],
    updatedAttractors: {},
    updatedComponents: {},
    updatedForces: {},
    updatedPersonas: {},
    updatedTerms: {},
    ...overrides,
  };
}

describe("computeInvalidMarks", () => {
  test("marks a base force's sticky row invalid when a required text field is blanked by an update", () => {
    const state = emptyState({
      baseForces: [baseStressor],
      updatedForces: { "S-01": { description: "" } },
    });
    const marks = computeInvalidMarks(state);
    expect(marks.invalidForceKeys.has("S-01")).toBe(true);
  });

  test("marks an added force's sticky row invalid when it has zero components toggled", () => {
    const added: AddedForce = { ...baseStressor, id: "S-02", tempId: "tmp-1", components: [] };
    const state = emptyState({ addedForces: [added] });
    const marks = computeInvalidMarks(state);
    expect(marks.invalidForceKeys.has("tmp-1")).toBe(true);
  });

  test("does not mark a valid force's sticky row", () => {
    const state = emptyState({ baseForces: [baseStressor] });
    const marks = computeInvalidMarks(state);
    expect(marks.invalidForceKeys.has("S-01")).toBe(false);
  });

  // Scoping decision (see render-decisions.ts module docs): the NKP matrix
  // has no per-cell required/optional concept, so "zero components toggled"
  // is a whole-row failure that does not point at any one component column
  // over another. invalidComponentColumns therefore stays empty for this
  // case, even though the row itself is marked invalid above.
  test("invalidComponentColumns stays empty for the zero-components case, even with components in play", () => {
    const state = emptyState({
      baseComponents: [cliComponent, proposedComponent],
      baseForces: [{ ...baseStressor, components: [] }],
    });
    const marks = computeInvalidMarks(state);
    expect(marks.invalidComponentColumns.size).toBe(0);
  });

  test("invalidComponentColumns stays empty even when multiple forces are invalid for text-field reasons", () => {
    const state = emptyState({
      baseComponents: [cliComponent],
      baseForces: [baseStressor, secondStressor],
      updatedForces: {
        "S-01": { attractorId: "" },
        "S-02": { naiveChangeOrFeature: "" },
      },
    });
    const marks = computeInvalidMarks(state);
    expect(marks.invalidForceKeys.has("S-01")).toBe(true);
    expect(marks.invalidForceKeys.has("S-02")).toBe(true);
    expect(marks.invalidComponentColumns.size).toBe(0);
  });
});

describe("visibleComponents", () => {
  const allVisible: VisibilityOptions = { showProposed: true, showUnrelated: true, filteredForceIds: null };

  test("returns all components when both toggles are on and no filter is active", () => {
    const state = emptyState({
      baseComponents: [cliComponent, proposedComponent],
      baseForces: [baseStressor, secondStressor],
    });
    expect(visibleComponents(state, allVisible)).toEqual(["cli", "web-view"]);
  });

  test("excludes proposed base components when showProposed is false", () => {
    const state = emptyState({
      baseComponents: [cliComponent, proposedComponent],
      baseForces: [baseStressor, secondStressor],
    });
    const result = visibleComponents(state, { ...allVisible, showProposed: false });
    expect(result).toEqual(["cli"]);
  });

  test("excludes proposed components from addedComponents too, not just baseComponents", () => {
    const state = emptyState({
      baseComponents: [cliComponent],
      addedComponents: [{ ...proposedComponent, name: "added-proposed" }],
      baseForces: [baseStressor],
    });
    const result = visibleComponents(state, { ...allVisible, showProposed: false });
    expect(result).toEqual(["cli"]);
  });

  test("with filteredForceIds set and showUnrelated false, keeps only components used by the filtered forces", () => {
    const state = emptyState({
      baseComponents: [cliComponent, proposedComponent],
      baseForces: [baseStressor, secondStressor],
    });
    const result = visibleComponents(state, {
      showProposed: true,
      showUnrelated: false,
      filteredForceIds: ["S-01"],
    });
    expect(result).toEqual(["cli"]);
  });

  test("with filteredForceIds null and showUnrelated false, checks relation against ALL forces (no filter active)", () => {
    const state = emptyState({
      baseComponents: [cliComponent, proposedComponent],
      baseForces: [baseStressor, secondStressor],
    });
    const result = visibleComponents(state, {
      showProposed: true,
      showUnrelated: false,
      filteredForceIds: null,
    });
    expect(result).toEqual(["cli", "web-view"]);
  });

  test("relatedness check uses a force's pending-update-merged components, not just its base components", () => {
    const state = emptyState({
      baseComponents: [cliComponent, proposedComponent],
      baseForces: [baseStressor],
      updatedForces: { "S-01": { components: ["web-view"] } },
    });
    const result = visibleComponents(state, {
      showProposed: true,
      showUnrelated: false,
      filteredForceIds: ["S-01"],
    });
    expect(result).toEqual(["web-view"]);
  });

  test("relatedness check also considers addedForces' components when filteredForceIds references a tempId", () => {
    const added: AddedForce = { ...secondStressor, tempId: "tmp-1", components: ["web-view"] };
    const state = emptyState({
      baseComponents: [cliComponent, proposedComponent],
      addedForces: [added],
    });
    const result = visibleComponents(state, {
      showProposed: true,
      showUnrelated: false,
      filteredForceIds: ["tmp-1"],
    });
    expect(result).toEqual(["web-view"]);
  });
});

describe("attractorOptions", () => {
  test("returns base attractors sorted by name", () => {
    const state = emptyState({ baseAttractors: [secondAttractor, baseAttractor] });
    expect(attractorOptions(state)).toEqual([
      { id: "A-02", name: "architecture-clarity" },
      { id: "A-01", name: "resilience" },
    ]);
  });

  test("merges in attractors added this session, sorted alongside base attractors", () => {
    const addedThisSession: SnapshotAttractor = {
      id: "A-03",
      name: "aardvark-of-attractors",
      description: "sorts first",
      positiveState: "x",
      negativeState: "y",
    };
    const state = emptyState({
      baseAttractors: [baseAttractor, secondAttractor],
      addedAttractors: [addedThisSession],
    });
    expect(attractorOptions(state)).toEqual([
      { id: "A-03", name: "aardvark-of-attractors" },
      { id: "A-02", name: "architecture-clarity" },
      { id: "A-01", name: "resilience" },
    ]);
  });

  test("returns an empty array for a state with no attractors at all", () => {
    expect(attractorOptions(emptyState())).toEqual([]);
  });
});

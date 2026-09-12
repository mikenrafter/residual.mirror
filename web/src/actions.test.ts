import { describe, expect, test } from "bun:test";
import {
  addAttractorOption,
  addComponentColumn,
  addForceRow,
  toggleComponent,
  updateForceField,
} from "./actions";
import type { AddedForce, PendingState, SnapshotAttractor, SnapshotComponent, SnapshotForce } from "./model";

const baseAttractor: SnapshotAttractor = {
  id: "A-01",
  name: "resilience",
  description: "system tolerates partial failure",
  positiveState: "graceful degradation",
  negativeState: "cascading failure",
};

const baseComponent: SnapshotComponent = {
  name: "cli",
  description: "command-line interface",
  status: "actual",
  architectureSet: "iter1",
};

const otherComponent: SnapshotComponent = {
  name: "storage",
  description: "persistence layer",
  status: "actual",
  architectureSet: "iter1",
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

const addedForce: AddedForce = {
  tempId: "NEW-1",
  kind: "purpose",
  description: "",
  attractorId: "",
  naiveChangeOrFeature: "",
  outcomes: "",
  shortname: "",
  components: ["cli"],
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

describe("toggleComponent", () => {
  test("toggling a component ON for a base force with no pending update seeds updatedForces from the base list", () => {
    const state = emptyState({
      baseComponents: [baseComponent, otherComponent],
      baseForces: [baseStressor],
    });
    const original = structuredClone(state);

    const next = toggleComponent(state, "S-01", "storage");

    expect(next.updatedForces["S-01"]?.components).toEqual(["cli", "storage"]);
    expect(state).toEqual(original);
  });

  test("toggling a component OFF for a base force that restores the original list prunes the updatedForces entry entirely", () => {
    const state = emptyState({
      baseComponents: [baseComponent, otherComponent],
      baseForces: [baseStressor],
      updatedForces: { "S-01": { components: ["cli", "storage"] } },
    });
    const original = structuredClone(state);

    const next = toggleComponent(state, "S-01", "storage");

    expect(next.updatedForces["S-01"]).toBeUndefined();
    expect(state).toEqual(original);
  });

  test("toggling OFF a restoring component preserves other already-set fields on the update entry, pruning only components", () => {
    const state = emptyState({
      baseComponents: [baseComponent, otherComponent],
      baseForces: [baseStressor],
      updatedForces: {
        "S-01": { components: ["cli", "storage"], description: "edited description" },
      },
    });

    const next = toggleComponent(state, "S-01", "storage");

    expect(next.updatedForces["S-01"]).toEqual({ description: "edited description" });
  });

  test("toggling a component off a base force's original list (never previously toggled) adds a removal update", () => {
    const state = emptyState({
      baseComponents: [baseComponent, otherComponent],
      baseForces: [baseStressor],
    });

    const next = toggleComponent(state, "S-01", "cli");

    expect(next.updatedForces["S-01"]?.components).toEqual([]);
  });

  test("toggling a component for an added force mutates addedForces[i].components directly, not updatedForces", () => {
    const state = emptyState({
      baseComponents: [baseComponent, otherComponent],
      addedForces: [addedForce],
    });
    const original = structuredClone(state);

    const next = toggleComponent(state, "NEW-1", "storage");

    expect(next.addedForces[0]?.components).toEqual(["cli", "storage"]);
    expect(next.updatedForces).toEqual({});
    expect(state).toEqual(original);
  });

  test("toggling an added force's existing component off removes it from addedForces[i].components", () => {
    const state = emptyState({
      baseComponents: [baseComponent],
      addedForces: [addedForce],
    });

    const next = toggleComponent(state, "NEW-1", "cli");

    expect(next.addedForces[0]?.components).toEqual([]);
  });

  test("does not mutate the original addedForces array or force object", () => {
    const state = emptyState({
      baseComponents: [baseComponent, otherComponent],
      addedForces: [addedForce],
    });

    toggleComponent(state, "NEW-1", "storage");

    expect(state.addedForces[0]?.components).toEqual(["cli"]);
  });
});

describe("addForceRow", () => {
  test("appends a new AddedForce with empty text fields, empty components, and the given kind", () => {
    const state = emptyState();
    const original = structuredClone(state);

    const { state: next, tempId } = addForceRow(state, "stressor");

    expect(next.addedForces).toHaveLength(1);
    const row = next.addedForces[0]!;
    expect(row.tempId).toBe(tempId);
    expect(row.kind).toBe("stressor");
    expect(row.description).toBe("");
    expect(row.attractorId).toBe("");
    expect(row.naiveChangeOrFeature).toBe("");
    expect(row.outcomes).toBe("");
    expect(row.shortname).toBe("");
    expect(row.components).toEqual([]);
    expect(state).toEqual(original);
  });

  test("two sequential calls generate distinct tempIds", () => {
    const state = emptyState();

    const first = addForceRow(state, "stressor");
    const second = addForceRow(first.state, "purpose");

    expect(first.tempId).not.toBe(second.tempId);
    expect(second.state.addedForces.map((f) => f.tempId)).toContain(first.tempId);
    expect(second.state.addedForces.map((f) => f.tempId)).toContain(second.tempId);
  });

  test("generates a tempId that does not collide with an already-existing addedForces tempId", () => {
    const state = emptyState({ addedForces: [addedForce] }); // tempId "NEW-1" already taken

    const { tempId } = addForceRow(state, "purpose");

    expect(tempId).not.toBe("NEW-1");
  });
});

describe("updateForceField", () => {
  test("sets a field on a base force, creating the updatedForces entry if absent", () => {
    const state = emptyState({ baseForces: [baseStressor] });
    const original = structuredClone(state);

    const next = updateForceField(state, "S-01", "description", "revised description");

    expect(next.updatedForces["S-01"]?.description).toBe("revised description");
    expect(state).toEqual(original);
  });

  test("a second updateForceField call on a base force accumulates into the same updatedForces entry", () => {
    const state = emptyState({ baseForces: [baseStressor] });

    const afterFirst = updateForceField(state, "S-01", "description", "revised description");
    const afterSecond = updateForceField(afterFirst, "S-01", "outcomes", "new outcome | matches lexicon");

    expect(afterSecond.updatedForces["S-01"]).toEqual({
      description: "revised description",
      outcomes: "new outcome | matches lexicon",
    });
  });

  test("sets a field directly on the matching addedForces entry without touching updatedForces", () => {
    const state = emptyState({ addedForces: [addedForce] });
    const original = structuredClone(state);

    const next = updateForceField(state, "NEW-1", "shortname", "new-shortname");

    expect(next.addedForces[0]?.shortname).toBe("new-shortname");
    expect(next.updatedForces).toEqual({});
    expect(state).toEqual(original);
  });

  test("two sequential updateForceField calls on an added force accumulate both fields", () => {
    const state = emptyState({ addedForces: [addedForce] });

    const afterFirst = updateForceField(state, "NEW-1", "description", "first field");
    const afterSecond = updateForceField(afterFirst, "NEW-1", "attractorId", "A-01");

    const row = afterSecond.addedForces[0]!;
    expect(row.description).toBe("first field");
    expect(row.attractorId).toBe("A-01");
  });

  test("does not mutate the original addedForces array or force object", () => {
    const state = emptyState({ addedForces: [addedForce] });

    updateForceField(state, "NEW-1", "description", "changed");

    expect(state.addedForces[0]?.description).toBe("");
  });
});

describe("addComponentColumn", () => {
  const newComponent: SnapshotComponent = {
    name: "queue",
    description: "message broker",
    status: "proposed",
    architectureSet: "iter2",
  };

  test("appends the new component to addedComponents", () => {
    const state = emptyState({ baseComponents: [baseComponent] });
    const original = structuredClone(state);

    const next = addComponentColumn(state, newComponent);

    expect(next.addedComponents).toEqual([newComponent]);
    expect(state).toEqual(original);
  });

  test("throws when the component name already exists in baseComponents", () => {
    const state = emptyState({ baseComponents: [baseComponent] });

    expect(() => addComponentColumn(state, { ...newComponent, name: "cli" })).toThrow();
  });

  test("throws when the component name already exists in addedComponents", () => {
    const state = emptyState({ addedComponents: [newComponent] });

    expect(() => addComponentColumn(state, newComponent)).toThrow();
  });
});

describe("addAttractorOption", () => {
  const newAttractor: SnapshotAttractor = {
    id: "A-02",
    name: "adaptability",
    description: "system absorbs novel stressors",
    positiveState: "flexible response",
    negativeState: "brittle failure",
  };

  test("appends the new attractor to addedAttractors", () => {
    const state = emptyState({ baseAttractors: [baseAttractor] });
    const original = structuredClone(state);

    const next = addAttractorOption(state, newAttractor);

    expect(next.addedAttractors).toEqual([newAttractor]);
    expect(state).toEqual(original);
  });

  test("throws when the attractor id already exists in baseAttractors", () => {
    const state = emptyState({ baseAttractors: [baseAttractor] });

    expect(() => addAttractorOption(state, { ...newAttractor, id: "A-01" })).toThrow();
  });

  test("throws when the attractor id already exists in addedAttractors", () => {
    const state = emptyState({ addedAttractors: [newAttractor] });

    expect(() => addAttractorOption(state, newAttractor)).toThrow();
  });
});

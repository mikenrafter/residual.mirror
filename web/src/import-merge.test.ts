import { describe, expect, test } from "bun:test";
import { mergeImportedItems } from "./import-merge";
import type { PendingItem } from "./import-parser";
import type { PendingState, SnapshotAttractor, SnapshotComponent, SnapshotForce } from "./model";

// ---------------------------------------------------------------------------
// Fixtures — mirrors model.test.ts / actions.test.ts / forms.test.ts's
// shared shapes so this file reads consistently with its siblings.
// ---------------------------------------------------------------------------

const baseAttractor: SnapshotAttractor = {
  id: "A-01",
  name: "resilience",
  description: "system tolerates partial failure",
  positiveState: "graceful degradation",
  negativeState: "cascading failure",
};

const baseComponentCli: SnapshotComponent = {
  name: "cli",
  description: "command-line interface",
  status: "actual",
  architectureSet: "iter1",
};

const baseComponentStorage: SnapshotComponent = {
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

function emptyState(overrides: Partial<PendingState> = {}): PendingState {
  return {
    baseAttractors: [baseAttractor],
    baseComponents: [baseComponentCli, baseComponentStorage],
    baseForces: [baseStressor],
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

function item(overrides: Partial<PendingItem> & Pick<PendingItem, "kind" | "type">): PendingItem {
  return {
    fields: {},
    multipleFields: {},
    raw: `residual ${overrides.kind} ${overrides.type}`,
    ...overrides,
  };
}

describe("mergeImportedItems — add stressor/purpose", () => {
  test("adds a new stressor with all mapped fields set", () => {
    const state = emptyState();
    const result = mergeImportedItems(state, [
      item({
        kind: "add",
        type: "stressor",
        fields: {
          description: "cache stampede under load",
          "attractor-id": "A-01",
          "naive-change": "add jitter",
          outcomes: "cache serves stale gracefully",
          shortname: "cache-stampede",
        },
      }),
    ]);

    expect(result.unmergeable).toEqual([]);
    expect(result.state.addedForces).toHaveLength(1);
    const added = result.state.addedForces[0]!;
    expect(added.kind).toBe("stressor");
    expect(added.description).toBe("cache stampede under load");
    expect(added.attractorId).toBe("A-01");
    expect(added.naiveChangeOrFeature).toBe("add jitter");
    expect(added.outcomes).toBe("cache serves stale gracefully");
    expect(added.shortname).toBe("cache-stampede");
    expect(added.components).toEqual([]);
  });

  test("adds a new purpose with all mapped fields set", () => {
    const state = emptyState();
    const result = mergeImportedItems(state, [
      item({
        kind: "add",
        type: "purpose",
        fields: {
          description: "operators trust the dashboard",
          "attractor-id": "A-01",
          "naive-change": "add a status page",
        },
      }),
    ]);

    expect(result.unmergeable).toEqual([]);
    expect(result.state.addedForces).toHaveLength(1);
    const added = result.state.addedForces[0]!;
    expect(added.kind).toBe("purpose");
    expect(added.description).toBe("operators trust the dashboard");
    expect(added.naiveChangeOrFeature).toBe("add a status page");
  });

  test("preserves existing pending adds — merge is additive, not replacing", () => {
    const state = emptyState({
      addedForces: [
        {
          tempId: "NEW-1",
          kind: "stressor",
          description: "existing pending stressor",
          attractorId: "A-01",
          naiveChangeOrFeature: "x",
          outcomes: "",
          shortname: "",
          components: [],
        },
      ],
    });

    const result = mergeImportedItems(state, [
      item({
        kind: "add",
        type: "stressor",
        fields: { description: "new one", "attractor-id": "A-01", "naive-change": "y" },
      }),
    ]);

    expect(result.state.addedForces).toHaveLength(2);
    expect(result.state.addedForces[0]!.description).toBe("existing pending stressor");
    expect(result.state.addedForces[1]!.description).toBe("new one");
  });
});

describe("mergeImportedItems — add component", () => {
  test("adds a new component column", () => {
    const state = emptyState();
    const result = mergeImportedItems(state, [
      item({
        kind: "add",
        type: "component",
        fields: {
          name: "queue",
          description: "message queue",
          status: "proposed",
          "architecture-set": "iter2",
        },
      }),
    ]);

    expect(result.unmergeable).toEqual([]);
    expect(result.state.addedComponents).toEqual([
      { name: "queue", description: "message queue", status: "proposed", architectureSet: "iter2" },
    ]);
  });
});

describe("mergeImportedItems — add attractor", () => {
  test("adds a new attractor with a synthetic NEW-ATTR-1 id when none pending yet", () => {
    const state = emptyState();
    const result = mergeImportedItems(state, [
      item({
        kind: "add",
        type: "attractor",
        fields: {
          name: "trust",
          description: "operators believe the system",
          "positive-state": "confidence",
          "negative-state": "suspicion",
        },
      }),
    ]);

    expect(result.unmergeable).toEqual([]);
    expect(result.state.addedAttractors).toEqual([
      {
        id: "NEW-ATTR-1",
        name: "trust",
        description: "operators believe the system",
        positiveState: "confidence",
        negativeState: "suspicion",
      },
    ]);
  });

  test("continues the synthetic id sequence past existing pending attractors", () => {
    const state = emptyState({
      addedAttractors: [
        {
          id: "NEW-ATTR-1",
          name: "existing",
          description: "d",
          positiveState: "p",
          negativeState: "n",
        },
      ],
    });

    const result = mergeImportedItems(state, [
      item({
        kind: "add",
        type: "attractor",
        fields: { name: "second", description: "d2", "positive-state": "p2", "negative-state": "n2" },
      }),
    ]);

    expect(result.state.addedAttractors).toHaveLength(2);
    expect(result.state.addedAttractors[1]!.id).toBe("NEW-ATTR-2");
  });
});

describe("mergeImportedItems — update stressor/purpose", () => {
  test("updates text fields on an existing base force by force-id", () => {
    const state = emptyState();
    const result = mergeImportedItems(state, [
      item({
        kind: "update",
        type: "stressor",
        fields: { "force-id": "S-01", description: "queue backs up worse than before" },
      }),
    ]);

    expect(result.unmergeable).toEqual([]);
    expect(result.state.updatedForces["S-01"]).toEqual({ description: "queue backs up worse than before" });
  });

  test("add-component toggles ON a component not yet coupled", () => {
    const state = emptyState();
    const result = mergeImportedItems(state, [
      item({
        kind: "update",
        type: "stressor",
        fields: { "force-id": "S-01" },
        multipleFields: { "add-component": ["storage"] },
      }),
    ]);

    expect(result.state.updatedForces["S-01"]?.components).toEqual(["cli", "storage"]);
  });

  test("add-component on an already-coupled component is a no-op (does not toggle it off)", () => {
    const state = emptyState();
    const result = mergeImportedItems(state, [
      item({
        kind: "update",
        type: "stressor",
        fields: { "force-id": "S-01" },
        multipleFields: { "add-component": ["cli"] },
      }),
    ]);

    // baseStressor already has "cli" coupled — a naive unconditional toggle
    // would flip it OFF, which is the opposite of the imported line's intent.
    expect(result.state.updatedForces["S-01"]).toBeUndefined();
  });

  test("remove-component on a not-currently-coupled component is a no-op (does not toggle it on)", () => {
    const state = emptyState();
    const result = mergeImportedItems(state, [
      item({
        kind: "update",
        type: "stressor",
        fields: { "force-id": "S-01" },
        multipleFields: { "remove-component": ["storage"] },
      }),
    ]);

    expect(result.state.updatedForces["S-01"]).toBeUndefined();
  });

  test("remove-component on a currently-coupled component toggles it off", () => {
    const state = emptyState();
    const result = mergeImportedItems(state, [
      item({
        kind: "update",
        type: "stressor",
        fields: { "force-id": "S-01" },
        multipleFields: { "remove-component": ["cli"] },
      }),
    ]);

    expect(result.state.updatedForces["S-01"]?.components).toEqual([]);
  });
});

describe("mergeImportedItems — unmergeable items", () => {
  test("an update to an attractor has no reducer support yet and is reported as unmergeable, not applied or crashed on", () => {
    const state = emptyState();
    const attractorUpdateItem = item({
      kind: "update",
      type: "attractor",
      fields: { id: "A-01", name: "renamed" },
    });

    const result = mergeImportedItems(state, [attractorUpdateItem]);

    expect(result.unmergeable).toEqual([attractorUpdateItem]);
    expect(result.state).toEqual(state);
  });

  test("mergeable items still apply even when a later item in the same batch is unmergeable", () => {
    const state = emptyState();
    const attractorUpdateItem = item({ kind: "update", type: "attractor", fields: { id: "A-01" } });

    const result = mergeImportedItems(state, [
      item({
        kind: "add",
        type: "component",
        fields: { name: "queue", description: "d", status: "actual", "architecture-set": "iter2" },
      }),
      attractorUpdateItem,
    ]);

    expect(result.state.addedComponents).toHaveLength(1);
    expect(result.unmergeable).toEqual([attractorUpdateItem]);
  });
});

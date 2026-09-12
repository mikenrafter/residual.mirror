import { describe, expect, test } from "bun:test";
import { snapshotToPendingState, type RawLandscapeSnapshot } from "./snapshot-bridge";

const emptyRaw: RawLandscapeSnapshot = {
  attractors: [],
  stressors: [],
  purposes: [],
  components: [],
  residues: [],
};

describe("snapshotToPendingState", () => {
  test("maps a single attractor field-by-field to camelCase", () => {
    const raw: RawLandscapeSnapshot = {
      ...emptyRaw,
      attractors: [
        {
          id: "A-01",
          name: "resilience",
          description: "system tolerates partial failure",
          positive_state: "graceful degradation",
          negative_state: "cascading failure",
          source: "sidecar",
        },
      ],
    };

    const state = snapshotToPendingState(raw);

    expect(state.baseAttractors).toEqual([
      {
        id: "A-01",
        name: "resilience",
        description: "system tolerates partial failure",
        positiveState: "graceful degradation",
        negativeState: "cascading failure",
      },
    ]);
  });

  test("maps a single component field-by-field to camelCase", () => {
    const raw: RawLandscapeSnapshot = {
      ...emptyRaw,
      components: [
        {
          name: "cli",
          description: "command-line interface",
          status: "actual",
          architecture_set: "iter1",
          source: "working",
        },
      ],
    };

    const state = snapshotToPendingState(raw);

    expect(state.baseComponents).toEqual([
      {
        name: "cli",
        description: "command-line interface",
        status: "actual",
        architectureSet: "iter1",
      },
    ]);
  });

  test("maps a stressor from raw.stressors to kind 'stressor' with renamed fields", () => {
    const raw: RawLandscapeSnapshot = {
      ...emptyRaw,
      stressors: [
        {
          id: "S-01",
          shortname: "queue-overload",
          description: "queue backs up under load",
          naive_change: "add retry",
          outcomes: "requests drain",
          attractor_id: "A-01",
          kind: "stressor",
          source: "sidecar",
        },
      ],
    };

    const state = snapshotToPendingState(raw);

    expect(state.baseForces).toHaveLength(1);
    expect(state.baseForces[0]).toEqual({
      id: "S-01",
      kind: "stressor",
      description: "queue backs up under load",
      attractorId: "A-01",
      naiveChangeOrFeature: "add retry",
      outcomes: "requests drain",
      shortname: "queue-overload",
      components: [],
    });
  });

  test("maps a purpose from raw.purposes to kind 'purpose' regardless of its own raw kind field", () => {
    const raw: RawLandscapeSnapshot = {
      ...emptyRaw,
      purposes: [
        {
          id: "P-01",
          shortname: "fast-checkout",
          description: "checkout completes quickly",
          naive_change: "cache cart",
          outcomes: "checkout completes",
          attractor_id: "A-01",
          kind: "purpose",
          source: "sidecar",
        },
      ],
    };

    const state = snapshotToPendingState(raw);

    expect(state.baseForces).toHaveLength(1);
    expect(state.baseForces[0].kind).toBe("purpose");
    expect(state.baseForces[0].id).toBe("P-01");
  });

  test("computes a force's components from coupled residues, excluding uncoupled and other-force rows", () => {
    const raw: RawLandscapeSnapshot = {
      ...emptyRaw,
      stressors: [
        {
          id: "S-01",
          shortname: "queue-overload",
          description: "queue backs up under load",
          naive_change: "add retry",
          outcomes: "requests drain",
          attractor_id: "A-01",
          kind: "stressor",
          source: "sidecar",
        },
      ],
      purposes: [
        {
          id: "P-01",
          shortname: "fast-checkout",
          description: "checkout completes quickly",
          naive_change: "cache cart",
          outcomes: "checkout completes",
          attractor_id: "A-01",
          kind: "purpose",
          source: "sidecar",
        },
      ],
      residues: [
        // S-01 coupled to two components -> both included.
        {
          force_id: "S-01",
          component_id: "cli",
          coupled: true,
          whole_system: false,
          notes: "",
          source: "sidecar",
        },
        {
          force_id: "S-01",
          component_id: "db",
          coupled: true,
          whole_system: false,
          notes: "",
          source: "sidecar",
        },
        // S-01 x auth is NOT coupled -> excluded.
        {
          force_id: "S-01",
          component_id: "auth",
          coupled: false,
          whole_system: false,
          notes: "",
          source: "sidecar",
        },
        // Residue for a different force (P-01) -> must not leak into S-01's components.
        {
          force_id: "P-01",
          component_id: "web",
          coupled: true,
          whole_system: false,
          notes: "",
          source: "sidecar",
        },
      ],
    };

    const state = snapshotToPendingState(raw);

    const stressor = state.baseForces.find((f) => f.id === "S-01");
    expect(stressor?.components.sort()).toEqual(["cli", "db"]);

    const purpose = state.baseForces.find((f) => f.id === "P-01");
    expect(purpose?.components).toEqual(["web"]);
  });

  test("orders baseForces as mapped stressors first, then mapped purposes", () => {
    const raw: RawLandscapeSnapshot = {
      ...emptyRaw,
      stressors: [
        {
          id: "S-01",
          shortname: "s1",
          description: "d",
          naive_change: "n",
          outcomes: "o",
          attractor_id: "A-01",
          kind: "stressor",
          source: "sidecar",
        },
        {
          id: "S-02",
          shortname: "s2",
          description: "d",
          naive_change: "n",
          outcomes: "o",
          attractor_id: "A-01",
          kind: "stressor",
          source: "sidecar",
        },
      ],
      purposes: [
        {
          id: "P-01",
          shortname: "p1",
          description: "d",
          naive_change: "n",
          outcomes: "o",
          attractor_id: "A-01",
          kind: "purpose",
          source: "sidecar",
        },
      ],
    };

    const state = snapshotToPendingState(raw);

    expect(state.baseForces.map((f) => f.id)).toEqual(["S-01", "S-02", "P-01"]);
  });

  test("all added* collections start empty and updated* records start empty", () => {
    const state = snapshotToPendingState(emptyRaw);

    expect(state.addedAttractors).toEqual([]);
    expect(state.addedComponents).toEqual([]);
    expect(state.addedForces).toEqual([]);
    expect(state.addedPersonas).toEqual([]);
    expect(state.addedTerms).toEqual([]);

    expect(state.updatedAttractors).toEqual({});
    expect(state.updatedComponents).toEqual({});
    expect(state.updatedForces).toEqual({});
    expect(state.updatedPersonas).toEqual({});
    expect(state.updatedTerms).toEqual({});
  });

  test("baseTerms and basePersonas start empty since raw snapshot carries no term/persona data", () => {
    const state = snapshotToPendingState(emptyRaw);

    expect(state.baseTerms).toEqual([]);
    expect(state.basePersonas).toEqual([]);
  });

  test("an empty raw snapshot yields a valid, fully-empty PendingState with no crash", () => {
    const state = snapshotToPendingState(emptyRaw);

    expect(state.baseAttractors).toEqual([]);
    expect(state.baseComponents).toEqual([]);
    expect(state.baseForces).toEqual([]);
    expect(state.basePersonas).toEqual([]);
    expect(state.baseTerms).toEqual([]);
    expect(state.addedAttractors).toEqual([]);
    expect(state.addedComponents).toEqual([]);
    expect(state.addedForces).toEqual([]);
    expect(state.addedPersonas).toEqual([]);
    expect(state.addedTerms).toEqual([]);
    expect(state.updatedAttractors).toEqual({});
    expect(state.updatedComponents).toEqual({});
    expect(state.updatedForces).toEqual({});
    expect(state.updatedPersonas).toEqual({});
    expect(state.updatedTerms).toEqual({});
  });
});

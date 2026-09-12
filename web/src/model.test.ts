import { describe, expect, test } from "bun:test";
import { parseImportText } from "./import-parser";
import {
  computeStateValidity,
  isForceValid,
  orderedEntries,
  toCommandLines,
  type AddedForce,
  type PendingState,
  type SnapshotAttractor,
  type SnapshotComponent,
  type SnapshotForce,
} from "./model";

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

describe("isForceValid", () => {
  test("valid when all required fields are set and at least one component is toggled", () => {
    expect(
      isForceValid({
        description: "queue backs up under load",
        attractorId: "A-01",
        naiveChangeOrFeature: "add retry",
        components: ["cli"],
      }),
    ).toBe(true);
  });

  test("invalid when description is empty", () => {
    expect(
      isForceValid({
        description: "",
        attractorId: "A-01",
        naiveChangeOrFeature: "add retry",
        components: ["cli"],
      }),
    ).toBe(false);
  });

  test("invalid when attractor-id is empty", () => {
    expect(
      isForceValid({
        description: "d",
        attractorId: "",
        naiveChangeOrFeature: "add retry",
        components: ["cli"],
      }),
    ).toBe(false);
  });

  test("invalid when naive-change/feature is empty", () => {
    expect(
      isForceValid({
        description: "d",
        attractorId: "A-01",
        naiveChangeOrFeature: "",
        components: ["cli"],
      }),
    ).toBe(false);
  });

  test("invalid when zero components are toggled, even with all other fields set", () => {
    expect(
      isForceValid({
        description: "d",
        attractorId: "A-01",
        naiveChangeOrFeature: "add retry",
        components: [],
      }),
    ).toBe(false);
  });

  test("empty outcomes does NOT invalidate — outcomes is required:false in cli-schema.json", () => {
    // isForceValid's parameter type has no `outcomes` field at all: it is
    // never part of the check, by design.
    expect(
      isForceValid({
        description: "d",
        attractorId: "A-01",
        naiveChangeOrFeature: "add retry",
        components: ["cli"],
      }),
    ).toBe(true);
  });
});

describe("computeStateValidity", () => {
  test("a base force whose pending update blanks a required field becomes invalid", () => {
    const state = emptyState({
      baseAttractors: [baseAttractor],
      baseForces: [baseStressor],
      updatedForces: { "S-01": { description: "" } },
    });

    const validity = computeStateValidity(state);

    expect(validity.invalidForceIds.has("S-01")).toBe(true);
    expect(validity.invalidReasonsByForce["S-01"]).toContain("description");
  });

  test("an added force missing attractor-id is invalid, keyed by its tempId", () => {
    const addedForce: AddedForce = {
      tempId: "NEW-1",
      id: "",
      kind: "stressor",
      description: "d",
      attractorId: "",
      naiveChangeOrFeature: "n",
      outcomes: "",
      shortname: "",
      components: ["cli"],
    };
    const state = emptyState({ addedForces: [addedForce] });

    const validity = computeStateValidity(state);

    expect(validity.invalidForceIds.has("NEW-1")).toBe(true);
    expect(validity.invalidReasonsByForce["NEW-1"]).toContain("attractorId");
  });

  test("a fully valid base force with no pending update is not flagged invalid", () => {
    const state = emptyState({
      baseAttractors: [baseAttractor],
      baseForces: [baseStressor],
    });

    const validity = computeStateValidity(state);

    expect(validity.invalidForceIds.has("S-01")).toBe(false);
  });
});

describe("orderedEntries", () => {
  test("orders additions (component, attractor, persona, term, force) before all updates in the same bucket order", () => {
    const addedAttractor: SnapshotAttractor = {
      id: "",
      name: "new-attractor",
      description: "d",
      positiveState: "p",
      negativeState: "n",
    };
    const addedComponent: SnapshotComponent = {
      name: "gateway",
      description: "d",
      status: "proposed",
      architectureSet: "iter1",
    };
    const addedForce: AddedForce = {
      tempId: "NEW-1",
      id: "",
      kind: "stressor",
      description: "d",
      attractorId: "A-01",
      naiveChangeOrFeature: "n",
      outcomes: "",
      shortname: "",
      components: [], // no components: no synthetic update entry (see dedicated test below)
    };

    const state = emptyState({
      baseAttractors: [baseAttractor],
      baseForces: [baseStressor],
      addedAttractors: [addedAttractor],
      addedComponents: [addedComponent],
      addedForces: [addedForce],
      updatedAttractors: { "A-01": { description: "revised" } },
      updatedForces: { "S-01": { description: "revised" } },
    });

    expect(orderedEntries(state)).toEqual([
      { bucket: "component", action: "add", key: "gateway" },
      { bucket: "attractor", action: "add", key: "new-attractor" },
      { bucket: "force", action: "add", key: "NEW-1" },
      { bucket: "attractor", action: "update", key: "A-01" },
      { bucket: "force", action: "update", key: "S-01" },
    ]);
  });

  test("an added force with toggled components emits an add entry AND a synthetic update entry, both keyed by its tempId", () => {
    const addedForce: AddedForce = {
      tempId: "NEW-1",
      id: "",
      kind: "stressor",
      description: "d",
      attractorId: "A-01",
      naiveChangeOrFeature: "n",
      outcomes: "",
      shortname: "",
      components: ["cli"],
    };
    const state = emptyState({ addedForces: [addedForce] });

    expect(orderedEntries(state)).toEqual([
      { bucket: "force", action: "add", key: "NEW-1" },
      { bucket: "force", action: "update", key: "NEW-1" },
    ]);
  });

  test("terms are ordered last in both the add phase and the update phase", () => {
    const addedTerm = { term: "hyperliminal coupling", definition: "d" };
    const addedComponent: SnapshotComponent = {
      name: "gateway",
      description: "d",
      status: "proposed",
      architectureSet: "iter1",
    };
    const state = emptyState({
      addedComponents: [addedComponent],
      addedTerms: [addedTerm],
      updatedTerms: { "hyperliminal coupling": { definition: "revised" } },
      updatedComponents: { cli: { description: "revised" } },
      baseComponents: [baseComponent],
    });

    const entries = orderedEntries(state);
    const addIndices = entries
      .map((e, i) => ({ e, i }))
      .filter(({ e }) => e.action === "add")
      .map(({ i }) => i);
    const updateIndices = entries
      .map((e, i) => ({ e, i }))
      .filter(({ e }) => e.action === "update")
      .map(({ i }) => i);

    expect(Math.max(...addIndices)).toBeLessThan(Math.min(...updateIndices));
    expect(entries[entries.findIndex((e) => e.bucket === "term" && e.action === "add")]).toEqual(
      { bucket: "term", action: "add", key: "hyperliminal coupling" },
    );
    expect(entries[entries.length - 1]).toEqual({
      bucket: "term",
      action: "update",
      key: "hyperliminal coupling",
    });
  });
});

describe("toCommandLines", () => {
  test("renders a valid added stressor as a quoted `residual add stressor` line marked valid:true", () => {
    // A component is toggled on, so per "no components selected is
    // invalid" this row IS a valid, complete matrix row — the add line
    // ends up wrapped in a shell-variable capture (see the dedicated
    // with-components test below for why), but it must still report
    // valid:true since the underlying force passes isForceValid.
    const addedForce: AddedForce = {
      tempId: "NEW-1",
      id: "",
      kind: "stressor",
      description: "queue backs up under load",
      attractorId: "A-01",
      naiveChangeOrFeature: "add retry",
      outcomes: "requests drain",
      shortname: "queue-overload",
      components: ["cli"],
    };
    const state = emptyState({ addedForces: [addedForce] });

    const lines = toCommandLines(state);
    const addLine = lines.find((l) => l.line.includes("residual add stressor"));

    expect(addLine).toBeDefined();
    expect(addLine!.valid).toBe(true);
    expect(addLine!.line).toContain('--description "queue backs up under load"');
    expect(addLine!.line).toContain('--attractor-id "A-01"');
    expect(addLine!.line).toContain('--naive-change "add retry"');
  });

  test("marks the line invalid when the underlying force fails validation", () => {
    const addedForce: AddedForce = {
      tempId: "NEW-1",
      id: "",
      kind: "stressor",
      description: "",
      attractorId: "A-01",
      naiveChangeOrFeature: "add retry",
      outcomes: "",
      shortname: "",
      components: ["cli"],
    };
    const state = emptyState({ addedForces: [addedForce] });

    const lines = toCommandLines(state);
    const addLine = lines.find((l) => l.line.includes("residual add stressor"));

    expect(addLine).toBeDefined();
    expect(addLine!.valid).toBe(false);
  });

  test("marks the add line invalid when zero components are toggled, even with every text field set — 'no components selected is invalid' applies to the whole row, including its add line", () => {
    const addedForce: AddedForce = {
      tempId: "NEW-1",
      id: "",
      kind: "stressor",
      description: "queue backs up under load",
      attractorId: "A-01",
      naiveChangeOrFeature: "add retry",
      outcomes: "requests drain",
      shortname: "queue-overload",
      components: [],
    };
    const state = emptyState({ addedForces: [addedForce] });

    const lines = toCommandLines(state);
    const addLine = lines.find((l) => l.line.includes("residual add stressor"));

    expect(addLine).toBeDefined();
    expect(addLine!.valid).toBe(false);
  });

  test("round-trips a generated `add stressor` line through parseImportText", () => {
    // No components toggled, so this stays a plain, directly-runnable
    // `residual add ...` line with no shell-variable capture wrapper —
    // see the with-components tests below for that case.
    const addedForce: AddedForce = {
      tempId: "NEW-1",
      id: "",
      kind: "stressor",
      description: 'text with "quotes" inside',
      attractorId: "A-01",
      naiveChangeOrFeature: "add retry",
      outcomes: "requests drain",
      shortname: "queue-overload",
      components: [],
    };
    const state = emptyState({ addedForces: [addedForce] });

    const addLine = toCommandLines(state).find((l) =>
      l.line.startsWith("residual add stressor"),
    )!;
    const parsed = parseImportText(addLine.line);

    expect(parsed.errors).toHaveLength(0);
    expect(parsed.items).toHaveLength(1);
    const item = parsed.items[0]!;
    expect(item.kind).toBe("add");
    expect(item.type).toBe("stressor");
    expect(item.fields.description).toBe('text with "quotes" inside');
    expect(item.fields["attractor-id"]).toBe("A-01");
    expect(item.fields["naive-change"]).toBe("add retry");
  });

  test("an added force's real id is not known until `add stressor` actually runs (it has no --force-id flag — id is server-assigned and only printed on success as 'Added stressor S-NN'), so a synthetic component-toggle update MUST NOT reference the tempId as a literal --force-id value — that string will never exist in residues.csv/stressors.csv and the generated script would silently fail", () => {
    const addedForce: AddedForce = {
      tempId: "NEW-1",
      id: "",
      kind: "stressor",
      description: "d",
      attractorId: "A-01",
      naiveChangeOrFeature: "n",
      outcomes: "",
      shortname: "",
      components: ["cli"],
    };
    const state = emptyState({ addedForces: [addedForce] });
    const lines = toCommandLines(state);

    const addLine = lines.find((l) => l.line.includes("residual add stressor"))!;
    const updateLine = lines.find((l) => l.line.includes("residual update stressor"))!;

    // The add line captures the real assigned id into a shell variable
    // derived from the tempId (sanitized: non-alphanumeric -> "_", suffixed
    // "_ID") instead of emitting a bare `residual add ...` invocation.
    expect(addLine.line.startsWith("NEW_1_ID=$(residual add stressor")).toBe(true);
    expect(addLine.line).toContain("residual add stressor");
    expect(addLine.line).toContain('--description "d"');
    // Whatever the exact capture mechanism, it must extract the id token
    // `residual add stressor` prints on success ("Added stressor S-NN").
    expect(addLine.line).toMatch(/\)\s*$/);

    // The update line references that shell variable, NOT the tempId.
    expect(updateLine.line).toBe(
      'residual update stressor --force-id "$NEW_1_ID" --add-component "cli"',
    );
    expect(updateLine.line).not.toContain("NEW-1");

    // This line is intentionally NOT parseImportText-round-trippable: it's
    // shell-variable-dependent, not a standalone `residual` invocation, so
    // import only needs to handle plain add/update lines (see the other
    // round-trip tests above/below), not this synthetic scripting form.
  });

  test("an updated base force's component diff emits --add-component for newly toggled-on components and --remove-component for toggled-off ones", () => {
    // baseStressor starts coupled only to "cli"; the update adds "gateway"
    // and drops "cli".
    const state = emptyState({
      baseAttractors: [baseAttractor],
      baseForces: [baseStressor],
      updatedForces: { "S-01": { components: ["gateway"] } },
    });

    const updateLine = toCommandLines(state).find((l) =>
      l.line.startsWith("residual update stressor"),
    )!;
    const parsed = parseImportText(updateLine.line);

    expect(parsed.errors).toHaveLength(0);
    const item = parsed.items[0]!;
    expect(item.fields["force-id"]).toBe("S-01");
    expect(item.multipleFields["add-component"]).toEqual(["gateway"]);
    expect(item.multipleFields["remove-component"]).toEqual(["cli"]);
  });

  test("round-trips a generated `add attractor` line through parseImportText", () => {
    const addedAttractor: SnapshotAttractor = {
      id: "",
      name: "new-attractor",
      description: "d",
      positiveState: "graceful",
      negativeState: "cascading",
    };
    const state = emptyState({ addedAttractors: [addedAttractor] });

    const addLine = toCommandLines(state).find((l) =>
      l.line.startsWith("residual add attractor"),
    )!;
    const parsed = parseImportText(addLine.line);

    expect(parsed.errors).toHaveLength(0);
    const item = parsed.items[0]!;
    expect(item.fields.name).toBe("new-attractor");
    expect(item.fields.description).toBe("d");
    expect(item.fields["positive-state"]).toBe("graceful");
    expect(item.fields["negative-state"]).toBe("cascading");
  });

  test("round-trips a generated `add component` line through parseImportText", () => {
    const addedComponent: SnapshotComponent = {
      name: "gateway",
      description: "edge api gateway",
      status: "proposed",
      architectureSet: "iter1",
    };
    const state = emptyState({ addedComponents: [addedComponent] });

    const addLine = toCommandLines(state).find((l) =>
      l.line.startsWith("residual add component"),
    )!;
    const parsed = parseImportText(addLine.line);

    expect(parsed.errors).toHaveLength(0);
    const item = parsed.items[0]!;
    expect(item.fields.name).toBe("gateway");
    expect(item.fields.status).toBe("proposed");
    expect(item.fields["architecture-set"]).toBe("iter1");
  });
});

// In-memory live-editing state layered on top of a loaded LandscapeSnapshot,
// plus validation and residual-CLI command-line generation.
//
// Pure, DOM-free, framework-free — see web/src/model.test.ts and the
// governing plan at residual/iterations/view-live-editing-plan.md (Phase 4).
// Sits between the embedded snapshot JSON (src/view/snapshot.rs's
// `LandscapeSnapshot`) and import-parser.ts's CLI-text parsing: this module
// owns "what has the user added/changed/toggled this session" plus
// validation, and turns that state back into `residual add`/`residual
// update` command-line text.
//
// Design notes (decisions made beyond the task sketch, documented per
// instructions):
//
// - `SnapshotAttractor` includes `description` even though the sketch in the
//   task omitted it: `residual add attractor --description` is a *required*
//   flag per web/generated/cli-schema.json ("add attractor" ->
//   description: required=true), and src/view/snapshot.rs's
//   `SnapshotAttractor` carries it too. Dropping it would make
//   `toCommandLines` unable to ever emit a valid `add attractor` line.
// - Required-fields ground truth for validation comes from
//   web/generated/cli-schema.json, NOT the plan prose. Per that schema,
//   `add stressor`/`add purpose` require description, attractor-shortname and
//   naive-change/feature — but NOT outcomes (outcomes: required=false in the
//   schema for both). `isForceValid` therefore does not check outcomes.
// - `add stressor`/`add purpose` have no `--add-component` flag in the
//   schema (only `update stressor`/`update purpose` do). Forces are
//   addressed everywhere — `add residue`, `update stressor`/`update
//   purpose`, `commit template` — by `--shortname`, which the CLI now
//   requires on `add stressor`/`add purpose` and which the caller (this UI)
//   chooses up front. So, unlike the S-nn/P-nn id (still server-assigned and
//   only ever visible in stdout), the shortname is known before the `add`
//   line even runs: a freshly *added* force's toggled components ride a
//   plain follow-up `update <type> --shortname ... --add-component ...`
//   line, with no shell-variable capture needed.
// - Personas and terms are tracked as their own add/update buckets even
//   though the plan's explicit ordering list only names "components,
//   attractors, personas, forces" (terms are not mentioned). Since terms
//   don't participate in the NKP matrix and aren't named in the ordering
//   rule, this module places them last, after forces, in both the add and
//   update phases — a conservative extension of the rule, not a
//   contradiction of it.

/** A landscape attractor, as it appears in the embedded snapshot or is added this session. */
export interface SnapshotAttractor {
  id: string;
  name: string;
  /** Required by `residual add attractor --description`; see module notes. */
  description: string;
  positiveState: string;
  negativeState: string;
}

/** A landscape component (NKP matrix column). */
export interface SnapshotComponent {
  name: string;
  description: string;
  status: "proposed" | "actual";
  architectureSet: string;
}

/** A stressor or purpose force (NKP matrix row), with its coupled components. */
export interface SnapshotForce {
  id: string;
  kind: "stressor" | "purpose";
  description: string;
  attractorId: string;
  /** `naive_change` for stressors, `feature` for purposes. */
  naiveChangeOrFeature: string;
  outcomes: string;
  shortname: string;
  /** Component names currently coupled to this force via residues.csv. */
  components: string[];
}

/** A defense/below-table persona (name + role), not part of the NKP matrix. */
export interface SnapshotPersona {
  name: string;
  role: string;
  concerns?: string;
  desires?: string;
}

/** A lexicon term, not part of the NKP matrix. */
export interface SnapshotTerm {
  term: string;
  definition: string;
  domain?: string;
  related?: string;
}

/** A newly-added force this session, not yet assigned a real force id by the CLI. */
export type AddedForce = SnapshotForce & { tempId: string };

/** Partial update to an existing (or newly-added) force, keyed by real id or tempId. */
export type ForceUpdate = Partial<Omit<SnapshotForce, "id" | "kind">>;

/** Partial update to an existing attractor, keyed by real id. */
export type AttractorUpdate = Partial<Omit<SnapshotAttractor, "id">>;

/** Partial update to an existing component, keyed by name (components have no separate id). */
export type ComponentUpdate = Partial<Omit<SnapshotComponent, "name">>;

/** Partial update to an existing persona, keyed by name. */
export type PersonaUpdate = Partial<Omit<SnapshotPersona, "name">>;

/** Partial update to an existing term, keyed by term text. */
export type TermUpdate = Partial<Omit<SnapshotTerm, "term">>;

/**
 * Everything the user has added or changed this editing session, layered on
 * top of a base snapshot loaded from the embedded `residual-snapshot` JSON.
 */
export interface PendingState {
  baseAttractors: SnapshotAttractor[];
  baseComponents: SnapshotComponent[];
  baseForces: SnapshotForce[];
  basePersonas: SnapshotPersona[];
  baseTerms: SnapshotTerm[];

  addedAttractors: SnapshotAttractor[];
  addedComponents: SnapshotComponent[];
  addedForces: AddedForce[];
  addedPersonas: SnapshotPersona[];
  addedTerms: SnapshotTerm[];

  /** Keyed by real attractor id. */
  updatedAttractors: Record<string, AttractorUpdate>;
  /** Keyed by component name. */
  updatedComponents: Record<string, ComponentUpdate>;
  /** Keyed by real force id OR an added force's tempId. */
  updatedForces: Record<string, ForceUpdate>;
  /** Keyed by persona name. */
  updatedPersonas: Record<string, PersonaUpdate>;
  /** Keyed by term text. */
  updatedTerms: Record<string, TermUpdate>;
  /** Existing force ids staged for removal, keyed by id with their CLI kind. */
  removedForces?: Record<string, "stressor" | "purpose">;
}

/** Required (non-outcomes) field names for a stressor/purpose, per cli-schema.json. */
export type RequiredForceField = "description" | "attractorId" | "naiveChangeOrFeature" | "shortname";

export const REQUIRED_FORCE_FIELDS: readonly RequiredForceField[] = [
  "description",
  "attractorId",
  "naiveChangeOrFeature",
  "shortname",
];

/** One resolved item in the fixed add/update, component/attractor/persona/term/force emission order. */
export interface OrderedEntry {
  bucket: "component" | "attractor" | "persona" | "term" | "force";
  action: "add" | "update";
  /** Real id/name/term/tempId identifying the entry within its bucket. */
  key: string;
}

/**
 * Returns whether a force (added, or the effective merge of a base force
 * with its pending update) is valid: every field in REQUIRED_FORCE_FIELDS is
 * non-empty, AND at least one component is toggled on (per the plan: "no
 * components selected is invalid"). Outcomes is intentionally not checked —
 * it is not required by `add stressor`/`add purpose` in cli-schema.json.
 */
export function isForceValid(force: {
  description: string;
  attractorId: string;
  naiveChangeOrFeature: string;
  shortname: string;
  components: string[];
}): boolean {
  for (const field of REQUIRED_FORCE_FIELDS) {
    if (force[field] === "") return false;
  }
  return force.components.length > 0;
}

/** Per-state validity: which forces (by id/tempId) are invalid, and why. */
export interface StateValidity {
  invalidForceIds: Set<string>;
  /** Reasons per invalid force id/tempId, e.g. ["description", "components"]. */
  invalidReasonsByForce: Record<string, string[]>;
}

/**
 * Computes validity across the whole current state: every base force merged
 * with its pending update (if any), plus every added force. Used later to
 * drive red-highlighting of invalid cells/rows/sticky headers (Phase 5).
 */
export function computeStateValidity(state: PendingState): StateValidity {
  const invalidForceIds = new Set<string>();
  const invalidReasonsByForce: Record<string, string[]> = {};

  const evaluate = (
    id: string,
    merged: {
      description: string;
      attractorId: string;
      naiveChangeOrFeature: string;
      shortname: string;
      components: string[];
    },
  ): void => {
    const reasons: string[] = [];
    for (const field of REQUIRED_FORCE_FIELDS) {
      if (merged[field] === "") reasons.push(field);
    }
    if (merged.components.length === 0) reasons.push("components");
    if (reasons.length > 0) {
      invalidForceIds.add(id);
      invalidReasonsByForce[id] = reasons;
    }
  };

  for (const base of state.baseForces) {
    if (state.removedForces?.[base.id] !== undefined) continue;
    const update = state.updatedForces[base.id];
    evaluate(base.id, {
      description: update?.description ?? base.description,
      attractorId: update?.attractorId ?? base.attractorId,
      naiveChangeOrFeature: update?.naiveChangeOrFeature ?? base.naiveChangeOrFeature,
      shortname: update?.shortname ?? base.shortname,
      components: update?.components ?? base.components,
    });
  }

  for (const added of state.addedForces) {
    const update = state.updatedForces[added.tempId];
    evaluate(added.tempId, {
      description: update?.description ?? added.description,
      attractorId: update?.attractorId ?? added.attractorId,
      naiveChangeOrFeature: update?.naiveChangeOrFeature ?? added.naiveChangeOrFeature,
      shortname: update?.shortname ?? added.shortname,
      components: update?.components ?? added.components,
    });
  }

  return { invalidForceIds, invalidReasonsByForce };
}

/**
 * Flattens a PendingState into the fixed emission order: components,
 * attractors, personas, terms, forces — with every addition across every
 * bucket ordered before every update across every bucket (see module notes
 * re: terms, and re: added forces with toggled components synthesizing a
 * follow-up "update" entry keyed by their tempId).
 */
export function orderedEntries(state: PendingState): OrderedEntry[] {
  const addedForceTempIds = new Set(state.addedForces.map((f) => f.tempId));

  const adds: OrderedEntry[] = [
    ...state.addedComponents.map((c): OrderedEntry => ({ bucket: "component", action: "add", key: c.name })),
    ...state.addedAttractors.map((a): OrderedEntry => ({ bucket: "attractor", action: "add", key: a.name })),
    ...state.addedPersonas.map((p): OrderedEntry => ({ bucket: "persona", action: "add", key: p.name })),
    ...state.addedTerms.map((t): OrderedEntry => ({ bucket: "term", action: "add", key: t.term })),
    ...state.addedForces.map((f): OrderedEntry => ({ bucket: "force", action: "add", key: f.tempId })),
  ];

  const updates: OrderedEntry[] = [
    ...Object.keys(state.updatedComponents).map(
      (k): OrderedEntry => ({ bucket: "component", action: "update", key: k }),
    ),
    ...Object.keys(state.updatedAttractors).map(
      (k): OrderedEntry => ({ bucket: "attractor", action: "update", key: k }),
    ),
    ...Object.keys(state.updatedPersonas).map(
      (k): OrderedEntry => ({ bucket: "persona", action: "update", key: k }),
    ),
    ...Object.keys(state.updatedTerms).map((k): OrderedEntry => ({ bucket: "term", action: "update", key: k })),
    // Synthetic follow-up updates for added forces whose toggled components
    // can't ride on their own `add` line (`add stressor`/`add purpose` have
    // no `--add-component` flag) — see module notes and toCommandLines.
    ...state.addedForces
      .filter((f) => f.components.length > 0)
      .map((f): OrderedEntry => ({ bucket: "force", action: "update", key: f.tempId })),
    // Real diffs against base forces (or, in principle, further edits to an
    // added force after its synthetic entry was already emitted above —
    // excluded here to avoid a second, ambiguous entry for the same tempId).
    ...Object.keys(state.updatedForces)
      .filter((k) => state.removedForces?.[k] === undefined)
      .filter((k) => !addedForceTempIds.has(k))
      .map((k): OrderedEntry => ({ bucket: "force", action: "update", key: k })),
  ];

  return [...adds, ...updates];
}

/** One rendered `residual add`/`residual update` command line, with its validity. */
export interface CommandLine {
  line: string;
  valid: boolean;
}

/**
 * Renders a PendingState into ordered `residual add`/`residual update`
 * command-line text (see orderedEntries for ordering), quoting every flag
 * value in double quotes and escaping embedded `"` as `\"`, mirroring
 * import-parser.ts's `tokenizeCommandLine` so lines round-trip through its
 * `parseImportText`. Lines belonging to an invalid force (see isForceValid)
 * are marked `valid: false`; the DOM layer (Phase 5) decides whether to
 * comment such lines out with `# `.
 */
/** Double-quotes a value for CLI text, escaping embedded `"` as `\"` — mirrors import-parser.ts's `tokenizeCommandLine` unescaping. */
function quoteValue(value: string): string {
  return `"${value.replace(/"/g, '\\"')}"`;
}

/** Renders `--flag "value"`. */
function flagText(name: string, value: string): string {
  return `--${name} ${quoteValue(value)}`;
}

export function toCommandLines(state: PendingState): CommandLine[] {
  const validity = computeStateValidity(state);
  const entries = orderedEntries(state);

  const addedComponentByName = new Map(state.addedComponents.map((c) => [c.name, c]));
  const addedAttractorByName = new Map(state.addedAttractors.map((a) => [a.name, a]));
  const addedPersonaByName = new Map(state.addedPersonas.map((p) => [p.name, p]));
  const addedTermByTerm = new Map(state.addedTerms.map((t) => [t.term, t]));
  const addedForceByTempId = new Map(state.addedForces.map((f) => [f.tempId, f]));
  const baseForceById = new Map(state.baseForces.map((f) => [f.id, f]));
  const attractorShortname = (id: string): string =>
    [...state.baseAttractors, ...state.addedAttractors].find((attractor) => attractor.id === id)?.name ?? id;

  const forceIsValid = (key: string): boolean => !validity.invalidForceIds.has(key);

  const renderComponentAdd = (name: string): CommandLine => {
    const c = addedComponentByName.get(name)!;
    const line = [
      "residual add component",
      flagText("architecture-set", c.architectureSet),
      flagText("description", c.description),
      flagText("name", c.name),
      flagText("status", c.status),
    ].join(" ");
    return { line, valid: true };
  };

  const renderComponentUpdate = (name: string): CommandLine => {
    const update = state.updatedComponents[name]!;
    const parts = ["residual update component", flagText("name", name)];
    if (update.architectureSet !== undefined) parts.push(flagText("architecture-set", update.architectureSet));
    if (update.description !== undefined) parts.push(flagText("description", update.description));
    if (update.status !== undefined) parts.push(flagText("status", update.status));
    return { line: parts.join(" "), valid: true };
  };

  const renderAttractorAdd = (name: string): CommandLine => {
    const a = addedAttractorByName.get(name)!;
    const line = [
      "residual add attractor",
      flagText("description", a.description),
      flagText("name", a.name),
      flagText("negative-state", a.negativeState),
      flagText("positive-state", a.positiveState),
    ].join(" ");
    return { line, valid: true };
  };

  const renderAttractorUpdate = (id: string): CommandLine => {
    const update = state.updatedAttractors[id]!;
    const parts = ["residual update attractor", flagText("id", id)];
    if (update.description !== undefined) parts.push(flagText("description", update.description));
    if (update.name !== undefined) parts.push(flagText("name", update.name));
    if (update.negativeState !== undefined) parts.push(flagText("negative-state", update.negativeState));
    if (update.positiveState !== undefined) parts.push(flagText("positive-state", update.positiveState));
    return { line: parts.join(" "), valid: true };
  };

  const renderPersonaAdd = (name: string): CommandLine => {
    const p = addedPersonaByName.get(name)!;
    const parts = ["residual add persona"];
    if (p.concerns !== undefined) parts.push(flagText("concerns", p.concerns));
    if (p.desires !== undefined) parts.push(flagText("desires", p.desires));
    parts.push(flagText("name", p.name));
    parts.push(flagText("role", p.role));
    return { line: parts.join(" "), valid: true };
  };

  const renderPersonaUpdate = (name: string): CommandLine => {
    const update = state.updatedPersonas[name]!;
    const parts = ["residual update persona"];
    if (update.concerns !== undefined) parts.push(flagText("concerns", update.concerns));
    if (update.desires !== undefined) parts.push(flagText("desires", update.desires));
    parts.push(flagText("name", name));
    if (update.role !== undefined) parts.push(flagText("role", update.role));
    return { line: parts.join(" "), valid: true };
  };

  const renderTermAdd = (term: string): CommandLine => {
    const t = addedTermByTerm.get(term)!;
    const parts = ["residual add term", flagText("definition", t.definition)];
    if (t.domain !== undefined) parts.push(flagText("domain", t.domain));
    if (t.related !== undefined) parts.push(flagText("related", t.related));
    parts.push(flagText("term", t.term));
    return { line: parts.join(" "), valid: true };
  };

  const renderTermUpdate = (term: string): CommandLine => {
    const update = state.updatedTerms[term]!;
    const parts = ["residual update term"];
    if (update.definition !== undefined) parts.push(flagText("definition", update.definition));
    if (update.domain !== undefined) parts.push(flagText("domain", update.domain));
    if (update.related !== undefined) parts.push(flagText("related", update.related));
    parts.push(flagText("term", term));
    return { line: parts.join(" "), valid: true };
  };

  const renderForceAdd = (tempId: string): CommandLine => {
    const f = addedForceByTempId.get(tempId)!;
    // Full isForceValid (via forceIsValid/computeStateValidity), not just
    // the text-field check: "no components selected is invalid" applies to
    // the whole row, so a force with zero components must render its add
    // line as invalid too, even though components aren't literally a flag
    // on `add stressor`/`add purpose` (see module notes).
    const valid = forceIsValid(tempId);
    const flags = [
      flagText("attractor-shortname", attractorShortname(f.attractorId)),
      flagText("description", f.description),
      flagText("naive-change", f.naiveChangeOrFeature),
      flagText("shortname", f.shortname),
    ];
    if (f.outcomes !== "") flags.push(flagText("outcomes", f.outcomes));

    return { line: ["residual add", f.kind, ...flags].join(" "), valid };
  };

  const renderForceUpdateSynthetic = (tempId: string): CommandLine => {
    const f = addedForceByTempId.get(tempId)!;
    const valid = forceIsValid(tempId);
    const parts = [`residual update ${f.kind}`, flagText("shortname", f.shortname)];
    for (const component of f.components) {
      parts.push(flagText("add-component", component));
    }
    return { line: parts.join(" "), valid };
  };

  const renderForceUpdateReal = (id: string): CommandLine => {
    const base = baseForceById.get(id)!;
    const update = state.updatedForces[id]!;
    const valid = forceIsValid(id);
    const parts = [`residual update ${base.kind}`, flagText("shortname", base.shortname)];
    if (update.description !== undefined) parts.push(flagText("description", update.description));
    if (update.attractorId !== undefined) {
      parts.push(flagText("attractor-shortname", attractorShortname(update.attractorId)));
    }
    if (update.naiveChangeOrFeature !== undefined) parts.push(flagText("naive-change", update.naiveChangeOrFeature));
    if (update.outcomes !== undefined) parts.push(flagText("outcomes", update.outcomes));
    if (update.shortname !== undefined) parts.push(flagText("rename", update.shortname));
    if (update.components !== undefined) {
      const before = new Set(base.components);
      const after = new Set(update.components);
      const added = update.components.filter((c) => !before.has(c));
      const removed = base.components.filter((c) => !after.has(c));
      for (const c of added) parts.push(flagText("add-component", c));
      for (const c of removed) parts.push(flagText("remove-component", c));
    }
    return { line: parts.join(" "), valid };
  };

  const lines = entries.map((entry): CommandLine => {
    if (entry.bucket === "component") {
      return entry.action === "add" ? renderComponentAdd(entry.key) : renderComponentUpdate(entry.key);
    }
    if (entry.bucket === "attractor") {
      return entry.action === "add" ? renderAttractorAdd(entry.key) : renderAttractorUpdate(entry.key);
    }
    if (entry.bucket === "persona") {
      return entry.action === "add" ? renderPersonaAdd(entry.key) : renderPersonaUpdate(entry.key);
    }
    if (entry.bucket === "term") {
      return entry.action === "add" ? renderTermAdd(entry.key) : renderTermUpdate(entry.key);
    }
    // bucket === "force"
    if (entry.action === "add") return renderForceAdd(entry.key);
    return addedForceByTempId.has(entry.key)
      ? renderForceUpdateSynthetic(entry.key)
      : renderForceUpdateReal(entry.key);
  });

  for (const [id, kind] of Object.entries(state.removedForces ?? {})) {
    const force = baseForceById.get(id);
    if (force !== undefined) {
      lines.push({ line: `residual ${kind} remove ${flagText("shortname", force.shortname)}`, valid: true });
    }
  }
  return lines;
}

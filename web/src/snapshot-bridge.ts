// Converts the raw embedded landscape snapshot JSON (Rust snake_case shape,
// see src/view/snapshot.rs's `LandscapeSnapshot`) into the camelCase
// `PendingState` shape that model.ts's live-editing layer operates on.
//
// This is the missing wiring between the server-rendered
// `<script id="residual-snapshot" type="application/json">` payload and
// everything built on top of model.ts in earlier phases. Defense-section
// data is entirely out of scope here.

import type { PendingState, SnapshotAttractor, SnapshotComponent, SnapshotForce } from "./model";

/** Raw (Rust snake_case) force record, as embedded in the snapshot JSON. */
export interface RawSnapshotForce {
  id: string;
  shortname: string;
  description: string;
  naive_change: string;
  outcomes: string;
  attractor_id: string;
  kind: "stressor" | "purpose";
  source: "sidecar" | "working";
}

/** Raw (Rust snake_case) attractor record, as embedded in the snapshot JSON. */
export interface RawSnapshotAttractor {
  id: string;
  name: string;
  description: string;
  positive_state: string;
  negative_state: string;
  source: "sidecar" | "working";
}

/** Raw (Rust snake_case) component record, as embedded in the snapshot JSON. */
export interface RawSnapshotComponent {
  name: string;
  description: string;
  status: string;
  architecture_set: string;
  source: "sidecar" | "working";
}

/** Raw (Rust snake_case) residue record, as embedded in the snapshot JSON. */
export interface RawSnapshotResidue {
  force_id: string;
  component_id: string;
  coupled: boolean;
  whole_system: boolean;
  notes: string;
  source: "sidecar" | "working";
}

/** Raw (Rust snake_case) top-level landscape snapshot, as embedded in the page. */
export interface RawLandscapeSnapshot {
  attractors: RawSnapshotAttractor[];
  stressors: RawSnapshotForce[];
  purposes: RawSnapshotForce[];
  components: RawSnapshotComponent[];
  residues: RawSnapshotResidue[];
}

/**
 * Converts a raw embedded landscape snapshot into a fresh `PendingState`
 * with no pending edits: base collections populated from the raw snapshot,
 * all `added*` collections empty, all `updated*` records empty.
 */
function mapForce(raw: RawSnapshotForce, kind: "stressor" | "purpose", residues: RawSnapshotResidue[]): SnapshotForce {
  return {
    id: raw.id,
    kind,
    description: raw.description,
    attractorId: raw.attractor_id,
    naiveChangeOrFeature: raw.naive_change,
    outcomes: raw.outcomes,
    shortname: raw.shortname,
    components: residues
      .filter((r) => r.force_id === raw.id && r.coupled === true)
      .map((r) => r.component_id),
  };
}

export function snapshotToPendingState(raw: RawLandscapeSnapshot): PendingState {
  const baseAttractors: SnapshotAttractor[] = raw.attractors.map((a) => ({
    id: a.id,
    name: a.name,
    description: a.description,
    positiveState: a.positive_state,
    negativeState: a.negative_state,
  }));

  const baseComponents: SnapshotComponent[] = raw.components.map((c) => ({
    name: c.name,
    description: c.description,
    status: c.status as SnapshotComponent["status"],
    architectureSet: c.architecture_set,
  }));

  const mappedStressors = raw.stressors.map((s) => mapForce(s, "stressor", raw.residues));
  const mappedPurposes = raw.purposes.map((p) => mapForce(p, "purpose", raw.residues));

  return {
    baseAttractors,
    baseComponents,
    baseForces: [...mappedStressors, ...mappedPurposes],
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
  };
}

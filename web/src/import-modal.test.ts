import { beforeEach, describe, expect, test } from "bun:test";
import { mountImportModal } from "./import-modal";
import type { PendingState, SnapshotAttractor, SnapshotComponent, SnapshotForce } from "./model";

// ---------------------------------------------------------------------------
// Fixtures — mirrors forms.test.ts's mount pattern: a hand-built HTML string
// matching import-modal.ts's documented markup contract, plus an in-memory
// getState/setState pair and an onChange counter.
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

function baseState(overrides: Partial<PendingState> = {}): PendingState {
  return {
    baseAttractors: [baseAttractor],
    baseComponents: [baseComponentCli],
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

function fixtureHtml(): string {
  return `
    <button data-import-trigger>Import</button>
    <div data-import-modal hidden>
      <textarea data-import-primary></textarea>
      <textarea data-import-errors></textarea>
      <ul data-import-error-list></ul>
      <button data-import-run>Import</button>
      <button data-import-retry>Retry</button>
    </div>
  `;
}

function mountFixture(state: PendingState): {
  container: HTMLElement;
  getState: () => PendingState;
  setState: (next: PendingState) => void;
  changeCount: () => number;
} {
  document.body.innerHTML = fixtureHtml();
  const container = document.body;

  let current = state;
  let changes = 0;
  const getState = (): PendingState => current;
  const setState = (next: PendingState): void => {
    current = next;
  };

  mountImportModal(container, getState, setState, {
    onChange: () => {
      changes += 1;
    },
  });

  return { container, getState, setState, changeCount: () => changes };
}

function textarea(container: HTMLElement, selector: string): HTMLTextAreaElement {
  const el = container.querySelector(selector);
  if (!(el instanceof HTMLTextAreaElement)) throw new Error(`fixture setup failed: ${selector} not a textarea`);
  return el;
}

function click(el: Element | null): void {
  if (el === null) throw new Error("fixture setup failed: element not found");
  el.dispatchEvent(new Event("click", { bubbles: true, cancelable: true }));
}

const VALID_COMPONENT_LINE =
  'residual add component --name "queue" --description "message queue" --status "actual" --architecture-set "iter2"';
const VALID_ATTRACTOR_LINE =
  'residual add attractor --name "trust" --description "operators believe the system" --positive-state "confidence" --negative-state "suspicion"';
const INVALID_LINE = "residual add component --name \"queue\"";

beforeEach(() => {
  document.body.innerHTML = "";
});

describe("mountImportModal — opening", () => {
  test("clicking the trigger button reveals the modal", () => {
    const { container } = mountFixture(baseState());
    const modal = container.querySelector("[data-import-modal]");
    expect(modal?.hasAttribute("hidden")).toBe(true);

    click(container.querySelector("[data-import-trigger]"));

    expect(modal?.hasAttribute("hidden")).toBe(false);
  });
});

describe("mountImportModal — successful import", () => {
  test("an all-valid paste merges into state and clears the primary textarea", () => {
    const { container, getState, changeCount } = mountFixture(baseState());
    const primary = textarea(container, "[data-import-primary]");
    primary.value = VALID_COMPONENT_LINE;

    click(container.querySelector("[data-import-run]"));

    expect(getState().addedComponents).toEqual([
      { name: "queue", description: "message queue", status: "actual", architectureSet: "iter2" },
    ]);
    expect(primary.value).toBe("");
    expect(changeCount()).toBe(1);
  });

  test("import is additive — existing pending state is preserved, not replaced", () => {
    const existing = baseState({
      addedAttractors: [
        { id: "NEW-ATTR-1", name: "existing", description: "d", positiveState: "p", negativeState: "n" },
      ],
    });
    const { container, getState } = mountFixture(existing);
    const primary = textarea(container, "[data-import-primary]");
    primary.value = VALID_COMPONENT_LINE;

    click(container.querySelector("[data-import-run]"));

    expect(getState().addedAttractors).toHaveLength(1);
    expect(getState().addedComponents).toHaveLength(1);
  });
});

describe("mountImportModal — mixed valid/invalid import", () => {
  test("splits valid and invalid lines: valid effects land in state, invalid raw text lands in the errors textarea with a human-readable message", () => {
    const { container, getState } = mountFixture(baseState());
    const primary = textarea(container, "[data-import-primary]");
    primary.value = `${VALID_COMPONENT_LINE}\n${INVALID_LINE}`;

    click(container.querySelector("[data-import-run]"));

    expect(getState().addedComponents).toHaveLength(1);

    const errors = textarea(container, "[data-import-errors]");
    expect(errors.value).toContain(INVALID_LINE);
    expect(primary.value).not.toContain(INVALID_LINE);

    const errorList = container.querySelector("[data-import-error-list]");
    expect(errorList?.textContent?.length ?? 0).toBeGreaterThan(0);
    expect(errorList?.querySelectorAll("li").length).toBe(1);
  });
});

describe("mountImportModal — retry", () => {
  test("fixing a previously-bad line in the errors textarea and clicking Retry merges it and clears it from the errors box", () => {
    const { container, getState } = mountFixture(baseState());
    const primary = textarea(container, "[data-import-primary]");
    primary.value = INVALID_LINE;
    click(container.querySelector("[data-import-run]"));

    const errors = textarea(container, "[data-import-errors]");
    expect(errors.value).toContain(INVALID_LINE);

    // Fix the line directly in the errors textarea (add the missing required flags).
    errors.value = VALID_COMPONENT_LINE;

    click(container.querySelector("[data-import-retry]"));

    expect(getState().addedComponents).toEqual([
      { name: "queue", description: "message queue", status: "actual", architectureSet: "iter2" },
    ]);
    expect(errors.value).toBe("");
  });

  test("retry leaves a still-invalid line in the errors box while merging a fixed one from the primary box", () => {
    const { container, getState } = mountFixture(baseState());
    const primary = textarea(container, "[data-import-primary]");
    primary.value = `${INVALID_LINE}\n${VALID_ATTRACTOR_LINE}`;
    click(container.querySelector("[data-import-run]"));

    const errors = textarea(container, "[data-import-errors]");
    expect(errors.value).toContain(INVALID_LINE);
    expect(getState().addedAttractors).toHaveLength(1);

    click(container.querySelector("[data-import-retry]"));

    // Still invalid — remains in the errors box, no crash, no duplicate merge.
    expect(errors.value).toContain(INVALID_LINE);
    expect(getState().addedAttractors).toHaveLength(1);
  });
});

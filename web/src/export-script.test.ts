import { beforeEach, describe, expect, test } from "bun:test";
import {
  buildBashScript,
  EXPORT_SCRIPT_FILENAME,
  mountExportScript,
  type TriggerDownload,
} from "./export-script";
import { toCommandLines, type PendingState, type SnapshotAttractor, type SnapshotForce } from "./model";

// ---------------------------------------------------------------------------
// Fixtures — mirrors model.test.ts's/forms.test.ts's minimal-state pattern.
// ---------------------------------------------------------------------------

const baseAttractor: SnapshotAttractor = {
  id: "A-01",
  name: "resilience",
  description: "system tolerates partial failure",
  positiveState: "graceful degradation",
  negativeState: "cascading failure",
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

// An intentionally-invalid update (blank description) so the command
// section exercises the same `# `-prefixing as the plain-copy path.
const invalidatingUpdate = { description: "" };

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

describe("buildBashScript", () => {
  test("first line is exactly the literal shebang, no leading blank line or BOM", () => {
    const script = buildBashScript(emptyState());
    const firstLine = script.split("\n")[0];
    expect(firstLine).toBe("#!/bin/env bash");
    expect(script.startsWith("#!/bin/env bash")).toBe(true);
    expect(script.charCodeAt(0)).not.toBe(0xfeff);
  });

  test("includes the write-authorization command", () => {
    const script = buildBashScript(emptyState());
    expect(script).toContain("residual write authorize");
  });

  test("command section matches toCommandLines exactly, including invalid-line commenting", () => {
    const state = emptyState({
      baseAttractors: [baseAttractor],
      baseForces: [baseStressor],
      updatedForces: { "S-01": invalidatingUpdate },
    });

    const script = buildBashScript(state);
    const expectedCommandLines = toCommandLines(state).map((cl) => (cl.valid ? cl.line : `# ${cl.line}`));

    for (const line of expectedCommandLines) {
      expect(script).toContain(line);
    }
    // The invalid update line must actually be `#`-commented in the script,
    // not just present as a substring of an uncommented line.
    const invalidLine = toCommandLines(state).find((cl) => !cl.valid);
    expect(invalidLine).toBeDefined();
    expect(script).toContain(`# ${invalidLine!.line}`);
  });

  test("empty PendingState produces shebang + auth line but does not crash on an empty command section", () => {
    const script = buildBashScript(emptyState());
    expect(script.length).toBeGreaterThan(0);
    expect(script).toContain("#!/bin/env bash");
    expect(script).toContain("residual write authorize");
  });

  test("pins the exact full script format for a simple two-line state (snapshot-style)", () => {
    // Base-only entries (no adds/updates) emit zero command lines by
    // toCommandLines' design (it renders diffs, not a full snapshot dump —
    // see model.ts's module notes). A no-op update per base entry is the
    // minimal way to actually exercise two rendered command lines here.
    const state = emptyState({
      baseAttractors: [baseAttractor],
      baseForces: [baseStressor],
      updatedAttractors: { "A-01": {} },
      updatedForces: { "S-01": {} },
    });
    const commandLines = toCommandLines(state).map((cl) => (cl.valid ? cl.line : `# ${cl.line}`));
    const expected = `#!/bin/env bash\n\nresidual write authorize\n\n${commandLines.join("\n")}\n`;
    expect(buildBashScript(state)).toBe(expected);
  });

  test("pins the exact full script format for a truly empty state", () => {
    const expected = "#!/bin/env bash\n\nresidual write authorize\n\n";
    expect(buildBashScript(emptyState())).toBe(expected);
  });
});

describe("mountExportScript", () => {
  function makeContainer(): HTMLElement {
    const container = document.createElement("div");
    container.innerHTML = "";
    const button = document.createElement("button");
    button.setAttribute("data-generate-script", "");
    button.textContent = "Generate bash script";
    container.appendChild(button);
    document.body.appendChild(container);
    return container;
  }

  let container: HTMLElement;

  beforeEach(() => {
    document.body.innerHTML = "";
    container = makeContainer();
  });

  test("clicking [data-generate-script] calls the injected triggerDownload with the exact filename and content", () => {
    const state = emptyState({
      baseAttractors: [baseAttractor],
      baseForces: [baseStressor],
    });
    const calls: Array<{ filename: string; content: string }> = [];
    const triggerDownload: TriggerDownload = (filename, content) => {
      calls.push({ filename, content });
    };

    mountExportScript(container, () => state, { triggerDownload });

    const button = container.querySelector<HTMLButtonElement>("[data-generate-script]");
    expect(button).not.toBeNull();
    button!.click();

    expect(calls.length).toBe(1);
    expect(calls[0]!.filename).toBe(EXPORT_SCRIPT_FILENAME);
    expect(calls[0]!.content).toBe(buildBashScript(state));
  });

  test("calls onChange after a successful generate-and-download", () => {
    const state = emptyState();
    let changed = 0;
    mountExportScript(container, () => state, {
      triggerDownload: () => {},
      onChange: () => {
        changed += 1;
      },
    });

    container.querySelector<HTMLButtonElement>("[data-generate-script]")!.click();

    expect(changed).toBe(1);
  });

  test("reflects the latest getState() on each click, not a stale snapshot", () => {
    let state = emptyState();
    const calls: string[] = [];
    mountExportScript(container, () => state, {
      triggerDownload: (_filename, content) => {
        calls.push(content);
      },
    });

    const button = container.querySelector<HTMLButtonElement>("[data-generate-script]")!;
    button.click();

    // Base-only entries emit zero command lines (see the "two-line state"
    // test above) — a no-op update is needed here too so the rebuilt
    // script's content actually differs from the first, truly-empty click.
    state = emptyState({
      baseAttractors: [baseAttractor],
      baseForces: [baseStressor],
      updatedAttractors: { "A-01": {} },
      updatedForces: { "S-01": {} },
    });
    button.click();

    expect(calls.length).toBe(2);
    expect(calls[0]).not.toBe(calls[1]);
  });

  // Best-effort: without overriding triggerDownload, the real
  // Blob/anchor/URL.createObjectURL path must not throw even in an
  // environment (like happy-dom) that may not fully support it — the
  // default implementation is expected to check `typeof
  // URL.createObjectURL === "function"` and no-op gracefully otherwise.
  test("without an injected triggerDownload, clicking the button does not throw", () => {
    const state = emptyState();
    mountExportScript(container, () => state);
    const button = container.querySelector<HTMLButtonElement>("[data-generate-script]")!;
    expect(() => button.click()).not.toThrow();
  });

  test("does nothing (and does not throw) if [data-generate-script] is absent from the container", () => {
    const emptyContainer = document.createElement("div");
    document.body.appendChild(emptyContainer);
    expect(() => mountExportScript(emptyContainer, () => emptyState())).not.toThrow();
  });
});

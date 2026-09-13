// Bash-script export path for the live-editing view (Phase 7 of
// residual/iterations/view-live-editing-plan.md — "Export (bash script)").
//
// This is a SIBLING to forms.ts's existing "Copy commands" flow
// (`regenerateStagedCommands` / `#staged-commands` / `#copy-staged`), which
// stays completely unchanged and never gains a write-authorization header.
// This module adds a NEW "Generate bash script" button that downloads a
// `.sh` file containing a shebang, `residual write authorize`, and the same
// ordered `residual add`/`residual update` commands `toCommandLines`
// produces (./model), with the same `# `-prefixing of invalid lines as the
// plain-copy textarea.
//
// Design decisions (documented per repo instructions):
//
// - Selector: `[data-generate-script]` on the new button, mirroring the
//   existing `#copy-staged`/`#clear-staged` id-based hooks used by
//   forms.ts, but as a data-attribute since this button is not expected to
//   be a singleton-by-id requirement anywhere else in the DOM contract.
// - Filename: `residual-import.sh` — this script is meant to be fed to
//   `bash residual-import.sh` to replay a live-editing session's staged
//   changes against the real CLI/CSVs, mirroring the "import" vocabulary
//   already used by import-parser.ts/import-modal.ts for the reverse
//   direction (pasting commands back in).
// - `residual write authorize` is emitted bare (no `--minutes` flag),
//   relying on its own default (`crate::storage::write_authorization::
//   DEFAULT_MINUTES`, currently 30 in src/storage/write_authorization.rs) —
//   simplicity per the task's own suggestion; the script doesn't need to
//   assert a specific number to work correctly.
// - Download mechanics (Blob/URL.createObjectURL/hidden-anchor-click) are
//   isolated behind an injectable `options.triggerDownload` seam so tests
//   never depend on happy-dom's (partial/absent) support for real browser
//   downloads. The default implementation defensively no-ops if
//   `URL.createObjectURL` isn't a function, rather than throwing.

import { toCommandLines, type PendingState } from "./model";

/** Filename used for the downloaded script — see module notes. */
export const EXPORT_SCRIPT_FILENAME = "residual-import.sh";

/**
 * Builds the full bash script text for the current `state`: a literal
 * `#!/bin/env bash` shebang as the first line, a blank line, the
 * write-authorization command, a blank line, then one `toCommandLines`
 * line per entry (in its order), `#`-prefixed when `valid` is `false`, each
 * on its own line, with a trailing newline.
 */
export function buildBashScript(state: PendingState): string {
  const commandLines = toCommandLines(state).map((cl) => (cl.valid ? cl.line : `# ${cl.line}`));
  const commandSection = commandLines.length > 0 ? `${commandLines.join("\n")}\n` : "";
  return `#!/bin/env bash\n\nresidual write authorize\n\n${commandSection}`;
}

/** Injectable download-trigger signature — see module notes on the seam. */
export type TriggerDownload = (filename: string, content: string) => void;

/**
 * Default `TriggerDownload`: builds a Blob, an object URL, and a hidden
 * `<a download>` element, clicks it, then cleans up. No-ops (rather than
 * throwing) if `URL.createObjectURL` is unavailable in the current
 * environment, since there is no real browser download mechanism to fall
 * back to anyway.
 */
function defaultTriggerDownload(filename: string, content: string): void {
  if (typeof URL === "undefined" || typeof URL.createObjectURL !== "function") return;
  const blob = new Blob([content], { type: "application/x-sh" });
  const url = URL.createObjectURL(blob);
  try {
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = filename;
    anchor.style.display = "none";
    document.body.appendChild(anchor);
    anchor.click();
    anchor.remove();
  } finally {
    URL.revokeObjectURL(url);
  }
}

/** Options controlling mountExportScript's side effects. */
export interface MountExportScriptOptions {
  /** Called after a successful script generation + download trigger. */
  onChange?: () => void;
  /** Overrides the real Blob/anchor/URL.createObjectURL download mechanics — see module notes. */
  triggerDownload?: TriggerDownload;
}

/**
 * Wires the `[data-generate-script]` button inside `container`: on click,
 * builds the script via `buildBashScript(getState())` and hands it to
 * `options.triggerDownload` (defaulting to the real Blob/anchor download
 * implementation) along with `EXPORT_SCRIPT_FILENAME`.
 */
export function mountExportScript(
  container: HTMLElement,
  getState: () => PendingState,
  options?: MountExportScriptOptions,
): void {
  const button = container.querySelector<HTMLButtonElement>("[data-generate-script]");
  if (!button) return;

  const triggerDownload = options?.triggerDownload ?? defaultTriggerDownload;

  button.addEventListener("click", () => {
    const script = buildBashScript(getState());
    triggerDownload(EXPORT_SCRIPT_FILENAME, script);
    options?.onChange?.();
  });
}

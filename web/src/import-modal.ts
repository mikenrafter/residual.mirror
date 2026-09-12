// DOM-wiring layer for the "Import" modal (Phase 6 of
// residual/iterations/view-live-editing-plan.md): lets a user paste
// `residual add ...` / `residual update ...` CLI text back into the live
// editing view. Delegates parsing to import-parser.ts's `parseImportText`
// and state application to import-merge.ts's `mergeImportedItems` — this
// module owns DOM mutation and the import/retry control flow only, mirroring
// forms.ts's/matrix-interactions.ts's split of "DOM layer calls pure
// reducers/model functions, never reimplements their logic".
//
// Markup contract (designed here, not dictated by shell.html yet — this is
// new UI, so mountImportModal both defines and consumes it):
//
//   <button data-import-trigger>Import</button>
//   <div data-import-modal hidden>
//     <textarea data-import-primary></textarea>
//     <textarea data-import-errors></textarea>
//     <ul data-import-error-list></ul>
//     <button data-import-run>Import</button>
//     <button data-import-retry>Retry</button>
//   </div>
//
// - `data-import-trigger` toggles the `hidden` attribute off `data-import-
//   modal` when clicked (open only — no explicit close button is required by
//   the task spec, so one isn't tested here; a close affordance can be added
//   later without changing this contract).
// - `data-import-primary` is the main paste target. `data-import-errors` is
//   a SECOND real `<textarea>` (per the task's explicit "move the erroneous
//   lines into a second textbox" wording) holding the raw, verbatim text of
//   lines that failed to parse — one per line, in `ParseError` order.
//   `data-import-error-list` is a separate `<ul>` of human-readable
//   "<raw line>: <message>" entries, since the errors textarea only carries
//   raw text and the task also wants messages surfaced somewhere.
// - `data-import-run` reads ONLY `data-import-primary`'s current value,
//   parses it, and:
//     - moves failing lines' raw text into `data-import-errors` (removing
//       them from `data-import-primary`'s displayed value — the erroneous
//       lines are NOT left behind in the primary box);
//     - populates `data-import-error-list` with one `<li>` per error;
//     - merges successfully-parsed items into state via
//       `mergeImportedItems` and calls `setState`;
//     - clears `data-import-primary` on a fully- or partially-successful
//       import (pinned design choice: an emptied primary box signals
//       "everything that could be applied, was" and is ready for the next
//       paste — still-bad lines live in the errors box, not the primary
//       one, so clearing primary never loses anything);
//     - calls `options?.onChange?.()`.
// - `data-import-retry` re-parses the COMBINED content of BOTH textareas
//   (primary's current value, then errors' current value, newline-joined —
//   letting a user fix a bad line directly inside the errors textarea and
//   retry without re-pasting everything), then applies the same
//   split/merge/clear logic as `data-import-run` against that combined text:
//   newly-valid lines move into (are represented by re-populating)
//   `data-import-primary` before being cleared post-merge, and still-invalid
//   lines remain in `data-import-errors`.

import { mergeImportedItems } from "./import-merge";
import { parseImportText } from "./import-parser";
import type { PendingState } from "./model";

/** Options controlling side effects mountImportModal() triggers beyond the DOM/state update itself. */
export interface MountImportModalOptions {
  /** Called after every state-mutating Import/Retry action, once `setState` has already been invoked. */
  onChange?: () => void;
}

/**
 * Attaches the Import modal's open/import/retry interactions to `container`
 * (an element already containing the markup described in this file's header
 * comment: `[data-import-trigger]`, `[data-import-modal]`,
 * `[data-import-primary]`, `[data-import-errors]`, `[data-import-error-list]`,
 * `[data-import-run]`, `[data-import-retry]`).
 */
/** A raw-line/message pair to surface in the errors textarea + error list, from either a ParseError or an unmergeable PendingItem. */
interface ImportErrorEntry {
  line: string;
  message: string;
}

function requireElement<T extends Element>(container: HTMLElement, selector: string): T {
  const el = container.querySelector(selector);
  if (el === null) throw new Error(`mountImportModal: missing required element ${selector}`);
  return el as T;
}

export function mountImportModal(
  container: HTMLElement,
  getState: () => PendingState,
  setState: (next: PendingState) => void,
  options?: MountImportModalOptions,
): void {
  const trigger = requireElement<HTMLElement>(container, "[data-import-trigger]");
  const modal = requireElement<HTMLElement>(container, "[data-import-modal]");
  const primary = requireElement<HTMLTextAreaElement>(container, "[data-import-primary]");
  const errorsBox = requireElement<HTMLTextAreaElement>(container, "[data-import-errors]");
  const errorList = requireElement<HTMLElement>(container, "[data-import-error-list]");
  const runButton = requireElement<HTMLElement>(container, "[data-import-run]");
  const retryButton = requireElement<HTMLElement>(container, "[data-import-retry]");

  const renderErrors = (entries: ImportErrorEntry[]): void => {
    errorsBox.value = entries.map((e) => e.line).join("\n");

    while (errorList.firstChild) errorList.removeChild(errorList.firstChild);
    for (const entry of entries) {
      const li = document.createElement("li");
      li.textContent = `${entry.line}: ${entry.message}`;
      errorList.appendChild(li);
    }
  };

  /** Parses `text`, merges the successfully-parsed items into state, and returns the combined list of still-bad lines (parse errors + unmergeable items). */
  const processImportText = (text: string): ImportErrorEntry[] => {
    const parsed = parseImportText(text);
    const mergeResult = mergeImportedItems(getState(), parsed.items);
    setState(mergeResult.state);

    const entries: ImportErrorEntry[] = [
      ...parsed.errors.map((e): ImportErrorEntry => ({ line: e.line, message: e.message })),
      ...mergeResult.unmergeable.map(
        (item): ImportErrorEntry => ({
          line: item.raw,
          message: `no support yet for "${item.kind} ${item.type}" updates`,
        }),
      ),
    ];
    return entries;
  };

  trigger.addEventListener("click", () => {
    modal.removeAttribute("hidden");
  });

  runButton.addEventListener("click", () => {
    const entries = processImportText(primary.value);
    renderErrors(entries);
    primary.value = "";
    options?.onChange?.();
  });

  retryButton.addEventListener("click", () => {
    const combined = `${primary.value}\n${errorsBox.value}`;
    const entries = processImportText(combined);
    renderErrors(entries);
    primary.value = "";
    options?.onChange?.();
  });
}

// DOM-wiring layer restoring the matrix-view interactions that used to live
// in an inline <script> in src/view/shell.html (live text filter, click-to-
// sort headers, accordion expand/collapse, fusion/fission threshold
// highlighting). That inline script was deleted wholesale while wiring in
// the new TS-based live-editing system (Phases 0-8 of
// residual/iterations/view-live-editing-plan.md) and never reimplemented —
// this module is the fix for that regression.
//
// Unlike matrix-interactions.ts / forms.ts, this module owns no application
// state: it is pure DOM presentation logic (filter/sort/accordion/fusion-
// fission-highlighting) layered on top of whatever `table.matrix` currently
// contains. Because matrix-interactions.ts and forms.ts can add rows/columns
// to that table at any time via their own `onChange` callbacks, every
// listener here is delegated on `container` (not bound per-row/per-header)
// so newly added elements are handled automatically without rewiring, and
// `recomputeFusionFission()` is exposed on the returned handle so a caller
// can force a recompute after a mutation it knows about (e.g. hang it off
// the same shared `onChange` callback matrix-interactions.ts/forms.ts already
// use).
//
// Threshold slider auto-initialization (min/max/value) happens once, at
// mount time, based on the number of `tr.force-row` present then — matching
// the original inline script's on-load behavior. It is intentionally not
// re-derived by `recomputeFusionFission()`.

/** Options controlling mountMatrixView(); currently there are none, but the
 * shape is kept for parity with MountOptions/MountFormsOptions in the sibling
 * modules and to leave room for a future `onChange`-style hook. */
export interface MatrixViewOptions {
  onChange?: () => void;
}

/** Returned by mountMatrixView() so a caller can force a fusion/fission
 * recompute after externally mutating the table (new rows/columns). */
export interface MatrixViewHandle {
  recomputeFusionFission: () => void;
}

type SortDir = "asc" | "desc";

function sortMatrixBy(container: HTMLElement, key: string, dir: SortDir): void {
  const table = container.querySelector("table.matrix");
  const tbody = table?.querySelector("tbody");
  if (!tbody) return;
  const rows = Array.from(tbody.querySelectorAll<HTMLTableRowElement>("tr.force-row"));
  const mult = dir === "desc" ? -1 : 1;

  rows.sort((ra, rb) => {
    if (key === "force") {
      return (ra.getAttribute("data-force-id") ?? "").localeCompare(rb.getAttribute("data-force-id") ?? "") * mult;
    }
    if (key === "total") {
      return (Number(ra.getAttribute("data-row-total") ?? 0) - Number(rb.getAttribute("data-row-total") ?? 0)) * mult;
    }
    if (key.indexOf("component:") === 0) {
      const comp = key.slice("component:".length);
      const ca = ra.querySelector(`td[data-component="${CSS.escape(comp)}"]`);
      const cb = rb.querySelector(`td[data-component="${CSS.escape(comp)}"]`);
      const va = ca?.getAttribute("data-coupled") === "1" ? 1 : 0;
      const vb = cb?.getAttribute("data-coupled") === "1" ? 1 : 0;
      return (va - vb) * mult;
    }
    return 0;
  });

  for (const row of rows) tbody.appendChild(row);
}

interface FusionFissionResult {
  fusion: Set<string>;
  fission: Set<string>;
}

function computeMatrixCandidates(container: HTMLElement): FusionFissionResult {
  const table = container.querySelector("table.matrix");
  const fusion = new Set<string>();
  const fission = new Set<string>();
  if (!table) return { fusion, fission };

  const componentNames = Array.from(table.querySelectorAll("thead th[data-component]")).map(
    (th) => th.getAttribute("data-component") ?? "",
  );

  const vectors: Record<string, Record<string, boolean>> = {};
  for (const name of componentNames) vectors[name] = {};

  for (const td of Array.from(table.querySelectorAll("tbody td[data-residue-cell]"))) {
    const comp = td.getAttribute("data-component") ?? "";
    const force = td.getAttribute("data-force-id") ?? "";
    if (vectors[comp]) vectors[comp][force] = td.getAttribute("data-coupled") === "1";
  }

  for (let i = 0; i < componentNames.length; i++) {
    for (let j = i + 1; j < componentNames.length; j++) {
      const a = vectors[componentNames[i]];
      const b = vectors[componentNames[j]];
      const forceIds = Object.keys(a);
      const identical = forceIds.length > 0 && forceIds.every((fid) => a[fid] === b[fid]);
      if (identical) {
        fusion.add(componentNames[i]);
        fusion.add(componentNames[j]);
      }
    }
  }

  const thresholdInput = container.querySelector("[data-threshold-input]");
  const threshold = thresholdInput instanceof HTMLInputElement ? Number(thresholdInput.value) : 1;

  for (const td of Array.from(table.querySelectorAll("tfoot td[data-col-total]"))) {
    const total = Number(td.getAttribute("data-col-total"));
    if (total > threshold) fission.add(td.getAttribute("data-component") ?? "");
  }

  return { fusion, fission };
}

function applyFusionFissionHighlighting(container: HTMLElement): void {
  const table = container.querySelector("table.matrix");
  if (!table) return;

  const { fusion, fission } = computeMatrixCandidates(container);
  const componentNames = Array.from(table.querySelectorAll("thead th[data-component]")).map(
    (th) => th.getAttribute("data-component") ?? "",
  );

  const onlyToggle = container.querySelector("[data-fusion-fission-filter]");
  const only = onlyToggle instanceof HTMLInputElement && onlyToggle.checked;

  for (const name of componentNames) {
    const isFusion = fusion.has(name);
    const isFission = fission.has(name);
    const show = !only || isFusion || isFission;
    for (const el of Array.from(container.querySelectorAll(`[data-component="${CSS.escape(name)}"]`))) {
      el.setAttribute("data-fusion", isFusion ? "1" : "0");
      el.setAttribute("data-fission", isFission ? "1" : "0");
      if (el instanceof HTMLElement) el.hidden = !show;
    }
  }
}

export function mountMatrixView(container: HTMLElement, _options?: MatrixViewOptions): MatrixViewHandle {
  // --- Filter -----------------------------------------------------------
  container.addEventListener("input", (event) => {
    const target = event.target;
    if (!(target instanceof Element)) return;
    const filterInput = target.closest("[data-force-filter]");
    if (filterInput instanceof HTMLInputElement) {
      const q = filterInput.value.trim().toLowerCase();
      for (const row of Array.from(container.querySelectorAll<HTMLTableRowElement>("table.matrix tbody tr.force-row"))) {
        const hay = (row.getAttribute("data-search") || row.textContent || "").toLowerCase();
        row.hidden = q !== "" && hay.indexOf(q) === -1;
      }
      return;
    }

    const thresholdInput = target.closest("[data-threshold-input]");
    if (thresholdInput instanceof HTMLInputElement) {
      const thresholdValue = container.querySelector("[data-threshold-value]");
      if (thresholdValue) thresholdValue.textContent = thresholdInput.value;
      applyFusionFissionHighlighting(container);
    }
  });

  // --- Accordion + sort (click) ------------------------------------------
  container.addEventListener("click", (event) => {
    const target = event.target;
    if (!(target instanceof Element)) return;

    const toggle = target.closest("[data-accordion-toggle]");
    if (toggle instanceof HTMLElement) {
      const detail = toggle.nextElementSibling;
      if (!(detail instanceof HTMLElement)) return;
      const expanded = toggle.getAttribute("aria-expanded") === "true";
      toggle.setAttribute("aria-expanded", String(!expanded));
      detail.hidden = expanded;
      return;
    }

    const th = target.closest("table.matrix thead th[data-sort-key]");
    if (th instanceof HTMLElement) {
      activateSort(container, th);
    }
  });

  // --- Sort (keyboard) -----------------------------------------------------
  container.addEventListener("keydown", (event) => {
    if (!(event instanceof KeyboardEvent)) return;
    if (event.key !== "Enter" && event.key !== " ") return;
    const target = event.target;
    if (!(target instanceof Element)) return;
    const th = target.closest("table.matrix thead th[data-sort-key]");
    if (th instanceof HTMLElement) {
      event.preventDefault();
      activateSort(container, th);
    }
  });

  // --- Fusion/fission filter toggle (change) -------------------------------
  container.addEventListener("change", (event) => {
    const target = event.target;
    if (!(target instanceof Element)) return;
    const filterToggle = target.closest("[data-fusion-fission-filter]");
    if (filterToggle instanceof HTMLInputElement) {
      applyFusionFissionHighlighting(container);
    }
  });

  // --- Threshold slider auto-initialization (once, at mount time) ---------
  const thresholdInput = container.querySelector("[data-threshold-input]");
  const thresholdValue = container.querySelector("[data-threshold-value]");
  const numForces = container.querySelectorAll("table.matrix tbody tr.force-row").length;
  if (thresholdInput instanceof HTMLInputElement) {
    const max = Math.max(1, numForces);
    const value = Math.max(1, Math.floor(numForces / 2));
    thresholdInput.min = "1";
    thresholdInput.max = String(max);
    thresholdInput.value = String(value);
    if (thresholdValue) thresholdValue.textContent = String(value);
  }

  return {
    recomputeFusionFission: () => applyFusionFissionHighlighting(container),
  };
}

function activateSort(container: HTMLElement, th: HTMLElement): void {
  const key = th.getAttribute("data-sort-key");
  if (!key) return;
  const current: SortDir | null = th.classList.contains("sort-asc")
    ? "asc"
    : th.classList.contains("sort-desc")
      ? "desc"
      : null;
  const next: SortDir = current === "asc" ? "desc" : "asc";

  for (const other of Array.from(container.querySelectorAll("table.matrix thead th[data-sort-key]"))) {
    other.classList.remove("sort-asc", "sort-desc");
  }
  th.classList.add(next === "asc" ? "sort-asc" : "sort-desc");
  sortMatrixBy(container, key, next);
}

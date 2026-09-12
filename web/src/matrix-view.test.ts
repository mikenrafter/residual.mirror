import { beforeEach, describe, expect, test } from "bun:test";
import { mountMatrixView } from "./matrix-view";

// ---------------------------------------------------------------------------
// Fixture — a hand-built toolbar + `table.matrix` fixture matching the
// markup contract in src/view/shell.html (toolbar controls: data-force-filter,
// data-fusion-fission-filter, data-threshold-input, data-threshold-value) and
// src/view/components/matrix.rs (table: thead th[data-sort-key], tbody
// tr.force-row with accordion toggle + .force-detail, td[data-residue-cell],
// tfoot td[data-col-total]). This mirrors matrix-interactions.test.ts's
// fixture-building style, adapted for this module's toolbar + static-table
// concerns (no PendingState/reducer wiring needed here — this module owns no
// application state).
// ---------------------------------------------------------------------------

interface ForceRowSpec {
  id: string;
  search: string;
  rowTotal: number;
  coupled: Record<string, boolean>;
}

interface FixtureSpec {
  components: string[];
  forces: ForceRowSpec[];
  colTotals: Record<string, number>;
}

function forceRowHtml(spec: ForceRowSpec, components: string[]): string {
  const cells = components
    .map((c) => {
      const coupled = spec.coupled[c] === true;
      return `<td data-residue-cell="true" data-force-id="${spec.id}" data-component="${c}" data-coupled="${
        coupled ? "1" : "0"
      }">${coupled ? "1" : ""}</td>`;
    })
    .join("");

  return `
    <tr class="force-row" data-force-id="${spec.id}" data-search="${spec.search}" data-row-total="${spec.rowTotal}">
      <th class="sticky-col" data-force-id="${spec.id}">
        <button type="button" data-accordion-toggle aria-expanded="false">${spec.id}</button>
        <div class="force-detail" hidden>
          <dl><dt>id</dt><dd>${spec.id}</dd></dl>
        </div>
      </th>
      ${cells}
      <td class="sticky-col-right" data-row-total="${spec.rowTotal}">${spec.rowTotal}</td>
    </tr>`;
}

function fixtureHtml(spec: FixtureSpec): string {
  const headerCells = spec.components
    .map((c) => `<th class="sticky-row" data-component="${c}" data-sort-key="component:${c}">${c}</th>`)
    .join("");
  const rows = spec.forces.map((f) => forceRowHtml(f, spec.components)).join("");
  const footerCells = spec.components
    .map((c) => `<td data-col-total="${spec.colTotals[c] ?? 0}" data-component="${c}">${spec.colTotals[c] ?? 0}</td>`)
    .join("");

  return `
    <div class="view-toolbar" data-view-toolbar>
      <input type="search" data-force-filter placeholder="filter…" />
      <label><input type="checkbox" data-fusion-fission-filter /> fusion/fission candidates only</label>
      <label>fission threshold
        <input type="range" data-threshold-input min="1" max="1" value="1" />
        <output data-threshold-value>1</output>
      </label>
    </div>
    <table class="matrix">
      <thead><tr>
        <th class="sticky-col sticky-row corner" data-sort-key="force">force</th>
        ${headerCells}
        <th class="sticky-row sticky-col-right corner" data-sort-key="total">total</th>
      </tr></thead>
      <tbody>${rows}</tbody>
      <tfoot><tr>
        <th class="sticky-col corner">totals</th>
        ${footerCells}
        <td class="sticky-col-right corner" data-grand-total="0">0</td>
      </tr></tfoot>
    </table>`;
}

/** The standard 4-force, 4-component fixture used across most tests below.
 * Coupling vectors: a=[1,0,1,0], b=[1,0,1,0] (identical -> fusion candidates),
 * c=[0,1,1,1] (distinct), d=[0,0,0,0] (never coupled). Column totals:
 * a=2, b=2, c=3, d=0. With N=4 force rows, threshold auto-inits to
 * max(1, floor(4/2)) = 2, so initially only c (3 > 2) is a fission candidate.
 * Rows are deliberately NOT pre-sorted by id or by total, so sort tests are
 * observable; each row's data-row-total is set to an arbitrary distinct
 * value (decoupled from actual coupling counts) to make total-sort
 * unambiguous. */
function standardFixture(): FixtureSpec {
  return {
    components: ["a", "b", "c", "d"],
    forces: [
      { id: "S-03", search: "S-03 gamma stressor", rowTotal: 8, coupled: { a: true, b: true, c: true } },
      { id: "S-01", search: "S-01 alpha stressor", rowTotal: 5, coupled: { a: true, b: true } },
      { id: "S-04", search: "S-04 delta purpose", rowTotal: 1, coupled: { c: true } },
      { id: "S-02", search: "S-02 beta stressor", rowTotal: 2, coupled: { c: true } },
    ],
    colTotals: { a: 2, b: 2, c: 3, d: 0 },
  };
}

function mountFixture(spec: FixtureSpec = standardFixture()): {
  container: HTMLElement;
  table: HTMLTableElement;
  handle: ReturnType<typeof mountMatrixView>;
} {
  document.body.innerHTML = `<div id="app">${fixtureHtml(spec)}</div>`;
  const container = document.getElementById("app");
  if (!(container instanceof HTMLElement)) throw new Error("fixture setup failed: #app not found");
  const table = container.querySelector("table.matrix");
  if (!(table instanceof HTMLTableElement)) throw new Error("fixture setup failed: table.matrix not found");

  const handle = mountMatrixView(container);
  return { container, table, handle };
}

function rowIds(table: HTMLTableElement): string[] {
  return Array.from(table.querySelectorAll<HTMLTableRowElement>("tbody tr.force-row")).map(
    (r) => r.getAttribute("data-force-id") ?? "",
  );
}

beforeEach(() => {
  document.body.innerHTML = "";
});

// ---------------------------------------------------------------------------
// 1. Filter
// ---------------------------------------------------------------------------

describe("mountMatrixView — filter", () => {
  test("typing into [data-force-filter] hides rows whose data-search doesn't match (case-insensitive substring)", () => {
    const { container, table } = mountFixture();
    const filterInput = container.querySelector("[data-force-filter]");
    if (!(filterInput instanceof HTMLInputElement)) throw new Error("filter input not found");

    filterInput.value = "ALPHA";
    filterInput.dispatchEvent(new Event("input", { bubbles: true }));

    const rows = Array.from(table.querySelectorAll<HTMLTableRowElement>("tbody tr.force-row"));
    const byId = new Map(rows.map((r) => [r.getAttribute("data-force-id"), r]));
    expect(byId.get("S-01")?.hidden).toBe(false);
    expect(byId.get("S-02")?.hidden).toBe(true);
    expect(byId.get("S-03")?.hidden).toBe(true);
    expect(byId.get("S-04")?.hidden).toBe(true);
  });

  test("clearing the filter input un-hides all rows", () => {
    const { container, table } = mountFixture();
    const filterInput = container.querySelector("[data-force-filter]");
    if (!(filterInput instanceof HTMLInputElement)) throw new Error("filter input not found");

    filterInput.value = "alpha";
    filterInput.dispatchEvent(new Event("input", { bubbles: true }));
    filterInput.value = "";
    filterInput.dispatchEvent(new Event("input", { bubbles: true }));

    const rows = Array.from(table.querySelectorAll<HTMLTableRowElement>("tbody tr.force-row"));
    for (const row of rows) {
      expect(row.hidden).toBe(false);
    }
  });
});

// ---------------------------------------------------------------------------
// 2. Accordion
// ---------------------------------------------------------------------------

describe("mountMatrixView — accordion", () => {
  test("clicking a [data-accordion-toggle] flips aria-expanded and its sibling .force-detail's hidden attribute, and reverses on a second click", () => {
    const { table } = mountFixture();
    const toggle = table.querySelector('tr[data-force-id="S-01"] [data-accordion-toggle]');
    if (!(toggle instanceof HTMLButtonElement)) throw new Error("toggle not found");
    const detail = toggle.nextElementSibling;
    if (!(detail instanceof HTMLElement)) throw new Error("detail not found");

    expect(toggle.getAttribute("aria-expanded")).toBe("false");
    expect(detail.hidden).toBe(true);

    toggle.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    expect(toggle.getAttribute("aria-expanded")).toBe("true");
    expect(detail.hidden).toBe(false);

    toggle.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    expect(toggle.getAttribute("aria-expanded")).toBe("false");
    expect(detail.hidden).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// 3. Sort
// ---------------------------------------------------------------------------

describe("mountMatrixView — sort", () => {
  test("clicking th[data-sort-key='force'] sorts ascending by data-force-id, then descending on a second click", () => {
    const { table } = mountFixture();
    const th = table.querySelector('thead th[data-sort-key="force"]');
    if (!(th instanceof HTMLElement)) throw new Error("force header not found");

    th.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    expect(rowIds(table)).toEqual(["S-01", "S-02", "S-03", "S-04"]);
    expect(th.classList.contains("sort-asc")).toBe(true);

    th.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    expect(rowIds(table)).toEqual(["S-04", "S-03", "S-02", "S-01"]);
    expect(th.classList.contains("sort-desc")).toBe(true);
  });

  test("clicking a different header clears the previous header's sort class (exclusive sort state)", () => {
    const { table } = mountFixture();
    const forceTh = table.querySelector('thead th[data-sort-key="force"]');
    const totalTh = table.querySelector('thead th[data-sort-key="total"]');
    if (!(forceTh instanceof HTMLElement) || !(totalTh instanceof HTMLElement)) {
      throw new Error("headers not found");
    }

    forceTh.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    expect(forceTh.classList.contains("sort-asc")).toBe(true);

    totalTh.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    expect(forceTh.classList.contains("sort-asc")).toBe(false);
    expect(forceTh.classList.contains("sort-desc")).toBe(false);
    expect(totalTh.classList.contains("sort-asc")).toBe(true);
  });

  test("clicking th[data-sort-key='total'] sorts numerically by data-row-total", () => {
    const { table } = mountFixture();
    const th = table.querySelector('thead th[data-sort-key="total"]');
    if (!(th instanceof HTMLElement)) throw new Error("total header not found");

    th.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    expect(rowIds(table)).toEqual(["S-04", "S-02", "S-01", "S-03"]); // totals 1,2,5,8

    th.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    expect(rowIds(table)).toEqual(["S-03", "S-01", "S-02", "S-04"]); // totals 8,5,2,1
  });

  test("clicking th[data-sort-key='component:a'] sorts by that column's data-coupled value", () => {
    const { table } = mountFixture();
    const th = table.querySelector('thead th[data-sort-key="component:a"]');
    if (!(th instanceof HTMLElement)) throw new Error("component:a header not found");

    th.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    // ascending: uncoupled (0) rows first in original relative order (S-04, S-02), then coupled (1) rows (S-03, S-01)
    expect(rowIds(table)).toEqual(["S-04", "S-02", "S-03", "S-01"]);
  });

  test("keyboard activation (Enter) on a focused header sorts the same as a click", () => {
    const { table } = mountFixture();
    const th = table.querySelector('thead th[data-sort-key="force"]');
    if (!(th instanceof HTMLElement)) throw new Error("force header not found");

    th.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true }));
    expect(rowIds(table)).toEqual(["S-01", "S-02", "S-03", "S-04"]);
  });

  test("keyboard activation (Space) on a focused header sorts the same as a click", () => {
    const { table } = mountFixture();
    const th = table.querySelector('thead th[data-sort-key="force"]');
    if (!(th instanceof HTMLElement)) throw new Error("force header not found");

    th.dispatchEvent(new KeyboardEvent("keydown", { key: " ", bubbles: true, cancelable: true }));
    expect(rowIds(table)).toEqual(["S-01", "S-02", "S-03", "S-04"]);
  });
});

// ---------------------------------------------------------------------------
// 4. Fusion / fission
// ---------------------------------------------------------------------------

function fusionAttr(el: Element): string | null {
  return el.getAttribute("data-fusion");
}
function fissionAttr(el: Element): string | null {
  return el.getAttribute("data-fission");
}

describe("mountMatrixView — threshold auto-initialization", () => {
  test("for N=4 force rows, threshold input gets min=1, max=4, value=2, and the output text matches", () => {
    const { container } = mountFixture();
    const thresholdInput = container.querySelector("[data-threshold-input]");
    const thresholdValue = container.querySelector("[data-threshold-value]");
    if (!(thresholdInput instanceof HTMLInputElement) || !(thresholdValue instanceof HTMLElement)) {
      throw new Error("threshold controls not found");
    }

    expect(thresholdInput.min).toBe("1");
    expect(thresholdInput.max).toBe("4");
    expect(thresholdInput.value).toBe("2");
    expect(thresholdValue.textContent).toBe("2");
  });

  test("for N=3 force rows, value auto-inits to floor(3/2)=1 (min 1 floor)", () => {
    const spec: FixtureSpec = {
      components: ["a"],
      forces: [
        { id: "S-01", search: "s1", rowTotal: 1, coupled: { a: true } },
        { id: "S-02", search: "s2", rowTotal: 1, coupled: {} },
        { id: "S-03", search: "s3", rowTotal: 1, coupled: {} },
      ],
      colTotals: { a: 1 },
    };
    const { container } = mountFixture(spec);
    const thresholdInput = container.querySelector("[data-threshold-input]");
    if (!(thresholdInput instanceof HTMLInputElement)) throw new Error("threshold input not found");
    expect(thresholdInput.min).toBe("1");
    expect(thresholdInput.max).toBe("3");
    expect(thresholdInput.value).toBe("1");
  });
});

describe("mountMatrixView — fusion/fission highlighting", () => {
  test("two components with identical coupling vectors both get data-fusion=1 on every element sharing their data-component, after recomputeFusionFission()", () => {
    const { container, table, handle } = mountFixture();
    handle.recomputeFusionFission();

    const aEls = Array.from(container.querySelectorAll('[data-component="a"]'));
    const bEls = Array.from(container.querySelectorAll('[data-component="b"]'));
    expect(aEls.length).toBeGreaterThan(0);
    expect(bEls.length).toBeGreaterThan(0);
    for (const el of [...aEls, ...bEls]) {
      expect(fusionAttr(el)).toBe("1");
    }

    const cEls = Array.from(container.querySelectorAll('[data-component="c"]'));
    for (const el of cEls) {
      expect(fusionAttr(el)).toBe("0");
    }
    void table;
  });

  test("a component whose footer data-col-total exceeds the current threshold gets data-fission=1; others do not", () => {
    const { container, handle } = mountFixture();
    handle.recomputeFusionFission();
    // threshold auto-inits to 2; c's col-total is 3 (> 2) -> fission; a,b (2, not > 2) and d (0) are not.
    for (const el of Array.from(container.querySelectorAll('[data-component="c"]'))) {
      expect(fissionAttr(el)).toBe("1");
    }
    for (const el of Array.from(container.querySelectorAll('[data-component="a"]'))) {
      expect(fissionAttr(el)).toBe("0");
    }
    for (const el of Array.from(container.querySelectorAll('[data-component="d"]'))) {
      expect(fissionAttr(el)).toBe("0");
    }
  });

  test("[data-fusion-fission-filter] checkbox, when checked, hides every element for a component that's neither fusion nor fission, and un-hides when unchecked", () => {
    const { container, handle } = mountFixture();
    handle.recomputeFusionFission();

    const filterToggle = container.querySelector("[data-fusion-fission-filter]");
    if (!(filterToggle instanceof HTMLInputElement)) throw new Error("filter toggle not found");

    filterToggle.checked = true;
    filterToggle.dispatchEvent(new Event("change", { bubbles: true }));

    // d is neither fusion (differs from all) nor fission (col-total 0, not > 2) -> hidden
    for (const el of Array.from(container.querySelectorAll('[data-component="d"]'))) {
      expect((el as HTMLElement).hidden).toBe(true);
    }
    // a is fusion -> stays visible
    for (const el of Array.from(container.querySelectorAll('[data-component="a"]'))) {
      expect((el as HTMLElement).hidden).toBe(false);
    }
    // c is fission -> stays visible
    for (const el of Array.from(container.querySelectorAll('[data-component="c"]'))) {
      expect((el as HTMLElement).hidden).toBe(false);
    }

    filterToggle.checked = false;
    filterToggle.dispatchEvent(new Event("change", { bubbles: true }));
    for (const el of Array.from(container.querySelectorAll('[data-component="d"]'))) {
      expect((el as HTMLElement).hidden).toBe(false);
    }
  });

  test("moving the threshold slider (dispatching an input event with a new value) recomputes and reapplies fission highlighting live", () => {
    const { container, handle } = mountFixture();
    handle.recomputeFusionFission();

    const thresholdInput = container.querySelector("[data-threshold-input]");
    if (!(thresholdInput instanceof HTMLInputElement)) throw new Error("threshold input not found");

    // Lower threshold to 1: now a and b's col-total (2) also exceeds 1 -> fission.
    thresholdInput.value = "1";
    thresholdInput.dispatchEvent(new Event("input", { bubbles: true }));

    for (const el of Array.from(container.querySelectorAll('[data-component="a"]'))) {
      expect(fissionAttr(el)).toBe("1");
    }
    for (const el of Array.from(container.querySelectorAll('[data-component="b"]'))) {
      expect(fissionAttr(el)).toBe("1");
    }
    // d's col-total is 0, still not > 1
    for (const el of Array.from(container.querySelectorAll('[data-component="d"]'))) {
      expect(fissionAttr(el)).toBe("0");
    }
  });
});

// ---------------------------------------------------------------------------
// 5. Re-invocation after external DOM mutation
// ---------------------------------------------------------------------------

describe("mountMatrixView — recomputeFusionFission stays correct after external DOM mutations", () => {
  test("a newly-appended row and a newly-appended component column are correctly marked after calling recomputeFusionFission()", () => {
    const { container, table, handle } = mountFixture();
    handle.recomputeFusionFission();

    // Simulate matrix-interactions.ts/forms.ts appending a new component column
    // "e" (all-zero across every force row, same as "d") and a new force row
    // "S-05" (also all-zero across every component), mirroring what
    // addComponentColumn/addForceRow would render directly into the DOM.
    const headerRow = table.querySelector("thead tr");
    const footerRow = table.querySelector("tfoot tr");
    const tbody = table.querySelector("tbody");
    if (!(headerRow instanceof HTMLElement) || !(footerRow instanceof HTMLElement) || !(tbody instanceof HTMLElement)) {
      throw new Error("fixture structure missing expected sections");
    }

    const newTh = document.createElement("th");
    newTh.setAttribute("data-component", "e");
    newTh.setAttribute("data-sort-key", "component:e");
    newTh.textContent = "e";
    const totalTh = headerRow.querySelector('[data-sort-key="total"]');
    headerRow.insertBefore(newTh, totalTh);

    const newFooterTd = document.createElement("td");
    newFooterTd.setAttribute("data-component", "e");
    newFooterTd.setAttribute("data-col-total", "0");
    const grandTotalTd = footerRow.querySelector("[data-grand-total]");
    footerRow.insertBefore(newFooterTd, grandTotalTd);

    for (const existingRow of Array.from(tbody.querySelectorAll("tr.force-row"))) {
      const newCell = document.createElement("td");
      newCell.setAttribute("data-residue-cell", "true");
      newCell.setAttribute("data-force-id", existingRow.getAttribute("data-force-id") ?? "");
      newCell.setAttribute("data-component", "e");
      newCell.setAttribute("data-coupled", "0");
      const rowTotalTd = existingRow.querySelector(".sticky-col-right");
      existingRow.insertBefore(newCell, rowTotalTd);
    }

    const newRow = document.createElement("tr");
    newRow.className = "force-row";
    newRow.setAttribute("data-force-id", "S-05");
    newRow.setAttribute("data-search", "S-05 epsilon stressor");
    newRow.setAttribute("data-row-total", "0");
    const newRowLabel = document.createElement("th");
    newRowLabel.setAttribute("data-force-id", "S-05");
    newRow.appendChild(newRowLabel);
    for (const comp of ["a", "b", "c", "d", "e"]) {
      const cell = document.createElement("td");
      cell.setAttribute("data-residue-cell", "true");
      cell.setAttribute("data-force-id", "S-05");
      cell.setAttribute("data-component", comp);
      cell.setAttribute("data-coupled", "0");
      newRow.appendChild(cell);
    }
    tbody.appendChild(newRow);

    handle.recomputeFusionFission();

    // "e" now has an identical (all-zero) coupling vector to "d" -> both fusion candidates.
    for (const el of Array.from(container.querySelectorAll('[data-component="e"]'))) {
      expect(fusionAttr(el)).toBe("1");
      expect(fissionAttr(el)).toBe("0");
    }
    for (const el of Array.from(container.querySelectorAll('[data-component="d"]'))) {
      expect(fusionAttr(el)).toBe("1");
    }
  });
});

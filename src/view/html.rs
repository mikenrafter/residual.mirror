//! Self-contained landscape HTML — snapshot JSON embedded in a single file, no
//! network fetch, no build step. The page stages `residual add …` commands for
//! the operator to copy; it never mutates the ledger.

use anyhow::Result;

use crate::view::snapshot::{
    DefenseSection, ForceKind, LandscapeSnapshot, SnapshotAttractor, SnapshotComponent,
    SnapshotForce, SnapshotMetaForce, SnapshotResidue,
};

/// The single-file page shell. Rendering substitutes [`SNAPSHOT_PLACEHOLDER`].
pub const TEMPLATE: &str = include_str!("template.html");

/// Token in [`TEMPLATE`] replaced by the serialized snapshot.
pub const SNAPSHOT_PLACEHOLDER: &str = "{{SNAPSHOT_JSON}}";

/// Element id of the `<script type="application/json">` block holding the snapshot.
pub const SNAPSHOT_ELEMENT_ID: &str = "residual-snapshot";

const MATRIX_PLACEHOLDER: &str = "{{MATRIX}}";
const DEFENSE_FORMS_PLACEHOLDER: &str = "{{DEFENSE_FORMS}}";
const DEFENSE_RECORDS_PLACEHOLDER: &str = "{{DEFENSE_RECORDS}}";

/// Render the full landscape page for a snapshot.
pub fn render_landscape_html(snapshot: &LandscapeSnapshot) -> Result<String> {
    let json = escape_script_json(&snapshot.to_json()?);
    let matrix = render_matrix(snapshot);
    let defense_records = match &snapshot.defense {
        Some(defense) => render_defense_records(defense),
        None => String::new(),
    };
    let defense_forms = match &snapshot.defense {
        Some(defense) => render_defense_forms(defense),
        None => String::new(),
    };

    let html = TEMPLATE
        .replace(SNAPSHOT_PLACEHOLDER, &json)
        .replace(MATRIX_PLACEHOLDER, &matrix)
        .replace(DEFENSE_RECORDS_PLACEHOLDER, &defense_records)
        .replace(DEFENSE_FORMS_PLACEHOLDER, &defense_forms);
    Ok(html)
}

/// Extract the embedded snapshot JSON from rendered HTML (round-trip helper).
pub fn extract_snapshot_json(html: &str) -> Option<&str> {
    let open = r#"<script id="residual-snapshot" type="application/json">"#;
    let start = html.find(open)? + open.len();
    let end = html[start..].find("</script>")? + start;
    Some(&html[start..end])
}

/// Prevent ledger text from closing the embedding `<script>` block.
fn escape_script_json(json: &str) -> String {
    json.replace('<', r"\u003c")
}

fn esc(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn force_kind_attr(kind: ForceKind) -> &'static str {
    match kind {
        ForceKind::Stressor => "stressor",
        ForceKind::Purpose => "purpose",
    }
}

/// `S-<shortname>` / `P-<shortname>` — the operator-facing label for a force.
fn force_label(force: &SnapshotForce) -> String {
    let prefix = match force.kind {
        ForceKind::Stressor => "S",
        ForceKind::Purpose => "P",
    };
    format!("{prefix}-{}", force.shortname)
}

/// Bucket an open-ended status string into the small set of known lifecycle
/// stages the UI styles distinctly; anything else falls back to "other" but
/// keeps its raw text in the tooltip.
fn status_bucket(status: &str) -> &'static str {
    match status {
        "proposed" => "proposed",
        "actual" => "actual",
        "deprecated" => "deprecated",
        _ => "other",
    }
}

fn status_tooltip(status: &str) -> String {
    match status {
        "proposed" => "proposed — staged in the ledger, not yet built".to_string(),
        "actual" => "actual — implemented and present in the codebase".to_string(),
        "deprecated" => "deprecated — superseded, scheduled for removal".to_string(),
        other => format!("{other} — non-standard status"),
    }
}

fn row_total(force_id: &str, residues: &[SnapshotResidue]) -> usize {
    residues
        .iter()
        .filter(|r| r.force_id == force_id && r.coupled)
        .count()
}

fn col_total(component: &str, residues: &[SnapshotResidue]) -> usize {
    residues
        .iter()
        .filter(|r| r.component_id == component && r.coupled)
        .count()
}

fn architecture_sets_for_force(
    force_id: &str,
    components: &[SnapshotComponent],
    residues: &[SnapshotResidue],
) -> String {
    let mut sets: Vec<&str> = residues
        .iter()
        .filter(|r| r.force_id == force_id && r.coupled)
        .filter_map(|r| {
            components
                .iter()
                .find(|c| c.name == r.component_id)
                .map(|c| c.architecture_set.as_str())
        })
        .collect();
    sets.sort_unstable();
    sets.dedup();
    sets.join(" ")
}

/// Single unified table: sticky-left force column (accordion for full force
/// detail), sticky-top component header row (status dot + tooltip), sticky-
/// right row-totals column, sticky-bottom column-totals footer. Replaces the
/// old side-by-side force list + component chips + bare matrix.
fn render_matrix(snapshot: &LandscapeSnapshot) -> String {
    let forces: Vec<&SnapshotForce> = snapshot
        .stressors
        .iter()
        .chain(snapshot.purposes.iter())
        .collect();

    let mut out = String::from(r#"<table class="matrix"><thead><tr>"#);
    out.push_str(
        r#"<th class="sticky-col sticky-row corner" data-sort-key="force" tabindex="0" role="button">force</th>"#,
    );
    for component in &snapshot.components {
        let bucket = status_bucket(&component.status);
        let tooltip = status_tooltip(&component.status);
        out.push_str(&format!(
            r#"<th class="sticky-row" data-component="{name}" data-status="{status}" data-architecture-set="{arch}" data-sort-key="component:{name}" title="{tooltip}" tabindex="0" role="button"><span class="status-dot status-{bucket}"></span>{name}</th>"#,
            name = esc(&component.name),
            status = esc(&component.status),
            arch = esc(&component.architecture_set),
            tooltip = esc(&tooltip),
            bucket = bucket,
        ));
    }
    out.push_str(
        r#"<th class="sticky-row sticky-col-right corner" data-sort-key="total">total</th>"#,
    );
    out.push_str("</tr></thead><tbody>");

    for force in &forces {
        let kind = force_kind_attr(force.kind);
        let arch = architecture_sets_for_force(&force.id, &snapshot.components, &snapshot.residues);
        let search = format!(
            "{} {} {} {} {}",
            force.id, force.shortname, kind, force.attractor_id, arch
        );
        let label = force_label(force);
        let total = row_total(&force.id, &snapshot.residues);
        let attractor_label = snapshot
            .attractors
            .iter()
            .find(|a| a.id == force.attractor_id)
            .map(|a| format!("{} · {}", a.id, a.name))
            .unwrap_or_else(|| force.attractor_id.clone());

        out.push_str(&format!(
            r#"<tr class="force-row" data-force-id="{id}" data-force-kind="{kind}" data-architecture-set="{arch}" data-attractor-id="{attractor_id}" data-search="{search}" data-row-total="{total}">"#,
            id = esc(&force.id),
            kind = kind,
            arch = esc(&arch),
            attractor_id = esc(&force.attractor_id),
            search = esc(&search),
            total = total,
        ));
        out.push_str(&format!(
            r#"<th class="sticky-col" data-force-id="{id}"><button type="button" class="force-accordion-toggle" data-accordion-toggle aria-expanded="false">{label}</button><div class="force-detail" hidden><dl><dt>id</dt><dd>{id}</dd><dt>attractor</dt><dd>{attractor}</dd><dt>description</dt><dd>{desc}</dd><dt>naive change</dt><dd>{naive}</dd><dt>outcomes</dt><dd>{outcomes}</dd></dl></div></th>"#,
            id = esc(&force.id),
            label = esc(&label),
            attractor = esc(&attractor_label),
            desc = esc(&force.description),
            naive = esc(&force.naive_change),
            outcomes = esc(&force.outcomes),
        ));

        for component in &snapshot.components {
            let residue = snapshot
                .residues
                .iter()
                .find(|r| r.force_id == force.id && r.component_id == component.name);
            let coupled = residue.map(|r| r.coupled).unwrap_or(false);
            let coupled_attr = if coupled { "1" } else { "0" };
            let body = if coupled { "1" } else { "" };
            out.push_str(&format!(
                r#"<td data-residue-cell data-force-id="{force}" data-component="{comp}" data-coupled="{coupled}">{body}</td>"#,
                force = esc(&force.id),
                comp = esc(&component.name),
                coupled = coupled_attr,
                body = body,
            ));
        }
        out.push_str(&format!(
            r#"<td class="sticky-col-right" data-row-total="{total}">{total}</td>"#,
            total = total
        ));
        out.push_str("</tr>");
    }
    out.push_str("</tbody><tfoot><tr>");
    out.push_str(r#"<th class="sticky-col corner">totals</th>"#);
    let mut grand_total = 0usize;
    for component in &snapshot.components {
        let total = col_total(&component.name, &snapshot.residues);
        grand_total += total;
        out.push_str(&format!(
            r#"<td data-col-total="{total}" data-component="{name}">{total}</td>"#,
            total = total,
            name = esc(&component.name),
        ));
    }
    out.push_str(&format!(
        r#"<td class="sticky-col-right corner" data-grand-total="{gt}">{gt}</td>"#,
        gt = grand_total
    ));
    out.push_str("</tr></tfoot></table>");
    out
}

fn render_defense_records(defense: &DefenseSection) -> String {
    let mut out = String::from(r#"<div class="defense-records"><h3>Defense</h3><ul>"#);
    for force in defense
        .meta_stressors
        .iter()
        .chain(defense.meta_purposes.iter())
    {
        out.push_str(&render_meta_force_item(force));
    }
    for attractor in &defense.meta_attractors {
        out.push_str(&render_meta_attractor_item(attractor));
    }
    for name in &defense.personas {
        out.push_str(&format!("<li>persona · {}</li>", esc(name)));
    }
    for name in &defense.strategies {
        out.push_str(&format!("<li>strategy · {}</li>", esc(name)));
    }
    for name in &defense.progress {
        out.push_str(&format!("<li>progress · {}</li>", esc(name)));
    }
    for name in &defense.pitches {
        out.push_str(&format!("<li>pitch · {}</li>", esc(name)));
    }
    out.push_str("</ul></div>");
    out
}

fn render_meta_force_item(force: &SnapshotMetaForce) -> String {
    format!(
        "<li>{} · {} — {}</li>",
        esc(&force.id),
        esc(&force.shortname),
        esc(&force.description)
    )
}

fn render_meta_attractor_item(attractor: &SnapshotAttractor) -> String {
    format!(
        "<li>{} · {} — {}</li>",
        esc(&attractor.id),
        esc(&attractor.name),
        esc(&attractor.description)
    )
}

fn render_defense_forms(_defense: &DefenseSection) -> String {
    // Presence of the defense section gates the forms; content is operator-staged.
    r#"
          <form data-command-generator="meta-stressor">
            <fieldset>
              <legend>meta-stressor</legend>
              <label>description</label>
              <input name="description" type="text" required />
              <label>shortname</label>
              <input name="shortname" type="text" />
              <button type="submit">stage</button>
            </fieldset>
          </form>
          <form data-command-generator="meta-attractor">
            <fieldset>
              <legend>meta-attractor</legend>
              <label>name</label>
              <input name="name" type="text" required />
              <label>description</label>
              <input name="description" type="text" required />
              <label>positive-state</label>
              <input name="positive_state" type="text" required />
              <label>negative-state</label>
              <input name="negative_state" type="text" required />
              <button type="submit">stage</button>
            </fieldset>
          </form>
          <form data-command-generator="meta-purpose">
            <fieldset>
              <legend>meta-purpose</legend>
              <label>description</label>
              <input name="description" type="text" required />
              <label>attractor-id</label>
              <input name="attractor_id" type="text" required />
              <label>naive-change</label>
              <input name="naive_change" type="text" required />
              <label>shortname</label>
              <input name="shortname" type="text" />
              <label>outcomes</label>
              <input name="outcomes" type="text" />
              <button type="submit">stage</button>
            </fieldset>
          </form>
          <form data-command-generator="defense-persona">
            <fieldset>
              <legend>defense-persona</legend>
              <label>name</label>
              <input name="name" type="text" required />
              <label>body</label>
              <textarea name="body" required></textarea>
              <button type="submit">stage</button>
            </fieldset>
          </form>
          <form data-command-generator="defense-strategy">
            <fieldset>
              <legend>defense-strategy</legend>
              <label>name</label>
              <input name="name" type="text" required />
              <label>body</label>
              <textarea name="body" required></textarea>
              <button type="submit">stage</button>
            </fieldset>
          </form>
          <form data-command-generator="defense-progress">
            <fieldset>
              <legend>defense-progress</legend>
              <label>name</label>
              <input name="name" type="text" required />
              <label>body</label>
              <textarea name="body" required></textarea>
              <button type="submit">stage</button>
            </fieldset>
          </form>
          <form data-command-generator="defense-pitch">
            <fieldset>
              <legend>defense-pitch</legend>
              <label>name</label>
              <input name="name" type="text" required />
              <label>body</label>
              <textarea name="body" required></textarea>
              <button type="submit">stage</button>
            </fieldset>
          </form>
"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::snapshot::{ForceKind, LandscapeSnapshot, RecordSource, SnapshotForce};
    use crate::view::testing::{fixture_defense_snapshot, fixture_main_snapshot};

    fn html(include_defense: bool) -> String {
        let snap = if include_defense {
            fixture_defense_snapshot()
        } else {
            fixture_main_snapshot()
        };
        render_landscape_html(&snap).expect("landscape html renders")
    }

    /// The page carries its own data — no fetch, no sidecar files at view time.
    #[test]
    fn html_embeds_snapshot_json_in_a_script_block() {
        let out = html(false);
        assert!(
            out.contains(r#"<script id="residual-snapshot" type="application/json">"#),
            "snapshot must be embedded as an application/json script block"
        );
        assert!(out.contains("S-01"), "embedded snapshot must carry force ids");
        assert!(out.contains("queue-overload"), "shortnames must be embedded");

        let json = extract_snapshot_json(&out).expect("snapshot json must be extractable");
        let parsed: LandscapeSnapshot =
            serde_json::from_str(json).expect("embedded snapshot must be valid LandscapeSnapshot json");
        assert_eq!(parsed.stressors.len(), 1);
        assert_eq!(parsed.stressors[0].id, "S-01");
    }

    /// Force rows live inside the matrix table itself (no separate force
    /// list), addressable and filterable via the same data hooks as before.
    #[test]
    fn html_matrix_rows_carry_filterable_force_data() {
        let out = html(false);
        assert!(
            out.contains("data-force-filter"),
            "the toolbar filter input must be present"
        );
        assert!(
            out.contains(r#"data-force-id="S-01""#),
            "force rows must be addressable by force id"
        );
        assert!(
            out.contains(r#"data-force-kind="stressor""#),
            "force rows must expose kind for filtering"
        );
        assert!(
            out.contains(r#"data-architecture-set="iter1""#),
            "architecture_set must be exposed for filtering"
        );
        assert!(
            out.contains(r#"data-attractor-id="A-01""#),
            "force rows must expose their attractor id"
        );
    }

    /// Forces render as `S-<shortname>` / `P-<shortname>` accordion toggles,
    /// with full detail (description/naive-change/outcomes/attractor) folded
    /// into a hidden panel rather than a separate list outside the table.
    #[test]
    fn html_force_rows_are_accordions_labeled_by_shortname() {
        let out = html(false);
        assert!(
            out.contains(">S-queue-overload<"),
            "stressor label must be S-<shortname>, got: {out}"
        );
        assert!(
            out.contains(">P-landscape-view<"),
            "purpose label must be P-<shortname>"
        );
        assert!(
            out.contains("data-accordion-toggle"),
            "force label must be an accordion toggle"
        );
        assert!(
            out.contains(r#"class="force-detail" hidden"#),
            "force detail must be present but collapsed by default"
        );
        assert!(
            out.contains("queue overload") && out.contains("add retry") && out.contains("requests drain"),
            "force detail must carry description/naive_change/outcomes"
        );
    }

    /// Row totals (rightmost sticky column) and column totals (sticky
    /// footer) are computed from coupled residues.
    #[test]
    fn html_matrix_has_row_and_column_totals() {
        let out = html(false);
        assert!(
            out.contains(r#"data-row-total="1""#),
            "S-01 couples to exactly one component (auth), row total must be 1"
        );
        assert!(
            out.contains("<tfoot>"),
            "column totals must render as a sticky table footer"
        );
        assert!(
            out.contains(r#"data-col-total="1" data-component="auth""#),
            "auth column total must be 1"
        );
        assert!(
            out.contains(r#"data-col-total="0" data-component="db""#),
            "db column total must be 0 (no coupling)"
        );
        assert!(
            out.contains("data-grand-total"),
            "the bottom-right corner cell must carry the grand total"
        );
    }

    /// Sticky-positioning hooks: leftmost/rightmost columns and header/footer.
    #[test]
    fn html_matrix_carries_sticky_hooks() {
        let out = html(false);
        assert!(out.contains("sticky-col"), "leftmost column must be sticky");
        assert!(out.contains("sticky-col-right"), "rightmost column must be sticky");
        assert!(out.contains("sticky-row"), "header row must be sticky");
        assert!(out.contains("<tfoot>"), "footer row exists to be sticky");
    }

    /// Column headers are click-sortable and carry component status metadata
    /// (styled distinctly, tooltip explains the stage) for every component,
    /// proposed included.
    #[test]
    fn html_matrix_headers_carry_sort_and_status_metadata() {
        let out = html(false);
        assert!(
            out.contains(r#"data-sort-key="force""#),
            "leftmost header must be sortable"
        );
        assert!(
            out.contains(r#"data-sort-key="component:auth""#),
            "component headers must be sortable"
        );
        assert!(
            out.contains(r#"data-sort-key="total""#),
            "totals header must be sortable"
        );
        assert!(
            out.contains(r#"data-status="actual""#),
            "component headers must expose raw status"
        );
        assert!(
            out.contains("status-dot status-actual"),
            "component status must be styled distinctly"
        );
        assert!(
            out.contains("title=\"actual — implemented and present in the codebase\""),
            "hovering a component header must explain its status via a tooltip"
        );
    }

    /// The filter input is not just markup — real JS must wire it to the
    /// data-search attribute on matrix rows so typing actually hides/shows
    /// rows in the table (the markup-only test above doesn't cover this).
    #[test]
    fn template_wires_working_filter_js_to_matrix_rows() {
        assert!(
            TEMPLATE.contains(r#"querySelector("[data-force-filter]")"#),
            "template must look up the filter input by its data-force-filter hook"
        );
        assert!(
            TEMPLATE.contains(r#".addEventListener("input""#),
            "filter input must react live as the operator types"
        );
        assert!(
            TEMPLATE.contains(r#"querySelectorAll("table.matrix tbody tr.force-row")"#),
            "filter must target matrix table rows"
        );
        assert!(
            TEMPLATE.contains(r#"getAttribute("data-search")"#),
            "filter must read the per-row data-search haystack (which embeds kind and architecture_set)"
        );
        assert!(
            TEMPLATE.contains("row.hidden ="),
            "filter must actually toggle row visibility, not just compute a match"
        );
    }

    /// Click-to-sort JS: header clicks reorder tbody rows by force id,
    /// row total, or a specific component's coupling.
    #[test]
    fn template_wires_click_to_sort_on_matrix_headers() {
        assert!(
            TEMPLATE.contains(r#"querySelectorAll("table.matrix thead th[data-sort-key]")"#),
            "template must attach click handlers to sortable headers"
        );
        assert!(
            TEMPLATE.contains("function sortMatrixBy"),
            "template must define the sort function"
        );
        assert!(
            TEMPLATE.contains("tbody.appendChild(row)"),
            "sort must actually reorder DOM rows"
        );
    }

    /// Force accordion toggle JS: clicking the S-/P- label expands the
    /// hidden detail panel in place, no separate list involved.
    #[test]
    fn template_wires_force_accordion_toggle() {
        assert!(
            TEMPLATE.contains(r#"querySelectorAll("[data-accordion-toggle]")"#),
            "template must wire up accordion toggles"
        );
        assert!(
            TEMPLATE.contains(r#"detail.hidden = expanded"#),
            "toggle must flip the detail panel's hidden state"
        );
    }

    /// Fusion/fission candidates are recomputed client-side from the
    /// rendered coupling cells, filterable, and the threshold is a live
    /// slider (not baked into the server-rendered page).
    #[test]
    fn template_wires_live_fusion_fission_threshold() {
        assert!(
            TEMPLATE.contains("data-fusion-fission-filter"),
            "template must expose a fusion/fission-only filter control"
        );
        assert!(
            TEMPLATE.contains("data-threshold-input"),
            "template must expose a live threshold slider"
        );
        assert!(
            TEMPLATE.contains("function computeMatrixCandidates"),
            "template must compute fusion/fission candidates client-side"
        );
        assert!(
            TEMPLATE.contains(r#"thresholdInput.addEventListener("input""#),
            "threshold slider must recompute candidates live as it moves"
        );
    }

    /// End-to-end: rendered rows carry exactly the data the template's filter JS
    /// reads, so a filter matching kind/architecture_set actually has something to match.
    #[test]
    fn html_filter_js_and_rendered_rows_share_the_same_data_hooks() {
        let out = html(false);
        assert!(
            out.contains(r#"data-search="S-01 queue-overload stressor A-01 iter1""#),
            "rendered data-search must embed id, shortname, kind, attractor, and architecture_set for the JS filter to match on"
        );
    }

    /// Matrix / heatmap affordance backed by data attributes.
    #[test]
    fn html_contains_residue_matrix_with_cell_data_attributes() {
        let out = html(false);
        assert!(out.contains(r#"data-view="matrix""#), "matrix view is missing");
        assert!(
            out.contains("data-residue-cell"),
            "matrix cells must be tagged data-residue-cell"
        );
        assert!(
            out.contains(r#"data-component="auth""#),
            "component columns must be addressable"
        );
        assert!(
            out.contains(r#"data-coupled="1""#),
            "the S-01 × auth coupling must render as a coupled cell"
        );
    }

    /// Staging forms + copy surface + the JS that builds commands.
    #[test]
    fn html_contains_command_generator_and_copy_surface() {
        let out = html(false);

        for target in ["stressor", "purpose", "attractor", "component"] {
            assert!(
                out.contains(&format!(r#"data-command-generator="{target}""#)),
                "missing staging form for {target}"
            );
        }
        assert!(
            out.contains("data-copy-commands"),
            "copy-commands surface is missing"
        );
        assert!(
            out.contains(r#"id="staged-commands""#),
            "staged command output element is missing"
        );
        assert!(
            out.contains("function buildAddCommand"),
            "JS command builder buildAddCommand is missing"
        );
        assert!(
            out.contains("function shellQuote"),
            "JS shell quoting helper shellQuote is missing"
        );
        assert!(
            out.contains("residual add "),
            "generated commands must be `residual add …`"
        );
        for flag in [
            "--description",
            "--attractor-id",
            "--naive-change",
            "--shortname",
            "--outcomes",
            "--positive-state",
            "--negative-state",
            "--architecture-set",
            "--status",
        ] {
            assert!(out.contains(flag), "command generator must emit {flag}");
        }
    }

    /// The UI is read-only against the ledger: it stages text, it does not write.
    #[test]
    fn html_command_generator_does_not_mutate_the_ledger() {
        let out = html(false);
        for forbidden in ["fetch(", "XMLHttpRequest", "navigator.sendBeacon"] {
            assert!(
                !out.contains(forbidden),
                "landscape UI must not call back into anything ({forbidden} found)"
            );
        }
    }

    /// Defense forms only exist in a defense landscape.
    #[test]
    fn html_omits_defense_forms_and_records_without_defense() {
        let out = html(false);
        for target in [
            "meta-stressor",
            "meta-attractor",
            "meta-purpose",
            "defense-persona",
            "defense-strategy",
            "defense-progress",
            "defense-pitch",
        ] {
            assert!(
                !out.contains(&format!(r#"data-command-generator="{target}""#)),
                "defense staging form for {target} must be absent without --defense"
            );
        }
        for needle in ["MS-01", "MA-01", "MP-01", "hostile-auditor", "alpha-strategy"] {
            assert!(
                !out.contains(needle),
                "defense record {needle} leaked into a non-defense landscape"
            );
        }
    }

    #[test]
    fn html_includes_defense_forms_and_records_with_defense() {
        let out = html(true);
        for target in [
            "meta-stressor",
            "meta-attractor",
            "meta-purpose",
            "defense-persona",
            "defense-strategy",
            "defense-progress",
            "defense-pitch",
        ] {
            assert!(
                out.contains(&format!(r#"data-command-generator="{target}""#)),
                "defense staging form for {target} must be present with --defense"
            );
        }
        assert!(out.contains("MS-01"), "meta-stressor must render with --defense");
        assert!(out.contains("MA-01"), "meta-attractor must render with --defense");
        assert!(out.contains("MP-01"), "meta-purpose must render with --defense");
        assert!(
            out.contains("hostile-auditor"),
            "defense persona must render with --defense"
        );
    }

    /// Ledger text must not be able to close the embedded script block.
    #[test]
    fn html_escapes_script_terminator_in_snapshot_values() {
        let mut snap = fixture_main_snapshot();
        snap.stressors = vec![SnapshotForce {
            id: "S-01".into(),
            shortname: "xss".into(),
            description: "</script><script>alert(1)</script>".into(),
            naive_change: "add retry".into(),
            outcomes: "drain".into(),
            attractor_id: "A-01".into(),
            kind: ForceKind::Stressor,
            source: RecordSource::Sidecar,
        }];

        let out = render_landscape_html(&snap).expect("landscape html renders");
        assert!(
            !out.contains("<script>alert(1)</script>"),
            "ledger text must not escape the embedded snapshot script block"
        );
    }

    /// The template asset itself is the JS home — keep it wired, not empty.
    #[test]
    fn template_asset_carries_placeholder_and_generator_js() {
        assert!(
            TEMPLATE.contains(SNAPSHOT_PLACEHOLDER),
            "template must contain the snapshot JSON substitution token"
        );
        assert!(
            TEMPLATE.contains(SNAPSHOT_ELEMENT_ID),
            "template must contain the snapshot script element id"
        );
        assert!(
            TEMPLATE.contains("function buildAddCommand"),
            "template must define buildAddCommand"
        );
        assert!(
            TEMPLATE.contains("function shellQuote"),
            "template must define shellQuote"
        );
        assert!(
            TEMPLATE.contains("data-copy-commands"),
            "template must define the copy-commands surface"
        );
    }
}

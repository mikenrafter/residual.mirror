//! Static-export landscape rendering — replaces the old `html.rs` string
//! templating with Dioxus-rendered fragments spliced into the same template
//! shell (`shell.html`).

use anyhow::Result;

use crate::view::components::defense::{render_defense_forms, render_defense_records};
use crate::view::components::matrix::render_matrix;
use crate::view::snapshot::LandscapeSnapshot;

pub(crate) const TEMPLATE: &str = include_str!("shell.html");

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
    json.replace('<', "\\u003c")
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

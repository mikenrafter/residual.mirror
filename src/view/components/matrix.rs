//! Matrix rendering — pure helpers moved verbatim from `src/view/html.rs`
//! (no Dioxus dependency, mechanical move only), plus the Dioxus-based
//! `render_matrix` itself.

use dioxus::prelude::*;

use crate::view::snapshot::{ForceKind, LandscapeSnapshot, SnapshotComponent, SnapshotForce, SnapshotResidue};

pub(crate) fn force_kind_attr(kind: ForceKind) -> &'static str {
    match kind {
        ForceKind::Stressor => "stressor",
        ForceKind::Purpose => "purpose",
    }
}

/// `S-<shortname>` / `P-<shortname>` — the operator-facing label for a force.
pub(crate) fn force_label(force: &SnapshotForce) -> String {
    let prefix = match force.kind {
        ForceKind::Stressor => "S",
        ForceKind::Purpose => "P",
    };
    format!("{prefix}-{}", force.shortname)
}

/// Bucket an open-ended status string into the small set of known lifecycle
/// stages the UI styles distinctly; anything else falls back to "other" but
/// keeps its raw text in the tooltip.
pub(crate) fn status_bucket(status: &str) -> &'static str {
    match status {
        "proposed" => "proposed",
        "actual" => "actual",
        "deprecated" => "deprecated",
        _ => "other",
    }
}

pub(crate) fn status_tooltip(status: &str) -> String {
    match status {
        "proposed" => "proposed — staged in the ledger, not yet built".to_string(),
        "actual" => "actual — implemented and present in the codebase".to_string(),
        "deprecated" => "deprecated — superseded, scheduled for removal".to_string(),
        other => format!("{other} — non-standard status"),
    }
}

pub(crate) fn row_total(force_id: &str, residues: &[SnapshotResidue]) -> usize {
    residues
        .iter()
        .filter(|r| r.force_id == force_id && r.coupled)
        .count()
}

pub(crate) fn col_total(component: &str, residues: &[SnapshotResidue]) -> usize {
    residues
        .iter()
        .filter(|r| r.component_id == component && r.coupled)
        .count()
}

pub(crate) fn architecture_sets_for_force(
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

/// Precomputed per-component header cell data (avoids recomputing inside the
/// `rsx!` for-loop body, which only accepts markup, not arbitrary statements).
struct HeaderCol<'a> {
    name: &'a str,
    status: &'a str,
    architecture_set: &'a str,
    tooltip: String,
    bucket: &'static str,
}

/// Precomputed per-(force, component) coupling cell data.
struct MatrixCell<'a> {
    component_name: &'a str,
    coupled_attr: &'static str,
    body: &'static str,
}

/// Precomputed per-force row data.
struct ForceRow<'a> {
    id: &'a str,
    kind: &'static str,
    arch: String,
    attractor_id: &'a str,
    search: String,
    total: usize,
    label: String,
    attractor_label: String,
    description: &'a str,
    naive_change: &'a str,
    outcomes: &'a str,
    cells: Vec<MatrixCell<'a>>,
}

/// Precomputed per-component footer total.
struct FooterCol<'a> {
    name: &'a str,
    total: usize,
}

/// Single unified table: sticky-left force column (accordion for full force
/// detail), sticky-top component header row (status dot + tooltip), sticky-
/// right row-totals column, sticky-bottom column-totals footer.
pub fn render_matrix(snapshot: &LandscapeSnapshot) -> String {
    let forces: Vec<&SnapshotForce> = snapshot
        .stressors
        .iter()
        .chain(snapshot.purposes.iter())
        .collect();

    let header_cols: Vec<HeaderCol> = snapshot
        .components
        .iter()
        .map(|component| HeaderCol {
            name: &component.name,
            status: &component.status,
            architecture_set: &component.architecture_set,
            tooltip: status_tooltip(&component.status),
            bucket: status_bucket(&component.status),
        })
        .collect();

    let force_rows: Vec<ForceRow> = forces
        .iter()
        .map(|force| {
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

            let cells: Vec<MatrixCell> = snapshot
                .components
                .iter()
                .map(|component| {
                    let residue = snapshot
                        .residues
                        .iter()
                        .find(|r| r.force_id == force.id && r.component_id == component.name);
                    let coupled = residue.map(|r| r.coupled).unwrap_or(false);
                    MatrixCell {
                        component_name: &component.name,
                        coupled_attr: if coupled { "1" } else { "0" },
                        body: if coupled { "1" } else { "" },
                    }
                })
                .collect();

            ForceRow {
                id: &force.id,
                kind,
                arch,
                attractor_id: &force.attractor_id,
                search,
                total,
                label,
                attractor_label,
                description: &force.description,
                naive_change: &force.naive_change,
                outcomes: &force.outcomes,
                cells,
            }
        })
        .collect();

    let footer_cols: Vec<FooterCol> = snapshot
        .components
        .iter()
        .map(|component| FooterCol {
            name: &component.name,
            total: col_total(&component.name, &snapshot.residues),
        })
        .collect();
    let grand_total: usize = footer_cols.iter().map(|c| c.total).sum();

    let element = rsx! {
        table { class: "matrix",
            thead {
                tr {
                    th {
                        class: "sticky-col sticky-row corner",
                        "data-sort-key": "force",
                        tabindex: "0",
                        role: "button",
                        "force"
                    }
                    for col in header_cols.iter() {
                        th {
                            class: "sticky-row",
                            "data-component": "{col.name}",
                            "data-status": "{col.status}",
                            "data-architecture-set": "{col.architecture_set}",
                            "data-sort-key": "component:{col.name}",
                            title: "{col.tooltip}",
                            tabindex: "0",
                            role: "button",
                            span { class: "status-dot status-{col.bucket}" }
                            "{col.name}"
                        }
                    }
                    th {
                        class: "sticky-row sticky-col-right corner",
                        "data-sort-key": "total",
                        "total"
                    }
                }
            }
            tbody {
                for row in force_rows.iter() {
                    tr {
                        class: "force-row",
                        "data-force-id": "{row.id}",
                        "data-force-kind": "{row.kind}",
                        "data-architecture-set": "{row.arch}",
                        "data-attractor-id": "{row.attractor_id}",
                        "data-search": "{row.search}",
                        "data-row-total": "{row.total}",
                        th {
                            class: "sticky-col",
                            "data-force-id": "{row.id}",
                            button {
                                r#type: "button",
                                class: "force-accordion-toggle",
                                "data-accordion-toggle": "true",
                                "aria-expanded": "false",
                                "{row.label}"
                            }
                            div {
                                class: "force-detail",
                                hidden: "true",
                                dl {
                                    dt { "id" }
                                    dd { "{row.id}" }
                                    dt { "attractor" }
                                    dd { "{row.attractor_label}" }
                                    dt { "description" }
                                    dd { "{row.description}" }
                                    dt { "naive change" }
                                    dd { "{row.naive_change}" }
                                    dt { "outcomes" }
                                    dd { "{row.outcomes}" }
                                }
                            }
                        }
                        for cell in row.cells.iter() {
                            td {
                                "data-residue-cell": "true",
                                "data-force-id": "{row.id}",
                                "data-component": "{cell.component_name}",
                                "data-coupled": "{cell.coupled_attr}",
                                "{cell.body}"
                            }
                        }
                        td {
                            class: "sticky-col-right",
                            "data-row-total": "{row.total}",
                            "{row.total}"
                        }
                    }
                }
            }
            tfoot {
                tr {
                    th { class: "sticky-col corner", "totals" }
                    for col in footer_cols.iter() {
                        td {
                            "data-col-total": "{col.total}",
                            "data-component": "{col.name}",
                            "{col.total}"
                        }
                    }
                    td {
                        class: "sticky-col-right corner",
                        "data-grand-total": "{grand_total}",
                        "{grand_total}"
                    }
                }
            }
        }
    };

    dioxus_ssr::render_element(element)
}

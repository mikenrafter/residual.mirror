//! Defense-section rendering — Dioxus-based implementation. Individual list
//! items are rendered through Dioxus (so escaping and markup stay consistent
//! with `matrix.rs`); the surrounding `<div>/<ul>` shell is assembled by
//! splicing those already-safe fragments in via `dangerous_inner_html`
//! (they were produced by our own renderer, not raw ledger text).

use dioxus::prelude::*;

use crate::view::snapshot::{DefenseSection, SnapshotAttractor, SnapshotMetaForce};

pub fn render_defense_records(defense: &DefenseSection) -> String {
    let mut items = String::new();
    for force in defense.meta_stressors.iter().chain(defense.meta_purposes.iter()) {
        items.push_str(&render_meta_force_item(force));
    }
    for attractor in &defense.meta_attractors {
        items.push_str(&render_meta_attractor_item(attractor));
    }
    for name in &defense.personas {
        items.push_str(&render_labeled_item("persona", name));
    }
    for name in &defense.strategies {
        items.push_str(&render_labeled_item("strategy", name));
    }
    for name in &defense.progress {
        items.push_str(&render_labeled_item("progress", name));
    }
    for name in &defense.pitches {
        items.push_str(&render_labeled_item("pitch", name));
    }

    dioxus_ssr::render_element(rsx! {
        div { class: "defense-records",
            h3 { "Defense" }
            ul { dangerous_inner_html: "{items}" }
        }
    })
}

pub fn render_meta_force_item(force: &SnapshotMetaForce) -> String {
    dioxus_ssr::render_element(rsx! {
        li { "{force.id} · {force.shortname} — {force.description}" }
    })
}

pub fn render_meta_attractor_item(attractor: &SnapshotAttractor) -> String {
    dioxus_ssr::render_element(rsx! {
        li { "{attractor.id} · {attractor.name} — {attractor.description}" }
    })
}

/// Shared shape for the persona/strategy/progress/pitch list items, which
/// only differ by their label.
fn render_labeled_item(kind: &str, name: &str) -> String {
    dioxus_ssr::render_element(rsx! {
        li { "{kind} · {name}" }
    })
}

pub fn render_defense_forms(_defense: &DefenseSection) -> String {
    // Presence of the defense section gates the forms; content is operator-staged.
    dioxus_ssr::render_element(rsx! {
        form { "data-command-generator": "meta-stressor",
            fieldset {
                legend { "meta-stressor" }
                label { "description" }
                input { name: "description", r#type: "text", required: "required" }
                label { "shortname" }
                input { name: "shortname", r#type: "text" }
                button { r#type: "submit", "stage" }
            }
        }
        form { "data-command-generator": "meta-attractor",
            fieldset {
                legend { "meta-attractor" }
                label { "name" }
                input { name: "name", r#type: "text", required: "required" }
                label { "description" }
                input { name: "description", r#type: "text", required: "required" }
                label { "positive-state" }
                input { name: "positive_state", r#type: "text", required: "required" }
                label { "negative-state" }
                input { name: "negative_state", r#type: "text", required: "required" }
                button { r#type: "submit", "stage" }
            }
        }
        form { "data-command-generator": "meta-purpose",
            fieldset {
                legend { "meta-purpose" }
                label { "description" }
                input { name: "description", r#type: "text", required: "required" }
                label { "attractor-id" }
                input { name: "attractor_id", r#type: "text", required: "required" }
                label { "naive-change" }
                input { name: "naive_change", r#type: "text", required: "required" }
                label { "shortname" }
                input { name: "shortname", r#type: "text" }
                label { "outcomes" }
                input { name: "outcomes", r#type: "text" }
                button { r#type: "submit", "stage" }
            }
        }
        form { "data-command-generator": "defense-persona",
            fieldset {
                legend { "defense-persona" }
                label { "name" }
                input { name: "name", r#type: "text", required: "required" }
                label { "body" }
                textarea { name: "body", required: "required" }
                button { r#type: "submit", "stage" }
            }
        }
        form { "data-command-generator": "defense-strategy",
            fieldset {
                legend { "defense-strategy" }
                label { "name" }
                input { name: "name", r#type: "text", required: "required" }
                label { "body" }
                textarea { name: "body", required: "required" }
                button { r#type: "submit", "stage" }
            }
        }
        form { "data-command-generator": "defense-progress",
            fieldset {
                legend { "defense-progress" }
                label { "name" }
                input { name: "name", r#type: "text", required: "required" }
                label { "body" }
                textarea { name: "body", required: "required" }
                button { r#type: "submit", "stage" }
            }
        }
        form { "data-command-generator": "defense-pitch",
            fieldset {
                legend { "defense-pitch" }
                label { "name" }
                input { name: "name", r#type: "text", required: "required" }
                label { "body" }
                textarea { name: "body", required: "required" }
                button { r#type: "submit", "stage" }
            }
        }
    })
}

use anyhow::Result;
use std::collections::{HashMap, HashSet};

use crate::cli::{MatrixOp, MatrixSortBy};
use crate::config::Config;

pub mod criticality;
pub mod forces;
pub mod matrix;
pub mod residual_index;

fn force_shortnames(residual_dir: &std::path::Path) -> Result<HashMap<String, String>> {
    let mut map = HashMap::new();
    for s in crate::storage::stressors::load(residual_dir)? {
        map.insert(s.id, s.shortname);
    }
    for p in crate::storage::purposes::load(residual_dir)? {
        map.insert(p.id, p.shortname);
    }
    Ok(map)
}

fn attractor_names(residual_dir: &std::path::Path) -> Result<HashMap<String, String>> {
    let attractors = crate::storage::attractors::load(residual_dir)?;
    Ok(attractors.into_iter().map(|a| (a.id, a.name)).collect())
}

fn build_matrix(
    residual_dir: &std::path::Path,
    filter: &[String],
    sort_by: MatrixSortBy,
) -> Result<matrix::NkpMatrix> {
    let shortnames = force_shortnames(residual_dir)?;
    let forces = forces::load_force_meta(residual_dir)?;
    let filtered = matrix::filter_forces(&forces, filter, &shortnames);
    let ordered = matrix::sort_forces(filtered, sort_by, &shortnames);
    let residues = crate::storage::format::read_residues(residual_dir)?;
    let keep: HashSet<String> = ordered.iter().map(|f| f.id.clone()).collect();
    let residues: Vec<_> = residues
        .into_iter()
        .filter(|r| keep.contains(&r.force_id))
        .collect();
    let attractor_by_force = forces::attractor_map(residual_dir)?;
    let mut m = matrix::NkpMatrix::build_from_residues(&residues, &attractor_by_force);
    m.reorder_rows(&ordered);
    if sort_by == MatrixSortBy::FusionFission {
        m.reorder_columns_fusion_fission();
    }
    Ok(m)
}

pub fn run(cfg: &Config, op: MatrixOp) -> Result<()> {
    let dir = crate::storage::metadata_dir(cfg)?;
    match op {
        MatrixOp::Show {
            csv,
            filter,
            sort_by,
        } => {
            let shortnames = force_shortnames(&dir)?;
            let attractor_names = attractor_names(&dir)?;
            let m = build_matrix(&dir, &filter, sort_by)?;
            if csv {
                m.print_csv(&shortnames, &attractor_names, sort_by)?;
            } else {
                m.print_colored(&shortnames, &attractor_names, sort_by);
            }
        }
        MatrixOp::Calc => {
            let m = matrix::NkpMatrix::build_from_dir(&dir)?;
            println!("N (nodes) = {}", m.n());
            println!("K (connections) = {}", m.k());
            println!(
                "K/N = {:.4}",
                if m.n() == 0 {
                    0.0
                } else {
                    m.k() as f64 / m.n() as f64
                }
            );
        }
        MatrixOp::Criticality => {
            let m = matrix::NkpMatrix::build_from_dir(&dir)?;
            let report = criticality::assess(&m);
            println!(
                "N = {}, K = {}, K/N = {:.4}",
                report.n, report.k, report.k_per_n
            );
            println!("Assessment: {}", report.assessment);
        }
        MatrixOp::Ri {
            stressors,
            naive_survived,
            residual_survived,
        } => {
            let ri = residual_index::calculate(naive_survived, residual_survived, stressors);
            let interpretation = residual_index::interpret(ri);
            println!("Ri = {:.4}", ri);
            println!("{}", interpretation);
        }
        MatrixOp::Fusion => {
            let m = matrix::NkpMatrix::build_from_dir(&dir)?;
            let candidates = m.fusion_candidates();
            if candidates.is_empty() {
                println!("No fusion candidates found.");
            } else {
                println!("Fusion candidates (identical stress-response patterns):");
                for (a, b) in &candidates {
                    println!("  {} ↔ {}", a, b);
                }
            }
        }
        MatrixOp::Fission => {
            let m = matrix::NkpMatrix::build_from_dir(&dir)?;
            let threshold = (m.force_ids.len() / 2).max(1);
            let candidates = m.fission_candidates(threshold);
            if candidates.is_empty() {
                println!("No fission candidates found (threshold = {}).", threshold);
            } else {
                println!("Fission candidates (col total > {}):", threshold);
                for comp in &candidates {
                    println!("  {}", comp);
                }
            }
        }
    }
    Ok(())
}

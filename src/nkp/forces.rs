//! Force metadata for NKP matrix ordering — coupling lives in residues.csv only.

use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForceMeta {
    pub id: String,
    pub attractor_id: String,
    pub description: String,
}

pub fn load_force_meta(residual_dir: &Path) -> Result<Vec<ForceMeta>> {
    let mut forces = Vec::new();
    for s in crate::storage::stressors::load(residual_dir)? {
        forces.push(ForceMeta {
            id: s.id,
            attractor_id: s.attractor_id,
            description: s.description,
        });
    }
    for p in crate::storage::purposes::load(residual_dir)? {
        forces.push(ForceMeta {
            id: p.id,
            attractor_id: p.attractor_id,
            description: p.description,
        });
    }
    Ok(forces)
}

pub fn attractor_map(residual_dir: &Path) -> Result<HashMap<String, String>> {
    let mut map = HashMap::new();
    for s in crate::storage::stressors::load(residual_dir)? {
        map.insert(s.id, s.attractor_id);
    }
    for p in crate::storage::purposes::load(residual_dir)? {
        map.insert(p.id, p.attractor_id);
    }
    Ok(map)
}

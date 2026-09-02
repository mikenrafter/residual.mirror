//! Meta-stressors — MS-* namespace in defense/meta-stressors.csv only.

use anyhow::Result;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetaStressor {
    pub id: String,
    pub shortname: String,
    pub description: String,
}

pub fn append(residual_dir: &Path, item: MetaStressor) -> Result<()> {
    let _ = (residual_dir, item);
    todo!("append MS-* to defense/meta-stressors.csv only")
}

pub fn list(residual_dir: &Path) -> Result<Vec<MetaStressor>> {
    let _ = residual_dir;
    todo!("list meta-stressors from defense/meta-stressors.csv")
}

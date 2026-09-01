//! +/- Residues schema — force ↔ component mapping.

pub const WHOLE_SYSTEM_STATUS: &str = "whole-system-residue";
pub const WHOLE_SYSTEM_COMPONENT: &str = "whole-system";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Residue {
    pub id: String,
    pub force_id: String,
    pub component_id: String,
    pub status: String,
    pub notes: String,
}

impl Residue {
    pub fn new(
        id: impl Into<String>,
        force_id: impl Into<String>,
        component_id: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            force_id: force_id.into(),
            component_id: component_id.into(),
            status: String::new(),
            notes: String::new(),
        }
    }

    /// NKP coupling cell in residues.csv (v4).
    pub fn coupling(
        id: impl Into<String>,
        force_id: impl Into<String>,
        component_id: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            force_id: force_id.into(),
            component_id: component_id.into(),
            status: "1".into(),
            notes: String::new(),
        }
    }

    pub fn is_coupled(&self) -> bool {
        if self.component_id.is_empty() {
            return false;
        }
        if self.is_whole_system() {
            return true;
        }
        if self.status == "1" {
            return true;
        }
        if self.status.is_empty() {
            return false;
        }
        // Legacy row/matrix cells: any non-zero/non-empty status marks coupling.
        self.status != "0"
    }

    pub fn whole_system(
        id: impl Into<String>,
        force_id: impl Into<String>,
        notes: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            force_id: force_id.into(),
            component_id: WHOLE_SYSTEM_COMPONENT.into(),
            status: "1".into(),
            notes: notes.into(),
        }
    }

    pub fn is_whole_system(&self) -> bool {
        is_whole_system_residue(self)
    }
}

pub fn naive_change_accepts_whole_system(naive_change: &str) -> bool {
    naive_change
        .to_lowercase()
        .contains(WHOLE_SYSTEM_STATUS)
}

pub fn is_whole_system_residue(residue: &Residue) -> bool {
    residue.component_id == WHOLE_SYSTEM_COMPONENT
}

pub fn tag_naive_change_whole_system(naive_change: &str) -> String {
    if naive_change_accepts_whole_system(naive_change) {
        naive_change.to_string()
    } else {
        format!("{WHOLE_SYSTEM_STATUS}: {naive_change}")
    }
}

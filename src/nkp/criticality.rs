use crate::nkp::matrix::NkpMatrix;

pub struct CriticalityReport {
    pub n: usize,
    pub k: usize,
    pub k_per_n: f64,
    pub assessment: String,
}

pub fn assess(matrix: &NkpMatrix) -> CriticalityReport {
    let n = matrix.n();
    let k = matrix.k();
    let k_per_n = if n == 0 { 0.0 } else { k as f64 / n as f64 };

    let assessment = if k_per_n < 0.5 {
        "under-connected: K/N < 0.5, system has too few connections to exhibit robust emergent behavior".to_string()
    } else if k_per_n <= 3.0 {
        "critical: 0.5 ≤ K/N ≤ 3.0, system is in the zone of criticality (Kauffman K≈2)".to_string()
    } else {
        "over-connected: K/N > 3.0, system has too many connections and may be chaotic".to_string()
    };

    CriticalityReport { n, k, k_per_n, assessment }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nkp::matrix::NkpMatrix;
    use crate::structure::analysis::residues::Residue;
    use std::collections::HashMap;

    fn full_grid(stressor_count: usize, component_count: usize) -> (Vec<Residue>, HashMap<String, String>) {
        let mut residues = Vec::new();
        let mut att = HashMap::new();
        for s in 0..stressor_count {
            let fid = format!("S-{:02}", s + 1);
            att.insert(fid.clone(), "A-01".into());
            for c in 0..component_count {
                residues.push(Residue::coupling(
                    format!("R-{:02}", residues.len() + 1),
                    &fid,
                    format!("c{c}"),
                ));
            }
        }
        (residues, att)
    }

    #[test]
    fn assess_empty_matrix_is_undercritical() {
        let m = NkpMatrix::build_from_residues(&[], &HashMap::new());
        let r = assess(&m);
        assert_eq!(r.k_per_n, 0.0);
        assert!(r.assessment.contains("under-connected"));
    }

    #[test]
    fn assess_critical_zone() {
        let residues = vec![
            Residue::coupling("R-01", "S-01", "auth"),
            Residue::coupling("R-02", "S-01", "db"),
            Residue::coupling("R-03", "S-02", "auth"),
            Residue::coupling("R-04", "S-02", "db"),
        ];
        let att = HashMap::from([
            ("S-01".into(), "A-01".into()),
            ("S-02".into(), "A-01".into()),
        ]);
        let m = NkpMatrix::build_from_residues(&residues, &att);
        let r = assess(&m);
        assert!(r.k_per_n >= 0.5 && r.k_per_n <= 3.0);
        assert!(r.assessment.contains("critical"));
    }

    #[test]
    fn assess_overcritical() {
        let (residues, att) = full_grid(8, 8);
        let m = NkpMatrix::build_from_residues(&residues, &att);
        let r = assess(&m);
        assert!(r.k_per_n > 3.0, "K/N={}", r.k_per_n);
        assert!(r.assessment.contains("over-connected"));
    }

    #[test]
    fn assess_undercritical_single_force_no_couplings() {
        let m = NkpMatrix::build_from_residues(&[], &HashMap::from([("S-01".into(), "A-01".into())]));
        let r = assess(&m);
        assert!(r.k_per_n < 0.5);
        assert!(r.assessment.contains("under-connected"));
    }
}

//! Landscape view — read-only HTML rendering of the residual ledger.
//!
//! `residual view` writes a self-contained page to a temp dir (or `--out`) and
//! prints the path. `residual serve` hands the same page out over loopback.
//! Neither mutates the ledger: the page stages `residual add …` commands that
//! the operator copies and runs.

pub mod commands;
pub mod html;
pub mod snapshot;

use anyhow::{anyhow, Context, Result};
use std::path::PathBuf;

use crate::config::Config;

#[allow(unused_imports)] // re-exports for green agents / external callers
pub use commands::{format_add_command, format_add_commands, StagedAdd};
pub use html::render_landscape_html;
#[allow(unused_imports)] // re-exports for green agents / external callers
pub use snapshot::{
    load_landscape_snapshot, load_landscape_snapshot_from_dir, load_landscape_snapshot_from_sources,
    resolve_sources, LandscapeSnapshot, LandscapeSources,
};

/// Filename written into the output directory.
pub const LANDSCAPE_FILENAME: &str = "landscape.html";

/// Loopback host the landscape binds to. Never 0.0.0.0: the ledger is not public.
pub const SERVE_HOST: &str = "127.0.0.1";

/// Default port for `residual serve` when `--port` is omitted.
pub const DEFAULT_PORT: u16 = 8787;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ViewOptions {
    /// Include the defense ledger (meta forces + defense artifacts).
    pub include_defense: bool,
    /// Destination directory. `None` means the system temp dir (ephemeral, uncommitted).
    pub out_dir: Option<PathBuf>,
}

/// What the server hands out: the page at `/`, the raw snapshot at `/snapshot.json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServeDocument {
    pub html: String,
    pub snapshot_json: String,
}

/// Render the landscape and write it to disk. Returns the written HTML path.
pub fn write_landscape_html(cfg: &Config, opts: &ViewOptions) -> Result<PathBuf> {
    let snap = load_landscape_snapshot(cfg, opts.include_defense)?;
    let html = render_landscape_html(&snap)?;

    let out_dir = match &opts.out_dir {
        Some(dir) => {
            std::fs::create_dir_all(dir)
                .with_context(|| format!("creating landscape out dir {}", dir.display()))?;
            dir.clone()
        }
        None => std::env::temp_dir(),
    };

    let path = absolutize(out_dir.join(LANDSCAPE_FILENAME))?;
    std::fs::write(&path, html)
        .with_context(|| format!("writing landscape html to {}", path.display()))?;
    Ok(path)
}

/// `residual view` entry point — writes the page and prints its path.
pub fn run_view(cfg: &Config, include_defense: bool, out_dir: Option<PathBuf>) -> Result<()> {
    let path = write_landscape_html(
        cfg,
        &ViewOptions {
            include_defense,
            out_dir,
        },
    )?;
    println!("{}", path.display());
    Ok(())
}

/// Printable URL for a bound port.
pub fn serve_url(port: u16) -> String {
    format!("http://{SERVE_HOST}:{port}/")
}

/// Build the served payload without binding a socket.
pub fn build_serve_document(cfg: &Config, include_defense: bool) -> Result<ServeDocument> {
    let snap = load_landscape_snapshot(cfg, include_defense)?;
    let html = render_landscape_html(&snap)?;
    let snapshot_json = snap.to_json()?;
    Ok(ServeDocument {
        html,
        snapshot_json,
    })
}

/// Bind the loopback listener and block. Prints the URL before serving.
pub fn run_serve(cfg: &Config, include_defense: bool, port: u16) -> Result<()> {
    let doc = build_serve_document(cfg, include_defense)?;
    let url = serve_url(port);
    println!("{url}");

    let addr = format!("{SERVE_HOST}:{port}");
    let server = tiny_http::Server::http(&addr)
        .map_err(|e| anyhow!("binding landscape server on {addr}: {e}"))?;

    for request in server.incoming_requests() {
        let path = request.url().split('?').next().unwrap_or("/");
        let response = match path {
            "/" | "" => tiny_http::Response::from_data(doc.html.as_bytes())
                .with_header(content_type("text/html; charset=utf-8")),
            "/snapshot.json" => tiny_http::Response::from_data(doc.snapshot_json.as_bytes())
                .with_header(content_type("application/json; charset=utf-8")),
            _ => tiny_http::Response::from_string("not found")
                .with_status_code(404)
                .with_header(content_type("text/plain; charset=utf-8")),
        };
        // Keep serving even if a single client disconnects mid-write.
        let _ = request.respond(response);
    }
    Ok(())
}

fn content_type(value: &str) -> tiny_http::Header {
    // Header values are ascii; the types we pass are literals.
    tiny_http::Header::from_bytes(&b"Content-Type"[..], value.as_bytes())
        .expect("Content-Type header is valid ascii")
}

fn absolutize(path: PathBuf) -> Result<PathBuf> {
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(std::env::current_dir()
            .context("resolving current dir for landscape path")?
            .join(path))
    }
}

#[cfg(test)]
pub(crate) mod testing {
    use std::path::Path;

    use crate::view::snapshot::{
        DefenseSection, ForceKind, LandscapeSnapshot, RecordSource, SnapshotAttractor,
        SnapshotComponent, SnapshotForce, SnapshotMetaForce, SnapshotResidue,
    };

    /// Seed a main ledger: A-01, S-01, P-01, components auth/db, S-01 × auth coupling.
    pub fn seed_main_ledger(dir: &Path) {
        std::fs::create_dir_all(dir).expect("create ledger dir");
        std::fs::write(
            dir.join("attractors.csv"),
            "id,name,description,positive_state,negative_state\n\
A-01,Stability,System remains stable,coherent NKP,Ri collapses\n",
        )
        .expect("write attractors.csv");
        std::fs::write(
            dir.join("stressors.csv"),
            "id,shortname,description,naive_change,outcomes,attractor_id\n\
S-01,queue-overload,queue overload,add retry,requests drain,A-01\n",
        )
        .expect("write stressors.csv");
        std::fs::write(
            dir.join("purposes.csv"),
            "id,shortname,description,naive_change,outcomes,attractor_id\n\
P-01,landscape-view,operator sees the landscape,ship a view command,operator filters forces,A-01\n",
        )
        .expect("write purposes.csv");
        std::fs::write(
            dir.join("components.csv"),
            "name,description,status,architecture_set\n\
auth,Auth module,actual,iter1\n\
db,Database,actual,iter1\n",
        )
        .expect("write components.csv");
        std::fs::write(dir.join("residues.csv"), "force,auth,db\nS-01,1,\n")
            .expect("write residues.csv");
    }

    /// Seed the defense ledger: MS-01, MA-01, MP-01 and one artifact of each kind.
    pub fn seed_defense_ledger(dir: &Path) {
        std::fs::create_dir_all(dir.join("defense/strategy")).expect("create defense/strategy");
        std::fs::create_dir_all(dir.join("defense/progress")).expect("create defense/progress");
        std::fs::create_dir_all(dir.join("defense/pitches")).expect("create defense/pitches");
        std::fs::create_dir_all(dir.join("defense-personas")).expect("create defense-personas");

        std::fs::write(
            dir.join("defense/meta-stressors.csv"),
            "id,shortname,description\n\
MS-01,meta-force-landscape,Defense-layer stressor\n",
        )
        .expect("write meta-stressors.csv");
        std::fs::write(
            dir.join("defense/meta-attractors.csv"),
            "id,name,description,positive_state,negative_state\n\
MA-01,Practitioner Standing,Defense attractor,safe to discuss,reads as heresy\n",
        )
        .expect("write meta-attractors.csv");
        std::fs::write(
            dir.join("defense/meta-purposes.csv"),
            "id,shortname,description,naive_change,outcomes,attractor_id\n\
MP-01,defense-efficacy,Defense purpose,capture meta forces,operator records defense purpose,MA-01\n",
        )
        .expect("write meta-purposes.csv");

        std::fs::write(dir.join("defense-personas/hostile-auditor.md"), "# md\n")
            .expect("write defense persona");
        std::fs::write(dir.join("defense/strategy/alpha-strategy.md"), "# md\n")
            .expect("write defense strategy");
        std::fs::write(dir.join("defense/progress/week-1.md"), "# md\n")
            .expect("write defense progress");
        std::fs::write(dir.join("defense/pitches/exec-summary.md"), "# md\n")
            .expect("write defense pitch");
    }

    /// In-memory main-ledger snapshot matching [`seed_main_ledger`] (no disk I/O).
    pub fn fixture_main_snapshot() -> LandscapeSnapshot {
        LandscapeSnapshot {
            attractors: vec![SnapshotAttractor {
                id: "A-01".into(),
                name: "Stability".into(),
                description: "System remains stable".into(),
                positive_state: "coherent NKP".into(),
                negative_state: "Ri collapses".into(),
                source: RecordSource::Sidecar,
            }],
            stressors: vec![SnapshotForce {
                id: "S-01".into(),
                shortname: "queue-overload".into(),
                description: "queue overload".into(),
                naive_change: "add retry".into(),
                outcomes: "requests drain".into(),
                attractor_id: "A-01".into(),
                kind: ForceKind::Stressor,
                source: RecordSource::Sidecar,
            }],
            purposes: vec![SnapshotForce {
                id: "P-01".into(),
                shortname: "landscape-view".into(),
                description: "operator sees the landscape".into(),
                naive_change: "ship a view command".into(),
                outcomes: "operator filters forces".into(),
                attractor_id: "A-01".into(),
                kind: ForceKind::Purpose,
                source: RecordSource::Sidecar,
            }],
            components: vec![
                SnapshotComponent {
                    name: "auth".into(),
                    description: "Auth module".into(),
                    status: "actual".into(),
                    architecture_set: "iter1".into(),
                    source: RecordSource::Sidecar,
                },
                SnapshotComponent {
                    name: "db".into(),
                    description: "Database".into(),
                    status: "actual".into(),
                    architecture_set: "iter1".into(),
                    source: RecordSource::Sidecar,
                },
            ],
            residues: vec![SnapshotResidue {
                force_id: "S-01".into(),
                component_id: "auth".into(),
                coupled: true,
                whole_system: false,
                notes: String::new(),
                source: RecordSource::Sidecar,
            }],
            defense: None,
        }
    }

    /// Main ledger + defense section matching [`seed_defense_ledger`].
    pub fn fixture_defense_snapshot() -> LandscapeSnapshot {
        let mut snap = fixture_main_snapshot();
        snap.defense = Some(DefenseSection {
            meta_stressors: vec![SnapshotMetaForce {
                id: "MS-01".into(),
                shortname: "meta-force-landscape".into(),
                description: "Defense-layer stressor".into(),
                naive_change: String::new(),
                outcomes: String::new(),
                attractor_id: String::new(),
                kind: ForceKind::Stressor,
                source: RecordSource::Sidecar,
            }],
            meta_purposes: vec![SnapshotMetaForce {
                id: "MP-01".into(),
                shortname: "defense-efficacy".into(),
                description: "Defense purpose".into(),
                naive_change: "capture meta forces".into(),
                outcomes: "operator records defense purpose".into(),
                attractor_id: "MA-01".into(),
                kind: ForceKind::Purpose,
                source: RecordSource::Sidecar,
            }],
            meta_attractors: vec![SnapshotAttractor {
                id: "MA-01".into(),
                name: "Practitioner Standing".into(),
                description: "Defense attractor".into(),
                positive_state: "safe to discuss".into(),
                negative_state: "reads as heresy".into(),
                source: RecordSource::Sidecar,
            }],
            personas: vec!["hostile-auditor".into()],
            strategies: vec!["alpha-strategy".into()],
            progress: vec!["week-1".into()],
            pitches: vec!["exec-summary".into()],
        });
        snap
    }
}

#[cfg(test)]
mod tests {
    use super::testing::{seed_defense_ledger, seed_main_ledger};
    use super::*;
    use std::path::Path;
    use tempfile::tempdir;

    fn cfg_for(ledger: &Path) -> Config {
        Config::for_test_residual_dir(ledger)
    }

    /// `residual view` produces a real file whose path the caller can print.
    #[test]
    fn view_writes_html_to_temp_dir_and_prints_path() {
        let dir = tempdir().unwrap();
        let ledger = dir.path().join("residual");
        seed_main_ledger(&ledger);

        let path = write_landscape_html(&cfg_for(&ledger), &ViewOptions::default())
            .expect("view must write a landscape html file");

        assert!(path.is_absolute(), "printed path must be absolute: {path:?}");
        assert_eq!(
            path.extension().and_then(|e| e.to_str()),
            Some("html"),
            "written artifact must be .html: {path:?}"
        );
        assert!(path.is_file(), "landscape file must exist at {path:?}");
        assert!(
            path.starts_with(std::env::temp_dir()),
            "default output is ephemeral temp, got {path:?}"
        );
        assert!(
            !path.starts_with(&ledger),
            "landscape must never be written into the ledger dir: {path:?}"
        );

        let out = std::fs::read_to_string(&path).expect("read landscape html");
        assert!(
            out.contains(r#"<script id="residual-snapshot" type="application/json">"#),
            "written html must carry the embedded snapshot"
        );
        assert!(out.contains("S-01"), "written html must carry ledger data");
        assert!(
            out.contains("data-copy-commands"),
            "written html must carry the copy-commands surface"
        );
    }

    /// `--out <dir>` wins over the temp dir and is created when missing.
    #[test]
    fn view_honors_out_dir_and_creates_it() {
        let dir = tempdir().unwrap();
        let ledger = dir.path().join("residual");
        seed_main_ledger(&ledger);
        let out_dir = dir.path().join("nested/out");

        let path = write_landscape_html(
            &cfg_for(&ledger),
            &ViewOptions {
                include_defense: false,
                out_dir: Some(out_dir.clone()),
            },
        )
        .expect("view must write into --out");

        assert_eq!(path, out_dir.join(LANDSCAPE_FILENAME));
        assert!(path.is_file(), "landscape file must exist at {path:?}");
    }

    /// Repeat runs must not error or append — the page is regenerated wholesale.
    #[test]
    fn view_overwrites_existing_landscape_file() {
        let dir = tempdir().unwrap();
        let ledger = dir.path().join("residual");
        seed_main_ledger(&ledger);
        let out_dir = dir.path().join("out");

        let opts = ViewOptions {
            include_defense: false,
            out_dir: Some(out_dir.clone()),
        };
        let first = write_landscape_html(&cfg_for(&ledger), &opts).expect("first view write");
        let second = write_landscape_html(&cfg_for(&ledger), &opts).expect("second view write");
        assert_eq!(first, second);

        let out = std::fs::read_to_string(&second).expect("read landscape html");
        assert_eq!(
            out.matches(r#"<script id="residual-snapshot""#).count(),
            1,
            "regenerating must overwrite, not append"
        );
    }

    /// Defense gating holds all the way through to the written file.
    #[test]
    fn view_without_defense_flag_writes_no_defense_records() {
        let dir = tempdir().unwrap();
        let ledger = dir.path().join("residual");
        seed_main_ledger(&ledger);
        seed_defense_ledger(&ledger);
        let out_dir = dir.path().join("out");

        let path = write_landscape_html(
            &cfg_for(&ledger),
            &ViewOptions {
                include_defense: false,
                out_dir: Some(out_dir),
            },
        )
        .expect("view must write a landscape html file");

        let out = std::fs::read_to_string(&path).expect("read landscape html");
        for needle in ["MS-01", "MA-01", "MP-01", "hostile-auditor", "alpha-strategy"] {
            assert!(
                !out.contains(needle),
                "defense record {needle} leaked into the default landscape"
            );
        }
    }

    #[test]
    fn view_with_defense_flag_writes_defense_records() {
        let dir = tempdir().unwrap();
        let ledger = dir.path().join("residual");
        seed_main_ledger(&ledger);
        seed_defense_ledger(&ledger);
        let out_dir = dir.path().join("out");

        let path = write_landscape_html(
            &cfg_for(&ledger),
            &ViewOptions {
                include_defense: true,
                out_dir: Some(out_dir),
            },
        )
        .expect("view must write a landscape html file");

        let out = std::fs::read_to_string(&path).expect("read landscape html");
        assert!(out.contains("MS-01"), "--defense must render meta-stressors");
        assert!(
            out.contains("hostile-auditor"),
            "--defense must render defense personas"
        );
    }

    /// The view command is read-only against the ledger.
    #[test]
    fn view_does_not_mutate_the_ledger() {
        let dir = tempdir().unwrap();
        let ledger = dir.path().join("residual");
        seed_main_ledger(&ledger);
        let before = std::fs::read_to_string(ledger.join("stressors.csv")).unwrap();
        let mut names_before: Vec<String> = std::fs::read_dir(&ledger)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names_before.sort();

        write_landscape_html(&cfg_for(&ledger), &ViewOptions::default())
            .expect("view must write a landscape html file");

        let after = std::fs::read_to_string(ledger.join("stressors.csv")).unwrap();
        let mut names_after: Vec<String> = std::fs::read_dir(&ledger)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names_after.sort();
        assert_eq!(before, after, "view must not rewrite ledger CSVs");
        assert_eq!(names_before, names_after, "view must not add ledger files");
    }

    #[test]
    fn serve_url_is_loopback_with_the_requested_port() {
        assert_eq!(serve_url(8765), "http://127.0.0.1:8765/");
        assert_eq!(
            serve_url(DEFAULT_PORT),
            format!("http://127.0.0.1:{DEFAULT_PORT}/")
        );
    }

    /// `serve` and `view` must not drift: same snapshot, same page.
    #[test]
    fn serve_document_matches_the_view_html_for_the_same_config() {
        let dir = tempdir().unwrap();
        let ledger = dir.path().join("residual");
        seed_main_ledger(&ledger);
        let cfg = Config::for_test_residual_dir(&ledger);

        let doc = build_serve_document(&cfg, false).expect("serve document builds");
        let snap = load_landscape_snapshot(&cfg, false).expect("snapshot loads");
        let expected = render_landscape_html(&snap).expect("landscape html renders");

        assert_eq!(doc.html, expected, "serve must hand out the same page as view");
        assert_eq!(
            doc.snapshot_json,
            snap.to_json().expect("snapshot json"),
            "serve must expose the same snapshot json"
        );
    }

    #[test]
    fn serve_document_excludes_defense_by_default() {
        let dir = tempdir().unwrap();
        let ledger = dir.path().join("residual");
        seed_main_ledger(&ledger);
        seed_defense_ledger(&ledger);
        let cfg = Config::for_test_residual_dir(&ledger);

        let doc = build_serve_document(&cfg, false).expect("serve document builds");
        for needle in ["MS-01", "MA-01", "MP-01", "hostile-auditor"] {
            assert!(
                !doc.html.contains(needle),
                "defense record {needle} leaked into a non-defense served page"
            );
            assert!(
                !doc.snapshot_json.contains(needle),
                "defense record {needle} leaked into non-defense served json"
            );
        }
        assert!(!doc.snapshot_json.contains("\"defense\""));
    }

    #[test]
    fn serve_document_includes_defense_with_flag() {
        let dir = tempdir().unwrap();
        let ledger = dir.path().join("residual");
        seed_main_ledger(&ledger);
        seed_defense_ledger(&ledger);
        let cfg = Config::for_test_residual_dir(&ledger);

        let doc = build_serve_document(&cfg, true).expect("serve document builds");
        assert!(doc.snapshot_json.contains("MS-01"));
        assert!(doc.html.contains("MS-01"));
    }
}

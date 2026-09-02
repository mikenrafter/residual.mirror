# Iteration 4 v3 — CLI hub + fully-qualified Structure/Storage tree

Prototype code structure (`architecture_set = iter4-cli-hub`).
**Iteration 5** implements A-02/A-04/A-06/A-07 outcomes atop this tree — see
[`residual/iterations/5.md`](residual/iterations/5.md). Component names remain
**fully-qualified** (e.g. `skills-personas`, not bare `personas`).

## v4 data model

On-disk ledger shape (`format_version = "v4"` in `config.toml`):

| Artifact | Role |
|----------|------|
| `residues.csv` | NKP coupling matrix (force × component, `1`/empty) — sole coupling source |
| `stressors.csv` / `purposes.csv` | Forces: outcomes, naïve change, attractor (no component lists) |
| `components.csv` | Component registry (proposed/actual status), not coupling |
| `lexicon.csv` | Domain vocabulary (legacy `terminology.csv` migrated by `residual migrate`) |
| `personas/<name>.md` | Stakeholder voices |
| `attractors.csv` | Attractors with positive and negative states |
| `defense/*.csv` | Meta forces (MS-/MA-/MP-) — isolated from main ledger |
| `.walk-review.toml` | Walk cadence state (on git-sidecar branch when enabled) |

**Git sidecar:** when `git_sidecar_enabled = true`, the full tree above lives on an orphan branch
(e.g. `residual/metadata`). Config may live in `../residual/config.toml` (stealth-mode) one
directory above the repo. Verify resolves metadata from the sidecar tip; tag scan uses working-tree
code + sidecar metadata.

**Workflow:** `residual add stressor|purpose`, then `residual add residue --force-id … --component-id …` per coupling. `residual matrix show` reads `residues.csv` only — not inline force columns.

The iter4 tree below is the Rust module layout; NKP matrix columns come from `residues.csv` and the component registry, not from force rows.

## Naming rule

Every registry row in `residual/components.csv` uses the exact fully-qualified string from the
architecture set. Bare leaf names (`personas`, `analysis`, `sessions`, …) are not component ids.

## Tree

```text
research-study                 ← STANDALONE, NOT RUNTIME (registry + terminology only)

cli                            ← hub: dispatch only; process text beside each clap action
cli-help                       ← completions / man / generate help

skills-personas                ← save/retrieve personas (SPLIT from research)
skills-research                ← walk+persona notes (NOT research-study)
skills-phases                  ← skill-list, skill-show, skill-data, ATAM+FMEA prose
skills-installer               ← skill-install, skill check-install

verification                   ← reads policy from storage-config (no separate verify config module)
verification-git-hook

structure                      ← EXTERNAL filter/sort/group API (default group=attractor)
├─ structure-analysis          ← NKP only — NOT ATAM/FMEA
│  ├─ structure-analysis-tag-scan
│  ├─ structure-analysis-force          ← force = 1/2 residue (purpose XOR stressor)
│  ├─ structure-analysis-purposes       ← :: Force — outcomes, NOT traits
│  ├─ structure-analysis-stressors      ← :: Force — outcomes, NOT traits
│  ├─ structure-analysis-attractors     ← +/- states, NO valence
│  └─ structure-analysis-residues      ← force ↔ component mapping
└─ structure-definition-*
   ├─ structure-definition-lexicon
   ├─ structure-definition-components
   └─ structure-definition-iterations

storage                        ← read / write / mutate
├─ storage-config              ← THE config (v4 TOML): format_version + app + verify policy keys
├─ storage-git-sidecar         ← orphan branch metadata; parent-dir stealth config
├─ storage-sessions
├─ storage-migration           ← naive → v3; v3 → v4; inline → git-sidecar branch lift
└─ storage-format

skills-guru                    ← topic-keyed guidance snippets for skill-data
skills-defense-walk            ← skill lens for defense ledger / outsider rehearsal

structure-defense-*            ← meta ledger (MS-/MA-/MP- only; never main stressors.csv)
├─ structure-defense-analysis
├─ structure-defense-strategy / progress / pitches
skills-defense-personas
structure-analysis-outsiders   ← audience framing + artifact routing
```

## research-study standalone

`research-study` appears in the component registry for longitudinal alpha/beta work, but it is
**not** a runtime Rust module. `skills-research` stores walk notes only. Do not fuse
`skills-personas` with `skills-research`.

## Config split (v4)

| Owner | Keys |
|-------|------|
| **storage-config** | `format_version`, `change_detection`, **and** verify policy (`super_strict`, `token_warn`) |

There is **no** `verification/config.rs` module. Verification *reads* policy from
storage-config (app + verify policy keys live together).

## CLI-as-hub rule

Process / fluency text lives **beside each clap action** (`/// Process:` / `about`), not in a
shared preamble module. Whole-system-residue reminder sits next to add force/residue actions.

| CLI surface | Owner |
|-------------|--------|
| `skill-list` / `skill-show` / `skill-data` | `skills-phases` |
| `skill-install` / `skill check-install` | `skills-installer` |
| `verify *` | `verification` (via storage-config policy) |
| `matrix *` | `structure-analysis` (NKP) |
| `init` / `add` / `remove` | `storage` via `storage-sessions` |
| `add component` | `structure-definition-components` |
| `walk record` / `verify walk-reminder` | `verification-git-hook` + `.walk-review.toml` |
| `migrate --sidecar` | `storage-migration` + `storage-git-sidecar` |
| `tag scan` | `structure-analysis-tag-scan` |
| `generate completions/man` | `cli-help` |
| `generate hook` | `verification-git-hook` |

## Force / residue / attractor

- **Force** = purpose XOR stressor + `shortname` + `naive_change` + `outcomes`. No traits.
- **Residue** = `force_id` × `component_id` (force is half of residue).
- **Attractor** = `positive_state` + `negative_state`. No valence.
- Lexicon continuity: ≥1 terminology word in each force **outcome** and each force **shortname**.

## Tag rule

Metadata-only OK. Tagged-in-code ⇒ must exist in metadata. Tag scan suggests; verification enforces.

## Gaps

1. **research-study / alpha-beta** — registry row only; no runtime module until the study lands.
2. **Legacy on-disk shapes** — `residual migrate` lifts naive→v3 (lexicon, attractor ± states) and v3→v4 (inline force `components` → `residues.csv`). Pre-v3 CSV may still carry `traits`/valence until migrated.
3. **Alternate backends** — migration covers naive→v3 and v3→v4; format is CSV. Deferred beyond that.
4. **P-value / interface discipline** — still no component.

See `residual/iterations/4.md` (stub) and `residual/components.csv` (25 fully-qualified names).

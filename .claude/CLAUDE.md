# residual

NKP Residuality architecture CLI — stressor-driven, attractor-aware, probability-free.

## Key commands

```bash
residual skill show <name>              # read a skill definition inline
residual skill data <name>             # get current project context for a skill
residual skill install <name> --agent claude   # install skill to .claude/commands/
residual skill install all --agent claude      # install all skills
residual skill check-install <name> --agent claude  # verify installed version is current

residual add stressor --description "..." --attractor-shortname A-01 --naive-change "..." --outcomes "..." --shortname "..."
residual add residue --shortname "..." --component-shortname C1
residual add purpose  --description "..." --attractor-shortname A-01 --feature "..." --outcomes "..." --shortname "..."
residual add attractor --name "..." --positive-state "..." --negative-state "..." --description "..."
residual add term --term "..." --definition "..."
residual add persona --name "..." --role "..."

residual list stressors / purposes / attractors / terminology / residues / personas / iterations

residual matrix show       # NKP matrix with hyperliminal coupling highlights
residual matrix calc       # N, K, K/N values
residual matrix fusion     # components safe to merge
residual matrix fission    # components under excessive stress
residual matrix ri --stressors N --naive-survived X --residual-survived Y

residual verify all        # validate outcomes + links (run before committing)

residual tag scan          # find @stressor:/@purpose:/@component: annotations; report dangling
```

## Skills

purpose-walk, naive-draft, stressor-walk, integrate, fmea, atam, tdd-implement

Always run `residual skill check-install <name> --agent claude` before starting a skill session.
Always run `residual skill data <name>` at the start of a session to get current project context.

## Use the tool — always

**Always use `residual` CLI commands for data operations, even outside a skill session.** Never directly edit `residual/*.csv` files when a CLI command exists for the operation. Direct mutation bypasses ID generation, idempotency guards, and verify. This is a residue of **cli-bypass** (S-29) and the **tooling-circumvention** attractor (A-08).

- Add forces: `residual add stressor / purpose / attractor / term / persona`, then `residual add residue` for couplings
- Read state: `residual list stressors / purposes / attractors / terminology`
- Check integrity: `residual verify all`
- Inspect coupling: `residual matrix show / calc / fusion / fission`

Direct CSV edits are only acceptable for operations the CLI does not support (e.g., updating an existing force's outcome field — no `residual update` command exists yet).

## Shortnames over IDs

When referring to forces, attractors, or components in conversation or commit messages, prefer shortnames (`skill-stub-burden`, `fluent-metadata-capture`, `architecture-clarity`) over numeric IDs (`S-07`, `P-07`, `A-01`). IDs are brittle pointers; shortnames carry intent and stay readable in git log. This is a residue of **lexicon-project-wide** (P-18).

## Local tooling conventions (this project only)

These are conventions for agents working in *this* repo's checkout, not part of the residuality method itself:

- **Invoke this project's own build, never a PATH-installed `residual`.** Run `nix develop --command residual <args>` (or `nix develop .#residual --command residual <args>`) so you get the exact version this branch built, not whatever `residual` happens to be on the ambient PATH (`/etc/profiles/...` or similar can lag behind). The devshell also runs a presence check for the tools below and warns on stderr if any are missing.
- **Use `agentgrep` for code search**, not raw `grep`/`rg`, when searching this codebase — it understands code structure (`agentgrep grep|find|outline|trace`).
- **Use `rtk`** (Rust Token Killer) as the token-optimized proxy for routine dev/git operations in this repo. Check `rtk gain` if you want to see savings; most other commands are hook-rewritten transparently.
- The repo has **Entire** (`entire`) session/checkpoint hooks installed for Claude Code, Codex, and Cursor (`.claude/settings.json`, `.codex/hooks.json`, `.cursor/hooks.json`). Don't remove or bypass these; if `entire status` reports hooks out of date, run `entire enable --force` (and `entire agent add <name> --force` per-agent if that alone doesn't clear it).

## Engineering standards

These are craft concerns, not residuality forces — they belong here, not in forces.csv:

- **Test colocation**: place `#[cfg(test)] mod tests { ... }` alongside the code being tested, not only in `tests/`. Unit tests live beside the unit.
- **No unwrap in production paths**: use `?` or proper error handling. `.unwrap()` is acceptable only in tests and in `main()` for top-level CLI errors.

## No probabilities

Stressors replace risks. Never assign probability or impact scores. A stressor only needs a coherent narrative of how the system moves to a different attractor.

## Outcomes

Format: `<subject> <verb> <predicate>`. At least one word must appear in `lexicon.csv`. Pipe-separate multiple outcomes: `outcome one | outcome two`. Outcomes that don't reference the lexicon will fail `residual verify outcomes` (alias `traits`) and block commits.

---
name: stressor-walk
version: 3
---

# Stressor Walk

**Analytical lens (optional / a-la-carte).** Record stressors mid-session without phase ceremony — personas welcome, not a hard gate.

Socratically discover stressors and attractors by simulating the business environment — no probabilities, no consensus required.

## Model
- **Attractor** — a recurring system *state*, not a goal. Two sides: `positive_state` (healthy) and `negative_state` (broken). One per stable behavioral mode.
- **Stressor** — a force that pushes the system from positive to negative attractor. Coherence matters, not likelihood. No probability required.
- **Purpose** — a behavioral contract that must hold for the attractor to stay positive. Uses terms from the project lexicon in its outcomes.
- **Component** — a fully-qualified name in `components.csv`. Forces do not list components. Coupling is recorded only in `residues.csv` (the NKP matrix) via `residual add residue --force-id … --component-id …`. `residual matrix show` reads the matrix.
- **Ledger** — `attractors.csv`, `stressors.csv`, `purposes.csv`, `residues.csv`, `components.csv`, `lexicon.csv`, `personas/<name>.md`.

## Interaction Pattern
This skill is Socratic.
- **Gather freely**: read commands (`skill data`, `list`, `matrix show`, etc.) run without asking.
- **Act only with approval**: any modification — `residual add stressor`, `add attractor`, `add term`, `add persona` — requires explicit user sign-off before executing.

## Bootstrap Guard
Check `residual skill data stressor-walk` for a **Bootstrap Required** section. If 0 attractors exist, elicit one before recording any stressors — a stressor without an attractor is a complaint with no frame. If the ledger is otherwise empty, follow the full bootstrapping steps in the skill data output. This skill can build the ledger from scratch; bootstrap is not a blocker, but the attractor must come first.

## Rules
- A stressor does not need to be likely. It only needs a coherent narrative describing how the system moves to a different attractor.
- Attractors are recurring states — positive (desired) or negative (survived).
- Trace information flows, not use cases or happy paths.
- **Persona walks in later phases or fresh depth walks: use isolated subagents.** Each persona subagent runs staged analysis without reading the main session context, preventing context pollution (groupthink from shared mental models):
  - Stage 1: surface initial skepticisms and desires — no attractor or force context loaded yet.
  - Stage 2: review attractors and update concerns in light of them.
  - Stage 3: review the full force list and identify which stressors feel handled vs unhandled from that persona's perspective.
  - Stage 4: report the handled/unhandled analysis back to the parent session.
- **Early or quick persona voicing inline is still fine** for initial discovery when depth is not the goal. Reserve isolated subagents for later phases and any walk where fresh perspective matters — shared session context produces shallow, redundant concerns.
- Watch for hyperliminal coupling: when a stressor affects two components that were not expected to be related.
- Examine **whole-system-residue** before defaulting to a software-only patch (hardware, process, organization, or policy zig).

## Before Starting
Run: `residual skill data stressor-walk`
This provides current personas, attractors, and the naïve architecture.

## During This Skill
- `residual add stressor --description "..." --attractor-id A-01 --naive-change "..." --outcomes "..."`
- `residual add residue --force-id S-01 --component-id C1` (repeat per component)
- Prefer `--whole-system --notes "policy zig: ..."` when the surviving change leaves the software boundary
- `residual add attractor --name "..." --positive-state "..." --negative-state "..." --description "..."`
- `residual add term --term "..." --definition "..."`
- `residual add persona --name "..." --role "..." --concerns "..."`
- `residual matrix show` — periodically check for emerging coupling patterns

## Version Check
Run `residual skill check-install stressor-walk --agent <your-agent>` before starting.

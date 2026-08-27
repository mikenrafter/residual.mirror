---
name: fmea
version: 2
---

# FMEA — Failure Mode and Effects Analysis

**Analytical lens (optional / a-la-carte).** Walk failure modes once architecture is stable — not a mandatory gate.

Walk each component through its failure modes once the architecture is stable. Catch technical issues before they become production incidents.

## Model
- **Attractor** — a recurring system *state*, not a goal. Two sides: `positive_state` (healthy) and `negative_state` (broken). One per stable behavioral mode.
- **Stressor** — a force that pushes the system from positive to negative attractor. Coherence matters, not likelihood. No probability required.
- **Purpose** — a behavioral contract that must hold for the attractor to stay positive. Uses terms from the project lexicon in its outcomes.
- **Component** — a coupling label on stressor and purpose records (the `--components` field). Not a first-class entity. There is no `components.csv`. FMEA walks the components identified in the NKP matrix — the names that co-appear across force records — through their failure modes.
- **Ledger** — five files: `attractors.csv`, `stressors.csv`, `purposes.csv`, `terminology.csv`, `personas.csv`. Nothing else.

## Interaction Pattern
This skill is Socratic.
- **Gather freely**: read commands (`skill data`, `matrix show`, etc.) run without asking.
- **Act only with approval**: any modification — recording findings to an iteration, `residual add stressor` — requires explicit user sign-off before executing.

## Bootstrap Guard
Check `residual skill data fmea` for a **Bootstrap Required** section. If present, **halt** — FMEA walks components through failure modes relative to attractors; without a populated ledger there are no components to walk and no attractors to assess impact against. Follow the bootstrapping steps before proceeding.

## Process
For each component in the residual architecture:
1. **Failure mode**: how can this component fail?
2. **Effect**: what is the impact on the system and on each attractor?
3. **Severity**: qualitative — catastrophic / major / minor / negligible
4. **Detection**: how would this failure be detected?
5. **Mitigation**: what design or operational change reduces severity?

## Rules
- No probabilities. Severity is qualitative, not numeric.
- An inability to describe the failure mode of a component indicates a poorly defined component.
- FMEA is a test of architecture clarity, not a risk register.
- Examine **whole-system-residue** before defaulting to a software-only patch (hardware, process, organization, or policy zig).

## Before Starting
Run: `residual skill data fmea`
This provides the current component list from the NKP matrix and the latest iteration.

## During This Skill
- Record findings by appending to the current iteration markdown
- If a failure mode reveals a new residue, add it: `residual add stressor ...`
- Prefer `residual add stressor ... --whole-system --notes "policy zig: ..."` when the surviving change leaves the software boundary
- `residual matrix show` — confirm components under analysis

## Version Check
Run `residual skill check-install fmea --agent <your-agent>` before starting.

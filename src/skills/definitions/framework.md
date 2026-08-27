---
name: framework
version: 1
description: >-
  NKP Residuality framework primer. Read this when starting a skill session on
  a project you have not seen before, or when the ledger is empty and you need
  orientation before bootstrapping.
---

# NKP Residuality Framework

Reference primer — not a workflow skill. Read before the first skill session on a new project, or any time you need to reorient.

## What Residuality Is

Residuality is a stressor-driven, attractor-aware approach to architecture. Instead of asking "what could go wrong?" (probability), it asks "what environment must this system survive, and does the architecture survive it?" The output is a *residual architecture* — the simplest structure that survives the stressors the system actually faces.

**No probabilities. No risk scores.** Only coherent narratives of how the system moves between states.

## The Five Primitives

### Attractor
A recurring system *state*, not a goal or mission statement. Every attractor has two sides:
- `positive_state` — what the system looks like when healthy
- `negative_state` — what the system looks like when broken (or just different)

One attractor per stable behavioral mode. If a description lists two concerns joined by "and", it is probably two attractors. Attractors are discovered by asking "describe the system when it's working well — and when it's broken."

### Stressor
A force from the system's environment that pushes it from positive to negative attractor. A stressor does not need to be likely — it only needs a coherent narrative of how the push happens. Sources: business environment, user behavior, operational conditions, regulatory change, team structure, third-party dependencies.

Stressors are linked to an attractor and to the components they stress via `--components`.

### Purpose
A behavioral contract that must hold for the attractor to remain positive. If a purpose is absent or broken, the system moves toward its negative attractor. Every purpose must produce at least one outcome using terms from the project lexicon. Purposes are anchors for architecture: every vertical slice in the design should map to at least one purpose.

### Component
A coupling label — not a first-class ledger entity. **There is no `components.csv`.** Components exist only as names in the `--components` field on stressor and purpose records. The NKP matrix is derived computationally from which component names co-appear across force records.

When a stressor affects two components that were not expected to be related, that is *hyperliminal coupling* — the naïve architecture did not anticipate this dependency.

Named concepts, personas, voices, and workflow steps are **not** components unless they appear in a `--components` field on a force record.

### Terminology / Lexicon
Domain terms that give outcomes their precision. Outcomes in stressors and purposes must use terms from `terminology.csv`. Vague language in outcomes means the outcome can never be verified — push back until the term is defined.

## The Ledger

Five CSV files — nothing else belongs to the model:

| file | contains |
|------|----------|
| `attractors.csv` | system states (positive + negative sides) |
| `stressors.csv` | forces, each linked to an attractor + components |
| `purposes.csv` | behavioral contracts, each linked to an attractor |
| `terminology.csv` | domain vocabulary |
| `personas.csv` | stakeholder voices (used in stressor-walk, fmea, atam) |

**Always use `residual add ...` commands. Never edit CSVs directly** — direct edits bypass ID generation, idempotency guards, and `verify`.

## NKP Matrix

N = unique component names + number of stressors  
K = total component coupling across all stressors  
K/N = coupling density

- Low K/N: components are relatively independent (easier to change in isolation)
- High K/N: components are tightly coupled (changes cascade)

```
residual matrix show      # visualize coupling
residual matrix fusion    # components safe to merge (identical stress patterns)
residual matrix fission   # components under excessive stress (candidates to split)
```

## Bootstrapping a New Project

A project is bootstrapped when it has at least one attractor, one stressor, and one purpose. Without these, skills that depend on the ledger (integrate, fmea, atam) cannot run meaningfully.

Bootstrapping sequence (Socratic — propose each for approval before `residual add`):
1. Elicit the attractor: "describe the system when it's healthy — and when it's broken"
2. Run `git log --stat --oneline | head -40` to surface architectural coupling signals (wide-spanning commits = components the naïve design didn't expect to be coupled; recurring churn = rework the design didn't anticipate)
3. Propose the attractor for approval, then `residual add attractor`
4. Elicit stressors from archaeological evidence and user discussion; propose each before recording
5. Derive purposes from the attractor positive state; propose each before recording
6. Add terminology as concepts emerge

## Skill Overview

Skills are analytical lenses — use them in any order your workflow needs:

| skill | use when |
|-------|----------|
| `purpose-walk` | defining behavioral contracts from the attractor positive state |
| `stressor-walk` | discovering forces from the business environment |
| `naive-draft` | proposing an initial architecture anchored to purposes |
| `integrate` | deriving residual architecture from the NKP matrix |
| `fmea` | walking failure modes component by component |
| `atam` | surfacing stakeholder trade-offs against the candidate architecture |
| `tdd-implement` | implementing changes with multi-agent R/G TDD |

No mandatory order. No mandatory gates. `residual verify all` enforces structure, not ceremony.

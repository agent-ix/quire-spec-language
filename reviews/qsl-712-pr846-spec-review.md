---
id: SR-9128
title: "Spec review (base) of quire-spec-language PR #846: ADR-011 layer edges exclude external crates"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@315939af5561b2348f658daa36b18d1ff34a6ea7; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md"
review_set: subset
---
# Spec review (base) of quire-spec-language PR #846

## Summary

Ticket: QSL-712. Base spec review of two sentences added to ADR-011: the
§6.1 intro sentence saying the "Depends on" column lists first-party layer
edges and that an external general-purpose crate is not a layer edge (with
the `quire-driver` / `serde_json` / FR-331 / ADR-013 T-7 example), and the
crate-map sentence that now admits "external general-purpose crates, which
are not layer edges" in a layer crate's `[dependencies]`.

Examined:
- ADR-011 §6.1 intro, new sentence (examined)
- ADR-011 crate-map sentence, lines 743-746 (examined)
- ADR-011 §6.1 layer table cells for SV, W, WG, 2, 4 and driver (examined)
- ADR-013 T-7 row (line 984) and line 789: descriptors cross as data; "A
  backend's descriptor is its FR-331 provider manifest. The driver reads it".
  The T-7 citation and the FR-331 manifest attribution are accurate (examined)
- Every other spec/ sentence naming the "Depends on" cell or external crates.
  Only spec/reviews/stage-dag/dependency.md FND-014 (a historical review
  record) and FR-090 (which reads the column for first-party `check`-core
  edges only) mention it; neither requires external crates in the cell
  (examined)
- tests/it/family_outcome_layering.rs:301-445, the per-crate
  `[dependencies]` allow-lists (examined)
- quire-driver origin/main Cargo.toml and open PR agent-ix/quire-driver#53
  (context_only)

`quire validate --scope .` on the ADR: exit 0, warnings only.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new rule contradicts §6.1 cells that name and restrict external general-purpose crates. The intro now says the column lists first-party layer edges and an external general-purpose crate is not a layer edge, and the crate map lets any layer crate add external general-purpose crates. But the SV cell names `serde` and `thiserror` ("each without `std`") and the WG cell says "`stacker` only". Under the new crate-map sentence WG may add any general-purpose crate, which its cell forbids, and SV's `serde`/`thiserror` entries are now either redundant or the only place their no-std constraint lives. "General-purpose" is not defined, so a reader cannot tell which external crates still have to be listed (e.g. `quire-contract-model`, quire-rs, `quire-canonical`) | ADR-011:715, ADR-011:721, ADR-011:723, ADR-011:744-746 |
| FND-002 | medium | ADR and enforced gate now disagree. tests/it/family_outcome_layering.rs:356-361 asserts qsl-forms `[dependencies]` are exactly the four layer crates, justified by the comment "Layer 2's 'Depends on' cell names no external crate, so its `[dependencies]` are exactly the four layer crates". After this PR that inference no longer follows: the ADR allows layer 2 to add an external general-purpose crate (e.g. `thiserror`), and the gate rejects it. The qsl-package and qsl-eval branches (lines 400-461, "The whole shipped table is fixed") are likewise stricter than the ADR now states | tests/it/family_outcome_layering.rs:356-361, tests/it/family_outcome_layering.rs:400-461, ADR-011:744-746 |

## Verdict

**FINDINGS (2 medium).** The ADR-013 T-7 citation is accurate (T-7: a
backend's descriptor is its FR-331 provider manifest, the driver reads it,
and it crosses as data), and FR-331 is the provider-manifest row. No other
sentence in spec/ requires external crates in the "Depends on" cell; the
only one is the historical review record stage-dag/dependency.md FND-014.
The current dependency set still passes the gate, so the change breaks no
enforced gate.

Recorded, not reported: the `quire-driver` example is true of open
agent-ix/quire-driver#53 (which moves `serde_json` to a normal dependency
to read retained manifest bytes), not of driver main, where `serde_json` is
still a dev-dependency.

## New findings (disposition pass 1)

Reviewed at agent-ix/quire-spec-language@8e0f761464e894b21749cd9f17473279698e3bb5.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | high | The narrowed crate-map rule now forbids dependencies that the enforced gate requires. External general-purpose crates are allowed only "where its cell names no external crate and no stricter per-crate dependency table governs it". Layer 3's cell names external crates ("FCD crates from `model::intake` only"), and qsl-semantics has no per-crate exact table (its test branch filters to workspace crates only). So the ADR now limits qsl-semantics to layer crates plus FCD, yet tests/it/family_outcome_layering.rs:385-394 asserts qsl-semantics MUST depend on `quire-canonical`, and qsl-semantics/Cargo.toml also names `serde`, `serde_json`, `thiserror` and `tempfile`. The same narrowing bars `serde`/`serde_json`/`thiserror` in qsl-source (I3 names quire-rs) and qsl-replay (layer 6 names `quire-contract-model`). Separately, the new definition ("one that parses or serializes data") excludes `thiserror`, `sha2`, `logos` and `tempfile`, which qsl-foundation, qsl-cst and qsl-semantics use with no exact table | ADR-011:715, ADR-011:728, ADR-011:745-747, tests/it/family_outcome_layering.rs:385-394, qsl-semantics/Cargo.toml |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 8e0f761: SV and WG cells are now exhaustive for their crates, and "general-purpose" is defined; no remaining contradiction with those cells (the definition's own reach is FND-003) |
| FND-002 | fixed | 8e0f761: per-crate exact dependency tables (qsl-forms, qsl-package, qsl-eval, qsl-route) are named as the stricter gate that governs where they exist; the qsl-semantics mismatch the narrowing creates is FND-003 |

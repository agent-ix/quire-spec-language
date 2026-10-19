---
id: SR-13276
title: "Code review of quire-spec-language PR #1040: QSL-548 E13e, RE-2 per-step simulation records and their settlement (FR-146)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@fab2b74c3; PR #1040 (git diff origin/main...HEAD, base b44aba06d): qsl-semantics/src/check/step_simulation.rs, qsl-semantics/src/check/refinement.rs, qsl-semantics/src/check/mod.rs, qsl-semantics/tests/it/refinement_declarations.rs, qsl-replay/src/model_check/step_settle.rs, qsl-replay/src/model_check.rs, qsl-replay/src/lib.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-146
    type: reviews
---
# SR-13276: Code review of quire-spec-language PR #1040

## Summary

Ticket: QSL-548 (E13e). Code review with the rust-review lane folded in.

The PR adds `qsl_semantics::check::step_simulation_records`. It writes one
`Initial` record and one `Step` record per step row, and no record when the
refinement has a hidden field or an `any` row. The hypotheses are the
invariant clauses whose context is an object type of the concrete model.
The PR also adds `qsl_replay::settle_step_simulation`, which maps a backend
outcome to a terminal value, a settlement basis and a truth.
`CheckedRefinement` gains `concrete_object_types()`. The records use
`StepSimulationRecord`, `StepObligation` and `StepRowRef` from
quire-semantic-value (QSL-899).

The coder did not re-run the checks after merging main, so I ran them at
fab2b74c3 with my own `CARGO_TARGET_DIR` (not `make ci`):

- `cargo fmt --all -- --check`: clean.
- `cargo build --workspace --all-targets`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings`: clean.
- `make string-edge`: no unmarked string comparison.
- `make arch-lint-api-surface-qsl arch-lint-canonical-encoder arch-lint-qualified-core`: pass.
- `cargo test -p qsl-semantics --features test-support --test it refinement_declarations`:
  25 of 25 pass, the 4 new tests included. Without `--features test-support`
  the module is compiled out and 0 tests run. The PR body's "qsl-semantics
  refinement_declarations tests" only means something with the feature on.
- `cargo test -p qsl-replay`: all pass (699 unit tests, the 2 new
  `step_settle` tests included, and every integration binary).
- `cargo test -p qsl-semantics --lib`: 674 pass, 1 ignored.

The rest is sound:

- `StepRowOverflow` uses a checked `u32::try_from`.
- No panic, `unsafe` or `unwrap` was added outside tests.
- There is no compatibility shim.
- The settlement never yields `Refuted`, and a test asserts that.
- `has_hidden` reads `ObjectRow::hidden`, which is exactly AX-2's set.
  RM-2 refuses an abstract population with no `from` row, so no abstract
  type escapes this check.

## Verdict

**Findings to fix in this PR, no build or gate failure.** There are three
medium findings:

- The certificate input cannot express a rejected certificate, so a caller
  would have to settle it `proved` (PC-2 forbids that).
- The hypotheses follow source order, so a record's identity changes when
  two clauses swap places.
- The test suite never runs the filter that excludes abstract-model
  invariants.

One low finding: the module docs claim an identity preimage that no spec
states. The spec and plan gaps are in SR-13277, the gap analysis.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `BackendCertificate` has `Absent`, `Unverifiable` and `Verified`, but no outcome for a certificate the core checker shows wrong. ADR-018 PC-6 separates that case (`QueryMismatch`, `ProofStepInvalid`, `NotRefutation`, `ShapeMismatch`) from an unverifiable one. PC-2 says a rejected certificate settles `inconclusive`, basis `unsettled`, `Inconclusive(CertificateRejected{rule, at})`, and "never settles `proved`". `InconclusiveCause::CertificateRejected` already exists. With this enum, a caller holding a rejected certificate (FR-314, QSL-662) can only pass `Unverifiable`, which settles `Proved{Inductive{1}, Uncertified}`. Fix: add `Rejected { rule, at }` that settles `Inconclusive(CertificateRejected{rule, at})` with basis `Unsettled`, and test it. | qsl-replay/src/model_check/step_settle.rs:21-30, 66-71 |
| FND-002 | medium | `hypotheses` is taken from `graph.state_clauses()` in declaration order, and QSV's `StepSimulationRecord` keeps the order it is given ("QSV neither sorts nor deduplicates it"). Swapping two invariant clauses in the source changes every record's canonical bytes, and so every RE-2 obligation identity, though the hypothesis set is the same. O-09 asks for content identity: an edit that leaves the content unchanged leaves the identity unchanged. Two structurally equal invariants share one clause node id (O-09: one node per distinct clause), so the key can also appear twice. Fix: sort ascending by `NodeKey` and dedupe before building the records, and test that reordering two clauses leaves the records equal. | qsl-semantics/src/check/step_simulation.rs:60-69 |
| FND-003 | medium | The filter that keeps a clause only when its context is a concrete-model object type has no test for the case it exists for. ADR-020 RM-1 lets the abstract model be a model of the declaring package, which is the concrete subject. That package's `state_clauses()` can then hold the abstract model's invariants, and they must not become induction hypotheses over the concrete state. The four new tests run with either no clause or one invariant on `Impl::Counter`. Removing the `concrete_object_types` check fails none of them. The fixture's `check_with` already declares `Spec` and `Impl` in one unit, so an invariant on `Spec::Counter` that is absent from every record's hypotheses is a small test. | qsl-semantics/src/check/step_simulation.rs:62-67; qsl-semantics/tests/it/refinement_declarations.rs:1636-1661 |
| FND-004 | low | The module docs say the record's "canonical bytes (`quire-canonical`, `Encode`) are its identity preimage" under ADR-013 O-09. O-09's obligation identity is a digest that CG computes over the subject node id, its occurrence key, the obligation kind and the `arguments`, not over a QSL record's encoding. FR-146 keys the record by "an occurrence key derived from the refinement node and the obligation", and the PR declares that key not built. The FR-146-AC-3 test compares canonical bytes as a stand-in for "obligation identity differs". State the docs as "the record's members, which the obligation identity will bind once its preimage is specified (SR-13277 FND-007)", and not as a preimage the spec defines. | qsl-semantics/src/check/step_simulation.rs:15-17; qsl-semantics/tests/it/refinement_declarations.rs:1657-1660 |

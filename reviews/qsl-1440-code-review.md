---
id: SR-202610310147
title: "Code review — QSL-1440 scalar validity proof"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-analyze/src/value_check/mod.rs, qsl-analyze/tests/it/value_validity.rs, qsl-replay/src/value_check.rs"
review_set: subset
---

## Summary

Reviewed the frozen QSL-1440 Rust diff at `d501217ba732e6a3af274f7cc3f8572888cb028b` against `origin/main` `886321c542bee2b0692f762b19d81d00bfad95b9`, including the eager occurrence proof, exhaustive assignment result, VV-18/FR-484-AC-6, and existing refutation cases.

## Verdict

**CONDITIONAL** — the implementation and focused coverage are sound on the reviewed paths, with one stale module-level claim that scalar operations can never be proved.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The arm documentation still says scalar operations are never proved, contradicting the new eager completion proof. | qsl-analyze/src/value_check/mod.rs:10-13 |

## Coverage

- Rust review: inspected the changed source and integration test, evaluator behavior, occurrence selection, checked-node traversal, repository conventions, and unchanged CI workflow. No duplicate implementation, unsafe path, or weakened gate found.
- Semantics: a completed call proves only when the selected source location is reachable through arithmetic/coercion nodes; a conditional or short-circuit parent is not traversed. `NotDecisive` is accumulated across the full odometer, so any nondecisive assignment prevents exhaustive proof. A violation at the selected occurrence still returns through native replay; an undefined result elsewhere remains nondecisive.
- Test: VV-18 is converted from the current QSpec model to a real QSL package and asserts the supported/proved/exhaustive settlement. VV-05's undefined refutation and VV-08's modulo deviation remain separate; VV-04, VV-06 and VV-10 retain their prior inconclusive outcomes.
- `cargo fmt --check`: passed.
- Focused Cargo test and Clippy could not run: Cargo attempted to fetch the locked `quire-contract-ir` revision into read-only `/home/peter/.cargo/git/db`; offline retry confirmed the revision is absent from the local cache. The coder reported a passing focused test against the earlier QSpec checkout; that is not a test run by this reviewer.
- No applicable AssuranceProfile found. No CI workflow changed. Aggregate `make ci` was not run, per the dispatching brief.


## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The module-level description now states the eager-path proof condition and preserves uncertain reachability as unknown. |

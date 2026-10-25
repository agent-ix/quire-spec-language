---
id: SR-17209
title: "Code review — QSL-822 object reads through invariant helpers"
type: SpecReview
analysis: code-review
scope: "QSL-822; FR-366-AC-10; qsl-semantics/src/check/state_clause.rs; qsl-semantics/src/check/mod.rs; qsl-eval/tests/it/type_invariants.rs"
review_set: subset
review_date: "2026-10-25"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-366
    type: references
---

## Summary

The scoped Rust review found the invariant checker now follows the reachable
local and imported checked-function graph and refuses only when that graph
performs an object read. Reference identity comparison and pass-through calls
remain accepted. Model-family typing admits model reads in helper bodies under
the selected model-complete profile. The corrected Pair fixture and regression
exercise the FR-366-AC-10 read and non-reading vectors. No findings remain.

## Verdict

**PASS** — focused regression, clippy, formatting, and scoped traceability checks pass.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Rust review

- Changed files: `qsl-semantics/src/check/state_clause.rs`,
  `qsl-semantics/src/check/mod.rs`, and
  `qsl-eval/tests/it/type_invariants.rs`. No CI workflow changed.
- The traversal is iterative, tracks visited local indices and imported
  package/function identities, and returns conservatively if a checked call
  cannot resolve to a body. It does not classify an argument's reference type
  alone as a read.
- The test asserts refusal cause and clause identity for direct and transitive
  reads, and asserts successful compilation for optional-reference forwarding
  and identity comparison. It also covers a named predicate body.
- No applicable `AssuranceProfile` was found under `spec/assurance/`.

## Code-test alignment

FR-366-AC-10 is tagged on the focused integration test. The test executes the
production compiler and distinguishes helper-body object reads from non-reading
helper calls. The scoped matrix reports every FR-366 acceptance criterion
tagged; AC-10 is bound to this test at line 1540.

## Gates

- Focused regression: `cargo test -p qsl-eval --test it a_record_invariant_follows_only_object_reading_helpers -- --nocapture` — passed (1 test).
- `cargo clippy -p qsl-semantics -p qsl-eval --all-targets -- -D warnings` — passed.
- `cargo fmt --all -- --check` — passed.
- Scoped `quire matrix --scope /private/tmp/qsl-822-scoped-matrix --format tsv --strict` — passed; all FR-366 ACs tagged.

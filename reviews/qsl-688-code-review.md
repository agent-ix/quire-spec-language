---
id: SR-2449
title: "Code review of quire-spec-language PR #688: exact numeric intake"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@b4be629d0b02e798ec0972682d27256b3779acf9; PR #688; examples/config-version/model.semantic-ir.json; qsl-semantics/src/model/intake.rs; qsl-semantics/tests/it/model_intake.rs; qsl-semantics/Cargo.toml; Cargo.lock; fresh agent-ix/filament-core-data origin/main@c620d6be99654a7a77f0ecc2f97d3c7136402651; QSL-59 boundary task/59-business-model-fix@240f8cced"
review_set: subset
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-056, type: reviews }
  - { target: ix://agent-ix/quire-spec-language/FR-260, type: reviews }
---
# Code review of quire-spec-language PR #688

## Summary

Ticket: AGE-2229. Immutable reviewed head: `14fbc9c89ac4623e4032f9241f3ef91aaac62714`.
The change correctly consumes FCD `c620d6be` in the numeric intake path, adds
safe-range and exclusive-bound handling, and keeps its source hunks separate
from QSL-59's record/clause/namespace changes. The FCD revision also changes
the architecture fixture's integer constraint operands from JSON numbers to
exact strings; the existing recorded architecture digest was not refreshed.

## Verdict

FAIL. The corpus admission test rejects the freshly lifted architecture
fixture because its recorded digest still identifies the pre-FCD-migration
document.

## Assurance Context

The review included the Rust lane, the changed test intent, the FCD public
consumer surface from a fresh `origin/main` fetch, and the QSL-59 hunk
boundary. No public FCD signature consumed by QSL changed incompatibly: QSL
continues to call `lift` and `decide` with the same signatures. `cargo fmt
--all -- --check` passed. The locked focused test was started but had not
produced a result at review recording time because the shared build lock was
occupied.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The FCD pin changes the architecture package's canonical content, but the intake corpus test still offers the old digest `aed361…`. The freshly lifted c620d6be fixture changes the two Count bound operands from JSON numbers to exact strings and hashes to `4090c5dd…` as the committed fixture bytes; `admit` therefore returns content mismatch while the test expects admission. Refresh the recorded digest from the c620d6be lifted document and retain the exact fixture evidence. | qsl-semantics/tests/it/model_intake.rs:143-145 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | high | The new safe-range initialization clamps explicit wide integer bounds. `reads_decimal_string_bounds_up_to_i128` still expects an explicit `i128::MIN` lower bound to remain `i128::MIN`, but `read_value_type` computes `max(-9_007_199_254_740_991, i128::MIN)` and returns the safe lower edge instead. The analogous FCD effective-range logic starts at the safe range only until an explicit bound is present, then preserves that exact bound. | qsl-semantics/src/model/intake.rs:2273-2315,4735-4763 |

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 14fbc9c89ac4623e4032f9241f3ef91aaac62714 |
| FND-002 | still-open | The digest correction does not change the safe-range intersection; explicit i128-wide bound coverage remains inconsistent with the implementation. |
| FND-002 | fixed | 233609db89f4fe7afe11560b6aa0105c325fc9f9 |

## Coverage

- `FR-260-AC-1` / `TC-730`: examined; the corpus digest assertion is the failing unit.
- `FR-056-AC-1` and `FR-056-AC-16`: examined for intake and exact integer behavior.
- `qsl-semantics/src/model/intake.rs:2268-2323`: examined for safe effective range, inclusive and exclusive bounds, i128 parsing, duplicate constraints, and overflow handling.
- `qsl-semantics/tests/it/model_intake.rs:4766-4789`: examined for the new range and exclusive-bound assertions.
- `qsl-semantics/Cargo.toml:43-44` and `Cargo.lock:6-22`: examined; both FCD crates resolve to c620d6be.
- QSL-59 boundary: examined against `task/59-business-model-fix@240f8cced`; its record/clause/namespace regions do not overlap the numeric reader hunk or numeric fixture test.

The focused locked test was started for this head but could not produce a result in
the shared build lane; the static contradiction in the retained i128 bound test is
independent of that unavailable execution result.

## Disposition pass 2

At `233609db89f4fe7afe11560b6aa0105c325fc9f9`, FND-002 is fixed. Explicit
inclusive and exclusive bounds now replace absent safe defaults and continue to
tighten when a second bound exists on the same side. No new code or gap findings
were identified. The focused model-intake gate is attributed to the dispatching
leader's queued run.

## Disposition pass 3

Reviewed the sole `da3dabe` to `b4be629` delta in the detached checkout
`/tmp/qsl688-review-b4`: ConfigVersion's `min` and `max` operands changed from
JSON numbers `0` and `1000` to canonical decimal strings. This matches FR-144's
exact integer representation and FCD's accepted constraint wire shape. The
supplied focused corpus result is 2/2 PASS; the aggregate old-head result is
red, and model_intake, compile, and lint checks were NOTRUN. Prior FND-001 and
FND-002 remain fixed; no new finding applies to this fixture-only delta.

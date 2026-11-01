---
id: SR-1957023
title: "Code and Rust review — checked temporal authentication successors"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; PR #1973 origin/main...HEAD at 95ec6998923cbb58fb831a1422f99505390ad6d9; all changed Rust files, including QSL-1381/1382/1383/1384; disposition round 4 reviewed at b365e72bcc6de760b36b40bcdabc35f1699c11ec"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-043
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-044
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-045
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-046
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-050
    type: reviews
---

## Summary

Reviewed the checked temporal evaluator, its strict `/2` source-locus adapter, progress ledger, FR-045 classification, query proof boundary, and changed integration tests. The strict source/clock selection checks are present and use the canonical `quire_checked::node::{NodeKind, Visit}` API directly; three defects remain in authenticated checked progress and lint cleanliness.

## Verdict

**FAIL** — two authenticated progress defects and one focused Clippy failure remain.

## Coverage

- `/code-review` and `/rust-review`: API contract, direct checked-node authority, refusal paths, resource handling, state retention, trace-tag/test alignment, and canonical node-kind API integration.
- Reviewed all changed Rust source and test files in `origin/main...HEAD`; the #1974 `NodeKind`/`Visit` use imports canonical types and adds no shim.
- Focused checks: FR-050 authenticated checked successor (1 passed), compiled protocol v2 suite (14 passed), writer handoff/determinism (2 passed), existing-directory refusal (1 passed), and checked query proof (1 passed). `cargo fmt --check` passed. Focused Clippy failed on `clippy::map_identity` at `src/temporal.rs:255`.
- No applicable AssuranceProfile applies to the changed Rust scope.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | [QSL-1381/1384] `evaluate_checked_with_progress_v2` records only `Progress` under authenticated selection and calls `run_checked(..., None)`. For checked `on each` clauses with captures, the authenticated route therefore does not retain instances/captures across calls; capture initializers can be reevaluated and restoration/replay semantics are lost. The test exercises only origin activation. | src/temporal.rs:208-223, tests/it/compiled_protocol_v2.rs:656-671 |
| FND-002 | medium | [QSL-1384] Authenticated ledger keys omit checked clause/source identity. Two checked packages at declaration index 0 with the same admitted definition/revision and clock configuration map to the same `AuthenticatedBinding`; sharing a `Ledger` can make one clause's watermark/closure reject or settle another clause. | src/temporal/progress.rs:23-41, src/temporal.rs:208-223, src/temporal.rs:482-547 |
| FND-003 | medium | [QSL-1382/1384] Focused Clippy with `-D warnings` fails because `mapping_support_checked_v2` applies the identity mapping `.map_err(|error| error)?`; remove the redundant conversion so the changed crates pass their focused Rust gate. | src/temporal.rs:255-256 |

## Dispositions

Round 4, reviewed at head b365e72bcc6de760b36b40bcdabc35f1699c11ec.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Authenticated evaluation now passes retained instance state into the checked evaluator. after_excerpt: `let instances = ledger     .checked_authenticated_instances     .entry(ledger_binding)     .or_default(); run_checked(checked, declaration, trace, retained.watermark, &mut work, Some(instances))` |
| FND-002 | fixed | The checked key now includes the exact checked binding and authenticated artifact binding. after_excerpt: `pub(super) struct CheckedAuthenticatedBinding {     checked: CheckedBinding,     admitted: AuthenticatedBinding, }` |
| FND-003 | fixed | The identity map_err was removed. after_excerpt: `let classification = mapping_support_checked(checked, declaration, surrounding_execution)?;` |

## Final disposition

**PASS — clean at `b365e72bcc6de760b36b40bcdabc35f1699c11ec`.** Round 4 verified all findings in the pass-3 record as fixed and found no new findings. The original `## Verdict` and `## Findings` sections above preserve the pass-3 results; the round-4 outcomes are recorded in `## Dispositions`.

---
id: SR-60032
title: "code review and rust review of QSL-1117 PropertyForm::Refinement outcome mapping (quire-spec-language PR 1605)"
type: SpecReview
analysis: code-review
scope: "; Cargo.lock, qsl-replay/src/outcome.rs, qsl-replay/src/proof_result/{plugin_read,plugin_tests}.rs, qsl-analyze/src/model_check/{check,reduce}.rs"
review_set: subset
date: '2026-10-26'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-290
    type: reviews
---
## Summary

Ticket: QSL-1117. Verdict: PASS with low findings.

Wire spelling: verified byte-identical to quire-semantic-value at 0a365dbb. I round-tripped all six PropertyForm values through `quire_canonical::to_vec` (the QSV encoder, FR-121-AC-5) and through qsl-replay's `wire_form` + serde_json in a scratch test (not committed): the four strings and `{"refinement":{"liveness":false}}` / `{"refinement":{"liveness":true}}` match byte for byte.

Lock: `git diff origin/main -- Cargo.lock` is one line, quire-semantic-value 0f922ad -> 0a365dbb; quire-exact (f6d62c09) and quire-canonical (e1e78261) unchanged, QSV's own deps unchanged. `cargo build --workspace --locked` and `--all-features --locked` pass. No wildcard arm, no allow, no compat fallback. Every PropertyForm match in the workspace is exhaustive; the other uses are equality tests (`== PropertyForm::Liveness`) and qsl-semantics only constructs the four forms.

Mutations (focused tests reduction_cause_wire_keeps_its_payload, tc_774_cause_reader_*): writer drops liveness, writer swaps liveness, reader liveness wrong-kind accepted, reader swaps liveness, reader drops the unknown outer-member refusal: all killed. Survivors: reader body-not-an-object accepted (FND-002), reader extra member in the refinement body accepted (FND-002).

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | The writer test hard-codes the JSON; nothing ties `wire_form` to the QSV encoder, so a QSV spelling change would not fail it, and no test round-trips writer output through `read_plugin_inconclusive_cause`. I verified equality by hand, so this is test strength only. Fix: assert `serde_json::to_vec(&wire_form(f)) == quire_canonical::to_vec(&f, ..)` for the six forms (qsl-replay already depends on quire-canonical), and add one write-then-read per Refinement value. | qsl-replay/src/outcome.rs:1659 | correct-requirement-no-evidence |
| FND-002 | low | Two reader refusals the FR-290 row states have no test: `{"refinement":1}` (body of another kind, `wrong_kind(body_pointer)`) and `{"refinement":{"liveness":true,"x":1}}` (UnknownMember at /cause/form/refinement/x). Mutating the first to return Ok, and filtering the second out, both keep the three focused tests green. Fix: add the two vectors to tc_774_cause_reader_refuses_in_order. | qsl-replay/src/proof_result/plugin_tests.rs:406 | correct-requirement-no-evidence |
| FND-003 | low | The qsl-analyze arms are unreachable today and untested: `property_form` is private to the checker, which only produces the four fieldless forms, so no test can build a Refinement clause (confirmed; the coder's claim holds). The two arms are consistent with the table (TableForm::of maps to RefinementSafety/Liveness, whose PT-2 row reads "no" under partial-order) and with ADR-020 (prepare refuses via FormNotDecided), but a future refinement clause would settle inconclusive ReductionNotPreserving under partial-order (precheck) and refuse FormNotDecided under every other selection; the asymmetry is by design of the table but undocumented. Fix: a comment at the prepare arm, and a test when a checker path first yields Refinement. | qsl-analyze/src/model_check/check.rs:1395, qsl-analyze/src/model_check/reduce.rs:115 | correct-requirement-no-evidence |

## Verdict

PASS. Mergeable as is; FND-001 and FND-002 are cheap and best taken in this PR. Gate: see the spec-review SR for the aggregate result (make ci stops at the inherited string-edge failure on main).

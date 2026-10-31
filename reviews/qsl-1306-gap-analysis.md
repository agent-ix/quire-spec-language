---
id: SR-1935002
title: "gap analysis of PR #1935 (QSL-1306): FR-315-AC-56 normal form"
type: SpecReview
analysis: gap-analysis
date: 2026-10-31
scope: "agent-ix/quire-spec-language; PR #1935 git diff origin/main...13402c26a (FR-315-AC-53, FR-315-AC-56, TC-900 step 28; qsl-eval/src/smt/normal.rs tests; qsl-replay/tests/it/refinement_smt.rs)"
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/FR-315
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-900
  type: reviews
---
# SR-1935002: gap analysis of PR #1935 (QSL-1306)

## Summary

Ticket: QSL-1306. Plan completion: not assessed. `quoin matrix --repo . --json` did not finish within 600 s on this repo, so the criterion-to-test check was done by hand with a grep of `#[trace]` tags. FR-315-AC-56 is tagged by five `smt::normal` unit tests and by `refinement_queries_are_in_normal_form`. FR-315-AC-53 is tagged by `refinement_queries_print_in_the_normative_order`. Every new code path in `normal.rs` belongs to FR-315 "Normal form".

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-315-AC-56 | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:1415 | examined |
| FR-315-AC-53 | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:1412 | examined |
| TC-900 step 28 | spec/test-cases/TC-900-the-smt-lib-encoding-is-canonical.md:90 | examined |
| normal.rs tests | qsl-eval/src/smt/normal.rs:241-374 | examined |
| refinement_queries_are_in_normal_form | qsl-replay/tests/it/refinement_smt.rs:1057-1179 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-315-AC-56 says every Normal form rule gives its stated term, but several stated rewrites are never exercised: rule 5 across `=` (`(= (+ x 1) 3)` to `(= x 2)`) and with the literal on the right of `<`; rule 4 with the Boolean constant on the left (`(= true t)`, `(= false t)`); rule 6 with a `false` condition; and the walker does not detect an unshifted `(<= 0 (+ x 1))`, which only the pinned commitA string catches | qsl-eval/src/smt/normal.rs:291-316, qsl-replay/tests/it/refinement_smt.rs:1058-1110 |

## Verdict

**One medium gap, no orphan code.** The excluded shapes listed in FR-315-AC-56 match the `unnormal` walker one for one, and the walker checks itself with a probe. The pinned `Initial` and `commitA` strings match the AC text byte for byte. The missing cases are untested branches of AC-56's last sentence. A wrong `equality` shift, or a reversed constant-on-left branch, would pass the suite.

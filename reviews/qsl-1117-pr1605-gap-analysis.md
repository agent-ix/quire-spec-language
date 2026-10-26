---
id: SR-60033
title: "gap analysis of QSL-1117 Refinement outcome mapping (quire-spec-language PR 1605)"
type: SpecReview
analysis: gap-analysis
scope: "; spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md (AC-8, AC-9), spec/functional/FR-160-settle-reduced-verdicts-and-reduction-causes.md, qsl-replay/src/proof_result/plugin_tests.rs, qsl-replay/src/outcome.rs"
review_set: subset
date: '2026-10-26'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-290
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-160
    type: reviews
---
## Summary

Ticket: QSL-1117. Plan completion: not assessed (planless).

The new reader vectors sit inside tc_774_cause_reader_accepts_the_plugin_spellings (FR-290-AC-8) and tc_774_cause_reader_refuses_in_order (FR-290-AC-9), which already carry `#[trace("TC-774", ...)]`, so no new tag is owed; the tags are honest for the cases that existed but now cover vectors the AC text does not name. The writer test (outcome::tests::reduction_cause_wire_keeps_its_payload) is untagged and has no owning criterion: no FR in this repo states how the outcome document spells `form`.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | The outcome writer's spelling of a reduction cause's `form` is specified nowhere in QSL: FR-160's table lists `reduction-not-preserving` with no payload spelling, and the only text is FR-290's reader row. The spelling IR-595 and other consumers depend on therefore rests on code and a QSV encoder. Fix: state in FR-160 (or the FR-298 outcome section) that `form` is written in the FR-121 canonical spelling, give the Refinement object vector an AC with a TC tag, and tag the writer test with it. | spec/functional/FR-160-settle-reduced-verdicts-and-reduction-causes.md:84, qsl-replay/src/outcome.rs:1659 | missing-requirement |
| FND-002 | low | FR-290-AC-8 and AC-9 enumerate their vectors and omit the new Refinement ones (both liveness values accepted; missing liveness, non-bool liveness, stray `{"safety":{}}` refused), so the tagged tests assert more than the criterion says. Fix: add the vectors to AC-8 and AC-9. | spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md:555-556 | correct-requirement-no-evidence |
| FND-003 | low | qsl-analyze arms for Refinement (TableForm::of, prepare) have no owning criterion or test; they are unreachable until a checker path yields the form. Honest as untagged; track as a follow-up when one does. | qsl-analyze/src/model_check/reduce.rs:115 | correct-requirement-no-evidence |

## Verdict

PASS with low findings.

---
id: SR-16309
title: "spec-review of STD-444 seven review-found spec points (quire-spec-language PR 1389)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@4bd6cfdd; spec/functional/FR-062, FR-098, FR-100, FR-120, FR-192, FR-280, FR-281, FR-290, FR-400, FR-404; spec/test-cases/TC-160, TC-444, TC-472, TC-627, TC-761, TC-762, TC-763, TC-774, TC-1173"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-192
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-281
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-400
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: references
---

# Spec review of STD-444 seven review-found spec points (quire-spec-language PR 1389)

## Summary

Ticket: STD-444 (points 1 to 7). One spec-review pass over the diff of head 4bd6cfdd against its merge base. Main has since gained #1387 (STD-443: FR-484, FR-357, FR-071, ADR-013/014, TC-1252). That merges cleanly with this head and touches none of its files, but main is no longer an ancestor of the head. QSpec read at main bb3404f9. Open QSL PR heads #1339, #1362 and #1384 were read for interplay.

Examined, clean:

- Point 1: FR-290-AC-12, FR-404-AC-2, TC-1173 step 2 and TC-774 step 8 now give the observable result, `inconclusive`/`Verdicts` with value `true`. That matches FR-098-AC-22's `q` request and FR-098 "Application claim: the observation".
- Point 2: FR-062 "Extent roots" names `RequirementRecord::extent_roots()` (`BTreeSet<WireNodeId>`), and FR-062-AC-15 / TC-160 step 13 give concrete sets for RR-3, RR-7, RR-8, RR-11 and RR-14 that agree with the fixture table. FR-098's unread-parameter rule is named as the consumer.
- Point 3: FR-100 "The call depth of an outcome" defines `frame_depth`. FR-098's observation rule reads it, so a stop at the claim site is observed only at depth 0. Hand-tracing FR-098-AC-27 and FR-100-AC-20 over `d` and `e` gives the stated depths and settlements. The function form `pure decreases(n)` follows the grammar (`qsl-cst/src/grammar.rs:581`) and the TC-734 precedent. Recursive claims are now reproduced, which widens behaviour.
- Point 4 (FR-120 half): `ExpansionLimit::EvaluationMeter { counter }` and FR-120-AC-21 / TC-472 are testable. The FR-192 setting is named by reference to FR-255's accounting setting name, which resolves to bare `work_units` on main and to `accounting.work_units` under #1339, so either landing order holds.
- Point 5: FR-400 places `identity` and `validate_derivation` on `EffectiveDeclarationPreimage` in `model::key`, matching main and #1384 (`model::key` is `pub mod`; #1384 has `declaration_key_wire` as a `model` function).
- Point 6: `duplicate-backend`, `inconsistent-candidates`, `not-statistical`, `invalid-value`, `unbounded-extent` and `unsupported-requested-capability` are all registered QSpec FR-272 causes. QSpec FR-290's candidate table backs the `inconsistent-candidates` rule. The engine-refusal table covers every refusal FR-127, FR-192 and FR-203 leave without a record (FR-192 Admission/RequiresBound/Symmetry/NotStatistical, FR-203-AC-7's Admission/RequiresBound, and FR-126's GuardedActivation and ComponentCertificateNotBuilt), ending in two catch-all rows, and each row names a record. The EN-1 manifest in FR-280 and FR-280-AC-7 / TC-761 step 6 matches #1362's `model_check::manifest` (pair, domains, `weak`/`strong` fairness, no evidence, no `dynamics`). The QSpec FR-331 `fairness_kinds` member exists.
- STD-445 (FR-126/FR-127 model-check meter setting) is outside point 4's named scope (FR-192-AC-1/TC-627) and has its own STD ticket, so it is not a scope deferral.
- The new ids FR-062-AC-15, FR-098-AC-27, FR-100-AC-20, FR-120-AC-21, FR-280-AC-7, FR-281-AC-10/11 and FR-400-AC-10 are unique on main and on all 83 other open QSL PR heads. The only textual conflicts with other heads (FR-120 with #1317, FR-286 with #1339) already exist against main.
- The added lines contain no deferral or status language.

Gate at 4bd6cfdd: `QSPEC_DIR=/Users/peter/dev/quire-specification make check-spec-validation` exits 0 with 0 error lines. Its warnings on touched files are the pre-existing FR-400 line 36 EARS warnings.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | scope-deferral: point 7 is not resolved. QSV FR-126-AC-1 to AC-5 still read bare `Test`, no QSV TC verifies them, and no QSV PR or Linear ticket carries the fix. The fix itself is spec authoring in quire-semantic-value: a new TC, and the Verification cells set to `Test (TC-n)`. The PR body files it as "QSV code-lane work" with no tracker home, so part of the ticket's own scope is split off untracked. Fix: author the QSV TC and Verification cells as a QSV spec PR under STD-444, or file a ticket and link it before merge. | agent-ix/quire-semantic-value spec/functional/FR-126-own-the-evaluation-result-vocabulary.md (AC-1..AC-5 rows) | missing-requirement |
| FND-002 | high | FR-192-AC-1 and TC-627 settle an expansion `EvaluationMeter` stop with limit `EvaluationMeter` and a `MaxCandidates` stop with limit `MaxCandidates`. For the same FR-120 stop, FR-203's rows write `limit` as `max_candidates` and as the exhausted counter's name (FR-255 Behavior 9). QSpec FR-461 Behavior 6 likewise says the provider `LimitReached` cause carries the counter as `limit`. So EN-4 and EN-5 produce different `limit-reached` records for one stop. Fix: FR-192's row and AC-1 take `limit` as the counter name for the meter (`work_units`, `value_occurrences`) and as `max_candidates` for the candidate stop, as FR-203 does. | spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md:105,146 | wrong-requirement |
| FND-003 | medium | FR-400-AC-10 expects "the identities and the `unsorted-derivation` and `duplicate-path` refusal causes" that QSpec TC-195's vectors state. The vectors file's `invalid_mutations` are `absent-node`, `cross-domain-declaration-key`, `owner-as-declaration-key`, `unsorted-derivation`, `duplicate-parent`, `parent-on-qualify-fact`, `mis-wired-link` and `unsorted-view`, so no vector states `duplicate-path`. The duplicate-path oracle has no source. Fix: name the vectors that do apply to `validate_derivation` (for example `unsorted-derivation` and `duplicate-parent`), or give an explicit duplicate-path preimage in the AC. | spec/functional/FR-400-separate-checked-terms-from-the-checker.md:351 | correct-requirement-no-evidence |
| FND-004 | medium | FR-281's `RequiresBound` engine-refusal row, FR-281-AC-11 and TC-763 steps 6 and 7 have the `unsupported_projection`/`unbounded-extent` diagnostic name FR-101's unbounded root. QSpec FR-272 fixes that cause's detail as "the kind, the candidate and its advertised modes", with no root. Fix: give the FR-272 detail (kind, candidate, advertised modes) and, if the root must travel, say where (for example the record's own detail beside the diagnostic). | spec/functional/FR-281-analyze-claims-with-in-process-engines.md:120,280 | wrong-requirement |
| FND-005 | low | FR-281-AC-11 lists seven refusal cases (refused and incomplete admission, RequiresBound, Symmetry, NotStatistical, unsupported, failed) but says "a request of all six returns six records". TC-763 step 6 resolves this as six items plus a seventh run, which the AC does not say. Fix: state in the AC that the incomplete admission is a separate run, or make it seven records. | spec/functional/FR-281-analyze-claims-with-in-process-engines.md:280 | wrong-requirement |

## Verdict

Not mergeable. FND-001 (scope-deferral of point 7) and FND-002 (EN-4 and EN-5 settle the same expansion stop with different `limit` values, contradicting QSpec FR-461) block the PR. FND-003 and FND-004 should be fixed in the same round, and FND-005 is a wording fix. Points 1, 2, 3 and 5 and the routing, manifest and engine-refusal coverage in point 6 are correct and testable. Before merge, the author must also merge origin/main (#1387) into the branch so main is an ancestor again.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | QSV #63 (head 39fb412f) adds TC-926, which verifies FR-126-AC-1 to AC-5. Those rows now read `Test (TC-926)`, AC-1 states `Category::ALL`'s eight values in order, and TC-925's exclusion points at TC-926. IR-989 is the ticket for retagging the tests. |
| FND-002 | fixed | At 7dec747e, FR-192's row, FR-192-AC-1 and TC-627 write `limit` as the exhausted counter's name (`work_units`, `value_occurrences`) or `max_candidates`, the same record FR-203 writes. |
| FND-003 | fixed | At 7dec747e, FR-400-AC-10 uses TC-195's real `unsorted-derivation` mutation and an explicit duplicate preimage, whose result is `DuplicatePath{earlier: 0, later: 1}`. That matches `ModelRefusalCause`. |
| FND-004 | fixed | At 7dec747e, the FR-281 RequiresBound row, FR-281-AC-11 and TC-763 steps 6 and 7 use QSpec FR-272's closed `unbounded-extent` payload (kind, candidate, advertised modes) and no root. |
| FND-005 | fixed | At 7dec747e, FR-281-AC-11 and TC-763 step 6 are one request of six cases plus a separate seventh request for the incomplete admission. |

Disposition pass 1 at 7dec747e: main 40077eaa is now an ancestor. `QSPEC_DIR=/Users/peter/dev/quire-specification make check-spec-validation` exits 0 with 0 error lines, and the only warnings on touched files are the FR-400 line 36 warnings that were already there. This pass found no new defects.

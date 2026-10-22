---
id: SR-16160
title: "Spec review — host-body dispatch, replay facade re-export and Begin's cancel handle (FR-401, FR-262, FR-403, ADR-011)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md, spec/functional/FR-262-evaluate-values-and-calls-at-any-depth.md, spec/functional/FR-403-implement-the-no-std-evaluator-in-quire-eval.md, spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md at 7efe5cd1"
review_set: subset
evaluated_revision: "7efe5cd1cdb40696dd3e992e60c58e2226c1c04e"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-401
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-262
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-403
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
---

## Summary

Ticket: STD-335. PR quire-spec-language#1280 adds FR-401 Behaviour 4's
host-body dispatch rule and FR-401-AC-8, defines "former path" and adds a
sanctioned `qsl-replay` crate-root re-export of moved types with FR-401-AC-9,
amends ADR-011's SV rule and X-10 row to match, and makes Begin's argument
admission take the meter and poll its installed handle (FR-262 statement,
FR-262 charge-only cancellation sentence, FR-262-AC-12, FR-401 contract item
3, FR-403 Argument admission item 5 and its closing paragraph). Reviewed
`git diff origin/main...HEAD` at 7efe5cd1 against FR-060 (T12-A, the seven-rule
set, "T12-A and the evaluator differential corpus"), FR-059, FR-277 (original
Cancel, poll points), FR-276, FR-262's existing cancellation and child-request
validation text, FR-403 Argument admission in full, ADR-011 §7.2, X-10, X-11
and X-14, and PR #1277's FR-403, FR-405 and FR-406 hunks.

Examined units: FR-401 Behaviour 4 added sentences, FR-401-AC-8, the
former-path definition, the sanctioned re-export paragraph, FR-401-AC-9,
FR-401 contract item 3 addition, FR-262 Begin admission paragraph, FR-262
"observe cancellation only at a charge" sentence, FR-262-AC-12, FR-403
Argument admission item 5 and closing paragraph, ADR-011 SV rule exception,
ADR-011 X-10 row. Clean: Behaviour 4 and AC-8 (consistent with FR-262's
scheduler, child-request validation and charge rules; the `7` versus `3` and
`8` versus `4` oracles fail if `f`'s interpreted body runs), FR-401 contract
item 3, the FR-262 charge-only sentence.

Ids FR-401-AC-8, FR-401-AC-9, FR-262-AC-12 and SR-16160 are absent from
origin/main and from every other open PR head (24 checked). PR #1277 edits
FR-403 at other hunks; `git merge-tree` of the two heads reports no conflict.
`origin/main` is an ancestor of the head. `make check-spec-validation`
(QSPEC_DIR=quire-specification) exits 0 with 19 warnings, none on a touched
file. Spec-only diff: code-review and gap-analysis do not apply.

## Verdict

NOT MERGEABLE as written: two high and four medium findings. FR-401's
justification for the re-export exception contradicts FR-060, which puts
CG's edge to `quire-semantic-value` outside T12-A, so CG does not need the
facade to name `Category`. FR-403's Argument admission still opens with
"with no meter in scope", contradicting the amended closing paragraph and
FR-262. The ADR-011 exception ("the SV items its public signatures name")
and FR-401's ("a moved type") cover different sets. FR-401-AC-9 names an
API-surface check that FR-060 defines as exactly seven rules, none about
re-exports. The poll points are stated three different ways across FR-262,
FR-277 and FR-403. The caller-supplied run sentence is not normative and no
AC tests it or a cancel during admission. Behaviour 4, AC-8 and the
charge-only sentence are sound. The three points are flagged in the PR body
as lead defaults, not owner answers.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The sanctioned re-export's premise is false: "FR-060 rule T12-A makes the layer-6 `qsl-replay` facade CG's only QSL edge, so CG can name a type only through that facade". FR-060 "T12-A and the evaluator differential corpus" says `quire-semantic-value` and `quire-eval` are not QSL crates for T12-A and CG's edges to them are outside T12-A, and ADR-011 X-11 makes SV a leaf CG depends on. CG can name `quire_semantic_value::catalog::Category` directly. State the actual reason for the exception or drop it. | spec/functional/FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md:102-109; spec/functional/FR-060-check-qsl-api-surface-boundary.md:223-232 |
| FND-002 | high | FR-403 Argument admission's first sentence still says admission runs "before the application's first charge, with no meter in scope", while the amended closing paragraph and FR-262 say admission takes the meter and polls its handle. The section contradicts itself. | spec/functional/FR-403-implement-the-no-std-evaluator-in-quire-eval.md:198-200,224-231 |
| FND-003 | medium | ADR-011 and FR-401 sanction different sets. ADR-011's SV rule and X-10 allow "the SV items its public signatures name", which includes X-11 items such as `InputRefusal` and `Location` (X-10 says the spine reports them) that FR-401 does not move and X-11's row still forbids ("no re-export from `qsl-semantics` or any other crate"). FR-401 allows any moved type, including the E types `Evaluation` and `CallFailure`, which ADR-011 does not name. Make the two statements cover one set. | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:836-839,1230,1231; spec/functional/FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md:102-109 |
| FND-004 | medium | FR-401-AC-9 asserts "The QSL API-surface check finds ... no `pub use`", but FR-060 says the API-surface rule set "SHALL contain exactly these seven rules" (T12-A to T12-G), none of which scans re-exports, and no TC is named. No check exists to run the AC against; name the check that owns it (TC-390, a new FR-060 rule with FR-060 amended, or a new TC). "(or other re-export)" is also open: say whether a `pub type` alias or a `pub mod` path counts. | spec/functional/FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md:268; spec/functional/FR-060-check-qsl-api-surface-boundary.md:81-92 |
| FND-005 | medium | The poll points are stated three ways: FR-262 "before each event the run charges to its own budget", FR-277 "before each scheduled logical step and after an invoked helper returns", FR-403 item 5 "for each value it visits". FR-262 says it uses "the points FR-277 polls its Cancel" yet paraphrases them differently. Point at FR-277's points and drop the restatements. | spec/functional/FR-262-evaluate-values-and-calls-at-any-depth.md:157-158; spec/functional/FR-403-implement-the-no-std-evaluator-in-quire-eval.md:218-219; spec/functional/FR-277-bound-every-lifecycle-operation-by-caller-limits.md:303-305 |
| FND-006 | medium | "A caller that passes its own run builds it over the handle it installed in the meter" is a description, not a requirement: no SHALL, no outcome when the run's Cancel differs from the meter's handle (which one admission polls, or a refusal), and FR-262-AC-12 does not test it. AC-12 also tests only a handle cancelled before Begin, not the statement's "cancelled during admission" case. Make the caller-supplied run rule normative with its mismatch outcome, and extend AC-12 to a cancel at a later argument's admission event and to a caller-supplied run. | spec/functional/FR-403-implement-the-no-std-evaluator-in-quire-eval.md:227-229; spec/functional/FR-262-evaluate-values-and-calls-at-any-depth.md:159-163,280 |
| FND-007 | low | FR-262 says "concrete Rust item names belong to the interface implementation", but the new statement names `admit_call` and `admit_expression` in `quire-eval`, and AC-12 asserts "`admit_call` received that meter", an internal call observation. Keep the item names in FR-403 or FR-401 and state AC-12's observable outcome only. | spec/functional/FR-262-evaluate-values-and-calls-at-any-depth.md:153-154,280 |
| FND-008 | low | FR-262-AC-12 is inserted above FR-262-AC-11; append it after AC-11. | spec/functional/FR-262-evaluate-values-and-calls-at-any-depth.md:280-281 |

---
id: SR-12900
title: "Integrity review of PR #991: FR-393 against FR-325, the S1 vocabulary and QSpec facet selection"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@104da0711; FR-393 Description, Outputs, Behavior, AC-1..AC-4; FR-325 Description, Outputs, Behavior; FR-230-AC-2; FR-234-AC-2; ADR-026 CK-1..CK-7, §11; qsl-cst/src/token.rs vocabulary; qsl-forms/src/dispatch.rs LeadingTokenKind; qsl-forms/src/syntax.rs TemporalOperatorForm; agent-ix/quire-specification@ad2b37e proposals/quire-v1/shared-grammar.md lines 25-90, 775-894"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-393
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-325
    type: reviews
---

## Summary

Ticket: QSL-580. PR quire-spec-language#991. This review checks that FR-393
is consistent with the merged FR-325 (integer `IntervalForm`), with the
S1 reserved-word vocabulary and its dispatch, and with QSpec's facet rule
for forms whose facet is not selected.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-393 contradicts merged FR-325 and does not amend it. FR-325 says S1 parses every temporal operator "with an interval `[a,b]`, an interval `[a,*]`, or no interval", and that "A bound that is not a decimal `u64` SHALL fail at S1 with a parse diagnostic". `eventually[0 ms, 3 ms]` must therefore fail under FR-325 and must parse under FR-393. FR-325's Outputs struct (`TemporalOperatorForm{operator, interval: Option<IntervalForm>, operands}`, already built at qsl-forms/src/syntax.rs:2930) is also left stale. The design choice adds to this: a second `timed_interval: Option<TimedIntervalForm>` field, with the rule "Some only when interval is None" kept in prose. That makes `Some/Some` representable, and every consumer (FR-326/FR-234 at S3, FR-255 keys, emission) has to check both fields. With pre-release crates and no compat layers, the sound choice is one field, `interval: Option<OperatorInterval>` with `enum OperatorInterval { Steps(IntervalForm), Timed(TimedIntervalForm) }`. FR-325's Description, Behavior and Outputs (and TC-835's `interval: None/Some{..}` oracles) would be amended in this PR. Failure: a coder who follows FR-325-AC-3's rule refuses every timed interval, or one who follows FR-393 breaks a SHALL in FR-325. | spec/functional/FR-393-parse-time-declarations-clocks-and-timed-intervals-and-build-their-s2-forms.md:89-92; spec/functional/FR-325-parse-temporal-operators-with-an-optional-interval.md:24-27,63-64,85-86 |
| FND-002 | medium | FR-393 does not say whether the new words are reserved or contextual. `time`, `urgent`, `dense`, `tick`, `period` and `Clock` are not in qsl-cst's vocabulary (token.rs) or in QSpec's common or complete-V1 reserved lists. FR-393 gives `LeadingTokenKind` `Time` and `Urgent` but does not say how these words lex, unlike the sibling precedent (`bound`: "a declaration-head spelling only; in an expression it stays an identifier", dispatch.rs, and QSpec's contextual-word paragraphs). Failure: if a coder reserves them, any existing model with a field or binder called `time`, `period` or `tick` stops parsing, and grammar.rs's every-reserved-word-refused test makes that a hard error. If a coder leaves them contextual, a user type named `Clock` is ambiguous with `type-ref =/ 'Clock'`. Fix: state the classification of each word and its positions, and add an AC that a field named `time` and one named `period` still parse. | spec/functional/FR-393-parse-time-declarations-clocks-and-timed-intervals-and-build-their-s2-forms.md:96-99; qsl-cst/src/token.rs:145-204; qsl-forms/src/dispatch.rs:176-180 |
| FND-003 | medium | "A `1-draft` unit selecting the timed facet" is not a term used in any other QSL artifact, and FR-393 gives no behaviour for a unit that does not select that facet. QSpec says "A form whose facet is not selected receives `unsupported_construct`", while FR-325 builds the same form under every profile. An implementer cannot tell whether a `time` declaration or `[0 ms, 3 ms]` in an untimed unit fails at S1, builds at S2 and refuses at S3, or builds silently. No AC tests it. Fix: name the facet or profile selection exactly, and add an AC for the unselected case with its stage and code. | spec/functional/FR-393-parse-time-declarations-clocks-and-timed-intervals-and-build-their-s2-forms.md:27-35,107-111; quire-specification proposals/quire-v1/shared-grammar.md:894 |

## Verdict

The second-token rule matches QSpec's lines 863-868: `(` followed by a
quantity-literal opens a timed interval, anything else is a parenthesised
operand. Telling `[` + quantity apart from FR-325's `[` + `uint ,` needs a
third token, and the Behavior states this correctly. The S3 deferrals
(duplicate time source, unit dimension, clock typing, negative constant,
interval order and punctuality, profile admission) agree with FR-230-AC-2
and FR-234-AC-2. FND-001 blocks merge.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | The facet fix cites FR-361 item 8 for "whether the unit admits the form is decided after S2". Item 8 covers protocol, memory and temporal forms, says "Neither is a model construct", and adds no gate. `time-decl`, `time-invariant`, `urgent-decl` and `Clock` are model declarations (ADR-026 CK-1), not in FR-361 item 5's families. So no QSL requirement owns QSpec's "A form whose facet is not selected receives `unsupported_construct`" for these forms after S2. The S1/S2 behaviour is now fully specified (AC-6). The post-S2 owner is outside this PR's scope and needs a follow-up on the S3 side (FR-230/FR-361). | spec/functional/FR-393-parse-time-declarations-clocks-and-timed-intervals-and-build-their-s2-forms.md:39-40,132-134; spec/functional/FR-361-gate-model-constructs-on-the-model-profile-selection.md:133-142 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | There is now one member, `interval: Option<OperatorInterval{Steps(IntervalForm), Timed(TimedIntervalForm)}>`, and the second field is gone. FR-325's Description, Outputs, Behavior (the u64 rule is now limited to step intervals) and AC-1/AC-2 are amended, and TC-835 is amended to `Some(Steps{..})`. The qsl-forms struct follows in the QSL-580 code PR. |
| FND-002 | fixed | A Behavior bullet makes `time`, `urgent`, `dense`, `tick` and `period` contextual and limits `Clock` to the unqualified type-ref. AC-2 tests that a field named `time` and a parameter named `period` still parse, and AC-7 covers `Clock`. |
| FND-003 | fixed | "Timed facet" is removed. S2 now builds these forms under every facet selection, including none, and refuses nothing for facet (Behavior; AC-6 tests equal forms under no facet and under the complete facets). The question of the post-S2 owner is FND-004. |

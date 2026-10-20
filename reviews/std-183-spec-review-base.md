---
id: SR-13491
title: 'Spec review of PR #1088: S3 timed-facet gate (FR-230), base'
type: SpecReview
analysis: base
scope: 'agent-ix/quire-spec-language@9cdbed126; git diff refs/remotes/origin/main...9cdbed126 (PR #1088, branch std-183-spec): spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md, spec/functional/FR-361-gate-model-constructs-on-the-model-profile-selection.md, spec/functional/FR-393-parse-time-declarations-clocks-and-timed-intervals-and-build-their-s2-forms.md, spec/test-cases/TC-685-s3-checks-time-declarations-and-clock-constraints.md; engine measured on origin/main 469f8b49c; QSpec origin/main 240faeb; ticket STD-183'
review_set: subset
date: '2026-10-20'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-230
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-361
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-393
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-685
  type: reviews
---

## Summary

Ticket: STD-183. PR: quire-spec-language#1088, head 9cdbed126. Method: spec-review/base. Owner decisions taken as given: facet identity `quire.temporal.complete/v1`, cause `declaration-form` for all four constructs.

Engine measured on origin/main 469f8b49c in a scratch clone (probe test in qsl-semantics `tests/it`, reverted after):

- S3 today, header `quire.model.complete/v1` only (no temporal facet): `time Config dense unit ms;`, `time invariant ...`, `urgent u when {...};`, `urgent u Config::attemptUpdate;`, `record R { x: Clock; }`, a function parameter `c: Clock` and a function result `Clock` each refuse once, `unsupported_construct`/`not-yet-implemented` (`AssemblyCause::TimedNotChecked`); the four-construct unit gives four refusals in source order. The `Clock` refusal's span is the `Clock` type reference, not the field. The same assembly with a `profile tc = "quire.temporal.complete/v1";` header line gives the same `not-yet-implemented` (assembly does not read the facet).
- E3 (`check::resolve_profiles`, called by `qsl-replay` `finish_unit` before S3): `quire.temporal.complete/v1` -> `unknown_profile`/`unsupported-selection`; `quire.temporal.timed/v1` -> `unknown_profile`/`unsupported-selection`; `quire.model.complete/v1` -> Ok. No QSL catalog (`HeaderLayer`, `protocol_profile`, `TemporalProfile`) holds the temporal facet.
- QSpec: the only mention of `quire.temporal.complete/v1` is the shared grammar's "Complete-V1 facet grammar" list; the timed productions "extend `declaration` when the package selects the complete facets"; FR-415 names no facet; no QSpec lock or catalog row holds the identity.

`make check-spec-validation` exit 0 (17 pre-existing warnings, none in the four touched files); `tools/check-index-completeness.sh` exit 0.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-230 Timed facet gate (bullets 1-3) | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md | examined |
| FR-230-AC-7 | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md | examined |
| FR-230-AC-6 | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md | examined |
| FR-230-AC-1 | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md | context_only |
| FR-230 Clock fields and the domain package | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md | examined |
| FR-361 item 8 | spec/functional/FR-361-gate-model-constructs-on-the-model-profile-selection.md | examined |
| FR-361 items 2, 6, 10, 11 | spec/functional/FR-361-gate-model-constructs-on-the-model-profile-selection.md | context_only |
| FR-393 S2 facet bullet | spec/functional/FR-393-parse-time-declarations-clocks-and-timed-intervals-and-build-their-s2-forms.md | examined |
| TC-685 scope line and step 7 | spec/test-cases/TC-685-s3-checks-time-declarations-and-clock-constraints.md | examined |
| ADR-026 §11 Rpc unit | spec/decisions/ADR-026-dense-time.md | context_only |
| QSpec shared grammar, Complete-V1 facet grammar and timed forms | QSpec proposals/quire-v1/shared-grammar.md | context_only |
| QSpec FR-415 | QSpec spec/functional/temporal/FR-415-declare-clocks-and-timed-models.md | context_only |
| check::resolve_profiles | qsl-semantics/src/check/profile.rs | context_only |
| PackageDeclarations::assemble_with_cancel TimedNotChecked | qsl-semantics/src/check/assemble.rs | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The gate's admitting branch is unreachable: `quire.temporal.complete/v1` is in no QSL catalog, so a header that selects it is refused `unknown_profile`/`unsupported-selection` at E3 before S3 runs (FR-361 items 2 and 11, FR-110; measured). The PR adds no E3 or FR-110 resolution for the identity, and FR-361 item 8 still says QSL has no such layer. As written, every unit that reaches S3 lacks the facet, so the gate refuses every timed construct and FR-230's other rules can never apply; the second half of FR-230-AC-7 cannot pass. | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md:217-224; spec/functional/FR-361-gate-model-constructs-on-the-model-profile-selection.md:133-142; qsl-semantics/src/check/profile.rs:156-189 |
| FND-002 | medium | FR-230-AC-7 and TC-685 step 7 are not marked as waiting on STD-192. Their fixture, ADR-026 §11's `Rpc`, declares `x: Clock` on the package-admitted object type `Call`, which the domain package cannot express until STD-192, and the admitting half is FR-230-AC-1, which waits. The "Clock fields and the domain package" run-now/wait list and TC-685's scope line are not updated. | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md:157,246; spec/test-cases/TC-685-s3-checks-time-declarations-and-clock-constraints.md:15,27,39 |
| FND-003 | medium | A `Clock` field of a package-admitted object type is not written in the unit's source, so "at the construct's span" and "in source order" are undefined for it. `Rpc`'s only `Clock` field is of this kind, yet FR-230-AC-7 places its refusal at its span in source order. FR-361 items 6 and 11 gate only forms the unit itself writes. | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md:217-224,246 |
| FND-004 | medium | Under the selected facet, a `Clock` parameter, a `Clock` result and a `Clock` field of a source-declared record (each refused today, measured) have no FR-230 rule. "Is checked by the rules above" sends them to rules that cover only clock fields of model object types, so whether they are admitted or refused, and with which code, is unspecified. | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md:227-228 |
| FND-005 | low | The gate's opening sentence says QSpec's shared grammar admits the timed forms when the unit selects `quire.temporal.complete/v1`. QSpec says they extend `declaration` "when the package selects the complete facets" and names no single facet for them (FR-415 names none). The identity is an owner decision, so the text should call it QSL's reading, not attribute it to QSpec. | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md:211-215 |

## Verdict

Findings. The intent fits the rest of the spec: FR-393's "S2 never refuses for facet; S3 decides" now points at the gate, the gate keeps FR-361 item 11's shape (source order, every construct, family, selected layers, required facet), and the suppression of other FR-230 refusals matches FR-361 item 12. FND-001 blocks merge: with no E3 resolution for `quire.temporal.complete/v1`, the gate refuses every timed construct in every unit and the admitting half can never run. Fix it by stating how a header selects the facet (an FR-110 or FR-361 row, or another resolution path), or by recording that the facet is unselectable until a named ticket and that FR-230's other rules wait on it. FND-002 to FND-004 are gaps a coder would trip on.

## New findings (disposition pass 1)

Head c6ad506f8. Engine re-measured on origin/main 469f8b49c (main has not moved), probe reverted after. With a header that does not select the facet, each of these refuses once `unsupported_construct`/`not-yet-implemented` at the `Clock` token: `type T = Clock;`, `tuple P(Clock, Integer);`, `union U { A(Clock), B }`, `record S { xs: Option<Clock>; }` and `record Q { xs: Sequence<Clock>; }`. FR-230-AC-7's source-only fixture gives six refusals, all `not-yet-implemented`, ordered by span 221, 248, 298, 432, 345, 398: the record's `Clock` comes before the two function `Clock`s, so today's order is not source order. Through the spine (`compose`), a header with `quire.temporal.complete/v1` gives one E3 refusal, `unknown_profile`/`unsupported-selection`, and no assembly result (FR-230-AC-8 holds today). The same unit without that header line gives S3's `TimedNotChecked(Clock)`.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | medium | The gate refuses a `Clock` only as a parameter, a result or a record field. A source-written `Clock` in a type alias, a tuple element, a union member payload or a type argument (`Option<Clock>`, `Sequence<Clock>`) is outside it. Since FR-230's other rules apply only once the facet is selectable, nothing says what S3 does with such a form, and an implementer could compile `type T = Clock;` unchecked. Today each of these refuses (measured). QSpec #298 (head 2104b290f) reads an unqualified `Clock` in every type position as the clock type and gives a non-selecting unit `unsupported_construct` for the form. State the gate for every source-written `Clock` type reference, with its owning declaration and family. | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md:239-242 |
| FND-007 | low | The wait on making `quire.temporal.complete/v1` selectable has no owning ticket or Remaining-work entry, unlike STD-192. The "run now" sentence of "Clock fields and the domain package" and FR-230-AC-2's "run now" clause still read as runnable today, and only the new preamble and TC-685's scope line say they wait. | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md:151,166-168,271 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | deferred | The gate now records that the clock rules for a `Clock` outside a model object type are "not yet stated", and they cannot run until the facet is selectable (FR-230-AC-8), so stating them is out of scope here. |
| FND-005 | fixed | - |
| FND-006 | fixed | - |
| FND-007 | fixed | - |

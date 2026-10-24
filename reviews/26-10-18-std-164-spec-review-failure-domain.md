---
id: SR-12812
title: "failure-domain review of STD-164 state-free footprints (FR-155, FR-162, TC-1075)"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language@8b413f340493dc7b760c011401a08ed358196e58; spec/functional/FR-155-derive-and-enforce-read-and-write-footprints.md, spec/functional/FR-162-offer-reductions-through-transition-system-hooks.md, spec/test-cases/TC-1075-footprints-are-state-free-and-cover-disabled-transitions.md"
review_set: subset
---

## Summary

Ticket: STD-164 (PR quire-spec-language#969). Failure-domain analysis of the
claimed coarsening: footprints keyed on the declared types of the receiver and
reference parameters, related to population member types and referenced field
types "when either type conforms to the other". Checked against QSpec FR-383
"Write footprint" and "Read footprint" items 4 and 5 (quire-specification
fd7a16b), and against the type model's supertype structure (QSL FR-082,
QSpec FR-150, NFR-012), which admits several direct supertypes per type.

## Verdict

FAIL. The superset claim at FR-155:112-114 holds only when the supertypes of
a type form a chain. The type model admits several direct supertypes
(QSL FR-082:321 "reached through several direct supertypes"; QSpec FR-150
"for a type, each direct supertype", diamonds; NFR-012 "fifteen direct
parents"). In such a model, an object's most-specific type `O` can conform to
the declared type `D` and to a population member type `M` while neither of
`D` and `M` conforms to the other. Then the declared-type footprint omits a
location that FR-383 derives, and independence becomes unsound. The rest of
the derivation is unaffected: `Field` locations are per object, and
navigation and quantifier `AnyField` reads already use static types. Also
cross-checked PR #967 (`qsl-542-e12g-footprints`, 1ad86fa88): its
`populations_relating` for `deletes p` uses the same either-direction test,
so it has the same flaw for parameters. Its receiver path keys on the
receiver reference's `object_type`, which is exact.

## Examined Scope

```yaml
scope:
  - {id: FR-155, path: spec/functional/FR-155-derive-and-enforce-read-and-write-footprints.md, role: examined, excerpt: "Wherever FR-383 relates an object's type to a population or a field (...), the derivation SHALL take the declared type of the receiver or parameter, and SHALL relate it to each population member type and each field's referenced type when either type conforms to the other. ... It contains every location FR-383 derives from the object's most-specific type and may contain more, which keeps independence sound and makes it slightly coarser."}
  - {id: FR-155-AC-7, path: spec/functional/FR-155-derive-and-enforce-read-and-write-footprints.md, role: examined, excerpt: "With `Sub` extending `Base`, `Base` declaring `cancel()` framed `deletes self`, populations `bases: Base` and `subs: Sub` and a holder field `r: Reference<Sub>`, the footprint of `cancel(s)` writes `Membership{bases, s}` and `Membership{subs, s}` and reads `AnyField{Holder/r}`"}
  - {id: QSpec FR-383, path: quire-specification spec/functional/analysis/FR-383-explore-under-state-space-reduction.md, role: context_only, excerpt: "4. `Membership{P, receiver}` and `Membership{P, argument}` for each reference argument, for each population `P` whose member type the object's type conforms to; 5. for each `deletes` entry, `AnyField{U, g}` for every reference-valued field `g` of every type `U` whose referenced type the deleted type conforms to"}
  - {id: FR-082, path: spec/functional/FR-082-resolve-conformance-subsetting-and-redefinition.md, role: context_only, excerpt: "A redefinition whose target is reached through several direct supertypes is judged along each of them"}
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The "either type conforms to the other" relation does not give a superset of FR-383's locations when a type has several supertypes. Failure scenario: `C extends A, B`, with `A` and `B` unrelated. `A` declares `cancel()` framed `deletes self`. The populations are `as: A` and `bs: B`, and a holder field is `h: Reference<B>`. `B` declares `ping()` with pre `true`. For `c: C`, FR-383 gives `cancel(c)` writes `Membership{as,c}` and `Membership{bs,c}`, and reads `AnyField{Holder/h}`. The declared-type rule (declared `A`) gives only `Membership{as,c}`, because `A` and `B` are unrelated. So `cancel(c)` and `ping(c)`, which reads `Membership{bs,c}`, are judged independent although `cancel` disables `ping`. A transition that clears `h` is also left out of the candidate of the disabled `cancel(c)`, so a reduced run can miss a violation the unreduced run reaches (FR-383 agreement). Fix: key on the most-specific type that the transition identity's reference triple already carries (see SR-12813 FND-001). If declared-type keying is kept, relate `D` to `M` when some type of the package conforms to both, not when either conforms to the other. Add a multi-supertype vector to TC-1075. | spec/functional/FR-155-derive-and-enforce-read-and-write-footprints.md:105-114; quire-specification FR-383 items 4-5; spec/functional/FR-082-resolve-conformance-subsetting-and-redefinition.md:321 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | FR-155 now keys on the identity's object_type, exactly FR-383 with no widening. AC-7 adds the `C extends A, B` vector, and its sets match FR-383 items 1-6: no clauses, so items 1-3 and 6 give nothing; item 4 gives the memberships; item 5 gives AnyField{Holder/h} only, because C conforms to B and not to Base or Sub (25695410f). |

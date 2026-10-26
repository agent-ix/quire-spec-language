---
id: SR-1635001
title: "Code review of quire-spec-language PR #1635: dimension equivalences (580g)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@d2934f0b83ac4e6548acfbde998d8227ba8c7ba8; PR #1635 diff against qsl-580f-time-units: qsl-cst/src/{cst,grammar}.rs; qsl-forms/src/{lib,syntax,value}.rs; qsl-semantics/src/check/assemble.rs, assemble/{equivalence,units}.rs, check.rs, mod.rs, time_units.rs, temporal_clause.rs; qsl-semantics/src/value/{mod,unit}.rs; qsl-replay/src/spine.rs; qsl-semantics/tests/it/{dimension_equivalence,timed_declarations}.rs; qsl-forms/tests/it/value_forms.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: reviews
---
# Code review of quire-spec-language PR #1635

## Summary

Ticket: QSL-580, slice 580g, stacked on #1633. The Rust review lane is folded
into this review. Reviewed only this PR's diff against `qsl-580f-time-units`.

The grammar matches QSpec's shared grammar exactly: `dimension-equivalence`
and `ratio-term = (uint | exact-number), qualified-name`. The S2 form keeps
the names, numbers and spans as written.

The root-ratio arithmetic is right. One root of the first dimension is
`(y × b) / (x × a)` roots of the second. `sizes` walks each component from its
least key and gives each member its root's size in class roots. The step-7
`path` check compares the ratio a cycle implies with the ratio written.
`TimeUnits::convert` converts `magnitude × scale × from_size / to_size ×
to.reciprocal`. I checked this against AC-27 (3 F::sec → 3000 R::ms) and AC-28
(3 G::msec → 3; with root ratio 1 → 3000), and the tests assert the same
values. A unit of an unrelated `Time` still refuses `OtherTimeDimension`.

Gates: `cargo test -p qsl-semantics -p qsl-forms -p qsl-cst --all-features`
ran at this head through `locked-build.sh` with `QSPEC_DIR` at QSpec
`origin/main` ccad669c, and passed (exit 0).

## Verdict

**CONDITIONAL.** One medium bug: a declaration whose unit cannot be composed
is dropped silently. Low findings otherwise.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `Scope::unit` returns `Some(None)` for a package unit of a derived package dimension, or one whose target chain fixes no scale. `steps_three_to_five` then hits `let unit = unit.as_ref()?;` and returns `None` without pushing an error. The declaration is neither admitted nor refused, and the unit assembles with no error. Example: with `acme/rpc` declaring `Area = Length ^ 2` and its unit `m2`, `dimension R::Time ~ F::Time by 1 R::m2 = 1 F::s;` assembles cleanly. FR-142 step 4 refuses it `invalid_package`/`incompatible-definition` (`m2` is not a unit of `R::Time`). The fn doc ("nothing to decide past step 4") does not match: the early return happens before step 4. | qsl-semantics/src/check/assemble/equivalence.rs:220-245, 531-560 |
| FND-002 | low | Two smaller issues. `AssemblyCause::DimensionEquivalence.retained` keeps the retained declarations, units and nodes as display strings, so neither a caller nor a test can read which node or value a step kept. `over_classes` drops a failed `ratio.pow(exponent)` silently (`if let Some(power)`), which would leave a wrong `size` rather than an error. With positive ratios it cannot fail today. | qsl-semantics/src/check/assemble.rs:305-312; qsl-semantics/src/check/assemble/equivalence.rs:450-460 |

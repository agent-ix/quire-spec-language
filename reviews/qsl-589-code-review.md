---
id: SR-12157
title: "Code review of quire-spec-language PR #994: EN-1 refuses an unbounded root with RequiresBound"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@3719fc81355c02cf403bf78151983e5dc99a81eb; qsl-analyze/src/model_check/check.rs, qsl-analyze/tests/it/counter.rs, qsl-analyze/tests/it/model_check.rs"
review_set: subset
---
# Code review of quire-spec-language PR #994

## Summary

Ticket: QSL-589. Code review with the rust-review lane, scoped to
`git diff origin/main...HEAD` (3 files). Static review only: no cargo or make
was run, per the dispatch brief.

Examined:
- `check.rs`: the new `ModelCheckRefusal::RequiresBound(RequiresBound)`
  variant, the narrowed doc on `ModelCheckRefusal::NotSimulated`, and the
  `check_model_bounds` error mapping in `check_model`.
- `model_check.rs`: `run_with`, `unbounded_roots`,
  `a_population_with_no_universe_is_refused_before_exploring` (FR-336-AC-2),
  `a_proof_bound_does_not_make_an_unranged_parameter_finite` (FR-336-AC-3),
  and the updated match in
  `an_empty_or_unbounded_subject_and_an_undecided_form_explore_nothing`.
- `counter.rs`: `operation`, `document`, `document_with_ranged_parameter`,
  `document_with`.
- Context only: `qsl-eval/src/simulation/not_simulated.rs`
  (`RequiresBound`, `NotSimulated`, `check_requires_bound`),
  `qsl-eval/src/simulation/model.rs` (`check_model_bounds`, `domains`,
  `ModelSystem::new`), `qsl-foundation/src/bound.rs` (`DomainKey`),
  `qsl-eval/src/model_subject.rs` (`proof_bounds`), FR-336, TC-846, FR-126-AC-1.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `#[trace("TC-846", "FR-336-AC-2")]` over-claims. FR-336-AC-2 has two halves: no universe returns `RequiresBound` naming the population root, and with `{a, b}` the run "explores and returns FR-126-AC-1's outcome" over the ConfigVersion subject. The test runs the Counter deadlock-freedom item, not ConfigVersion's `ReachesTwo`, and its second half accepts any `Violated` or `Holds`. FR-126-AC-1 requires `Holds{Exhaustive}` with 15 product states (and a specific `Violated` lasso without granularity), so the test cannot fail on that half. The PR body admits it is "not asserted", but the tag still reports the whole AC as backed in the matrix. EN-1 does not decide `always eventually` yet (`FormNotDecided`), so the half cannot be earned in this slice: drop `FR-336-AC-2` from the trace (keep `TC-846`) and say in the doc comment which half the test covers. | qsl-analyze/tests/it/model_check.rs:757-778 |
| FND-002 | low | `ModelCheckRefusal::NotSimulated(Box<NotSimulated>)` still admits `NotSimulated::RequiresBound` (and `GeneratorMismatch`, `EmptyInitial`, `KeyEncoding`), so the old nested form stays representable even though `check_model` never builds it. `check_requires_bound` returns only `RequiresBound` or `Extent`, which matches the new doc ("could not classify the subject's extent"). Narrowing the variant to `Extent(Box<ClassifyFailure>)` would delete the old shape outright instead of leaving it unreachable. | qsl-analyze/src/model_check/check.rs:93-95 |
| FND-003 | low | The zero meter is offered as evidence that nothing is explored, but the test never shows that `ZERO` stops a bounded run. The universe run that follows uses `UNLIMITED`. The ordering is sound by reading `check_model`: `check_model_bounds` runs before `Run` is built and before `run.explore()`. Running the universe case under `ZERO` too, and asserting it is not `RequiresBound`, would make the meter a real oracle. | qsl-analyze/tests/it/model_check.rs:764-767 |
| FND-004 | low | AC-2 says the refusal names "the population root". The test checks one root of kind `Population`, not which key it is (`DomainKey::Population { member_type, ordinal }` of `counters`). AC-3 is stronger here: it compares keys across runs. | qsl-analyze/tests/it/model_check.rs:765-767 |
| FND-005 | low | The `operation` doc comment still says the parameter is one "no finite domain bounds", but the parameter type is now passed in and `document_with_ranged_parameter` passes the bounded `Level`. | qsl-analyze/tests/it/counter.rs:84-86 |

## Verdict

CHANGES REQUESTED, on FND-001 only. That is a one-line trace fix. Everything
else is low.

What holds:
- The refusal is direct. `check_model` maps `NotSimulated::RequiresBound(roots)`
  to `ModelCheckRefusal::RequiresBound(roots)` and keeps `NotSimulated` for
  `Extent` alone. Before this PR, `ModelCheckRefusal` had no consumer outside
  `qsl-analyze`; nothing in `qsl-route`, `qsl-replay` or `qsl-eval` matched it.
  The only match on the old nested form was the test at `model_check.rs:669`,
  which is updated. `qsl-route::Disposition::RequiresBound` and
  `qsl-eval`'s `NotSimulated::RequiresBound` are separate layer types that
  `explore_request`/`sample_request` still use, not shims. No compat layer.
- Soundness. The pre-check runs after admission and before `Run` is built, so
  nothing is expanded or sampled and the meter is only copied. `check_model`
  moves `request.subject.simulation` into `ModelSystem::new` and never reads
  `request.subject.proof_bounds`, so a `ProofBound` cannot make an EN-1 root
  finite. AC-3's middle case is a real regression oracle for that: it fails if
  anyone starts reading proof bounds there.
- FR-336-AC-3's binding is correct. The unranged `Integer` parameter is
  refused with one `Integer` root, and refused again with the same root set
  when a `ProofBound{IntegerRange{0,3}}` is keyed on that root. The
  `Int[0, 3]` (`Level`) variant explores. "Explores" is what AC-3 asks for, so
  `Holds | Violated` is enough there.
- Rust lane: `RequiresBound` is a `BTreeMap` wrapper, so it is not boxed and
  does not trip `large_enum_variant`. No new panic surface outside test
  helpers, and no `unsafe`, integer conversion or lock. `thiserror` message
  style matches the enum.

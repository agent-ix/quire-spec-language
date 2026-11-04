---
id: SR-20261104074838
title: "Code review of quire-spec-language PR #2261 (QSL-1481): FR-153-AC-14 population causes"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@55ffe5444; qsl-semantics/src/model/observation.rs, qsl-semantics/src/model/observation/document.rs, qsl-eval/src/value/expression/causes.rs, qsl-eval/src/value/expression/evaluate.rs, qsl-eval/src/value/expression/mod.rs, qsl-eval/src/value/mod.rs, qsl-eval/tests/it/named_population.rs, qsl-eval/tests/it/model_reference_queries.rs"
review_set: subset
---
# Code review of quire-spec-language PR #2261

## Summary

Ticket: QSL-1481. Code review with the rust-review lane, scoped to the PR diff at 55ffe5444. The PR splits FR-106 check 7's `incomplete_population` cause into `missing-required-artifact` (no row) and `incomplete-scope` (row not `complete`) in `check_population_completeness` and in qsl-eval's `named_members`, walks a declared population that only a reference value makes required, and adds `undefined_contract_code` for FR-090-AC-21. One sibling path that reuses check 7 was not updated, and two doc comments in a touched file still state the old rule.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `admit_protocol_reference_seeds` still returns `incomplete_population`/`incomplete-scope` for every required seed population that is not complete, including one with no row. It tests `environment.population_complete(population)`, which is `false` for an unlisted population. FR-106-AC-65, amended for QSL-1481 in #2198, requires `incomplete_population`/`missing-required-artifact` at that node when the seed's declaration has no row. The environment holds provenance and a check-7 closure for every declared population (`population_reference_closures` seeds from `facts.population_identities()`), so the unlisted case reaches this branch. No test traces FR-106-AC-65. Fix: split on `environment.population_completeness().get(population)` as `named_members` now does (`Some(false)` incomplete-scope, `None` missing-required-artifact), and add a test with an unlisted seed declaration tagged FR-106-AC-65. | qsl-semantics/src/model/observation.rs:495-501; spec/functional/FR-106-admit-snapshots-and-invocations.md:1280 |
| FND-002 | low | Two doc comments in the touched file still say a required population "not listed `complete`" is `incomplete-scope`, which is now wrong for a population with no row: `admit_trace_snapshot` and `NamedReads`. Fix: name both causes, as the updated `named_members` doc does. | qsl-semantics/src/model/observation.rs:1746-1748; qsl-semantics/src/model/observation.rs:2097-2099 |

## Verdict

**CONDITIONAL**: FND-001 must be fixed in this PR. The rest of the diff is sound:
- `check_population_completeness` (document.rs:3128-3153): the new pass appends only declared, still-unwalked required names in `BTreeSet` identity order, so an undeclared name still reaches check 8 unchanged. The `match` on `completeness.get` is exhaustive over the three cases with no catch-all. The new `views` parameter is passed at all six call sites.
- `named_members` (evaluate.rs:3158-3167) and `NamedPopulationIncomplete::catalog_code` make the same split. The added `listed` field joins the derived `Eq`/`Hash`, which is correct since it changes the reported cause.
- `undefined_contract_code` matches every `UndefinedReason` variant explicitly, so a new reason fails to compile until it is classified. It is a free function because `UndefinedReason` lives in the external `no_std` leaf crate.
- Tests: `a_missing_row_an_open_extent_and_an_uncovered_subtype_each_have_their_own_cause` checks each cause, the reference-required missing row (`others` through `link`), the walk-order case and the atom read. `tc_227_...` checks the projection.

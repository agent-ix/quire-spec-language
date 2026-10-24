---
id: SR-9098
title: "Code review of quire-spec-language PR #838: integration merge resolutions and post-merge fixes"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@ac65a4a03235a9e04df46b221fecff3c3082ef93; merges 2f2b02d50 and 0d53a13ee, fixes 58521dab5 and ac65a4a03; qsl-semantics/src/model/observation/document.rs, qsl-semantics/src/check/state_clause.rs, qsl-semantics/tests/it/{main.rs,observation_composites.rs,model_population.rs}, qsl-replay/src/result/wire/value.rs"
review_set: subset
---
# Code review of quire-spec-language PR #838

## Summary

Ticket: QSL-59. Code review with the rust-review lane folded in. Scope is only
what was not already reviewed: the conflict resolutions of merge 2f2b02d50
(main 2079684e5 + ref 68e8bc593), the second merge 0d53a13ee (ref c2275112d,
#837), fix commit 58521dab5, format commit ac65a4a03, and the semantic
interaction of main's #828, #675, #680, #756, #786, #665, #773, #763 and #767
with the ref. The ref's own PRs were not re-reviewed. Reviewer-only: no build.

Examined:
- state_clause.rs (#786 vs ref): `StateClauseDeclaration::view()` sets
  `complete_value` from `self.selection.identity ==
  crate::value::definition::DefinitionLock::pinned().entry(crate::value::definition::CatalogRole::Root).identity`.
  That is #786's exact condition with its submodule-qualified path, and
  `check_clause` passes it on through `.with_complete_value(form.complete_value)`.
  No flat `crate::value::Name` import remains under check/. The #786
  import-graph assertion is present in xtask/src/import_graph.rs (examined)
- document.rs single admission path: `admit_value` is the ref's
  explicit work stack (`Admit` enum, `work`/`done` vectors, no recursion).
  #680's Option arms (`Absent` gives `OptionValue::none`, `Present` pushes
  `SomeOf` + Enter/Value/Leave) and the sequence bound check (moved into
  `Admit::Sequence`, after the elements, the same order as main's
  `Step::Sequence`) are in it. main's second `admit_value` and its `Step`
  worklist are gone. `admit_parameters`/`admit_result` call `admit_form`, which
  calls `admit_value` with `Presence::Required`. Deep input stays iterative:
  #680's 10,000-deep `deeply_nested_owned_forms_admit_without_recursive_value_walks`
  passes in the ci3 log (examined)
- the three changed observation_composites assertions (`None` to
  `Some("result")`): the ref's `admit_result` adds `.with("field", "result")`
  to a nested admission refusal. FR-106 check 10 states code and cause only
  and names no `field` member for a result, so #680's `None` pinned that
  behaviour and was not a stated requirement. Code and cause are still
  asserted, and the field is now pinned more tightly (examined)
- tests/it/main.rs: the module list equals the union of both sides minus
  `field_presence`, which the ref deleted in #834. main did not touch that
  file after the merge base, so no main test was lost. Every declared module
  has a file (examined)
- tc_410 (ref test vs #763): p1 now declares members A,B and p2 declares C,E,
  so #763's coverage rule (a declared member type covers the queried type) is
  met. The members are still b1:B and e1:E, and the expected universes are
  still root types [A], [C, D] and [X]. D is not a declared member of p2 and
  is still in e1's universe, so the test still proves the universe is the
  supertype component, not the population. It is a faithful FR-084-AC-7 test (examined)
- #763 `all_instances` in the merged tree: it has one consumer
  (value/model_query.rs), which matches `IncompleteSubtypeClosure`
  explicitly. Its `conforms` call passes `Setting::AdmissionAncestorSteps` like
  the ref's other callers, so the ref's walk-ceiling code rule applies (examined)
- qsl-replay/src/result/wire/value.rs: the conflict was test-module
  placement only; the trace tags changed from `("TC-992", "FR-056-AC-23")` to
  `("FR-056-AC-23")` (examined)
- ac65a4a03: rustfmt-only re-wrap of the `SomeOf` push and one
  `assert_refused` call, with no token change (examined)
- gate: qslintegrate-ci3.log is a full `make ci` (includes `ci-all-features`
  clippy and test lanes) and ends `EXIT 0` at 22:05:12. ci2 ended 20:32:42,
  the fmt edits have mtime 20:33:06 and ac65a4a03 is 20:33:11, so ci3 ran on
  the final tree. qslintegrate-allfeatures.log (16:59) predates the second
  merge and is superseded by ci3's all-features lane (examined)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

**PASS.** All five resolutions keep both sides' behaviour. The single
admission path keeps #680's Option and sequence-bound semantics on a
non-recursive work stack, and the merged tree passes a full `make ci` on the
final head.

Recorded, not reported (below the reporting bar):
- low: the bound check now in the shared `admit_value` also applies to
  object-field and record-field sequences in snapshots, including post-state
  under `Witness`. Before the merge, neither side checked those. This is
  stricter and consistent with FR-106 check 6.5 (only out-of-range integers
  are post-state evidence), but no test covers an over-long object-field
  sequence.
- low: `admit_result`'s top-level `field_kind_matches` refusal carries no
  `field`, while a nested refusal carries `field: result`. This comes from the
  ref.

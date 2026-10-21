---
id: SR-12263
title: "Code review of quire-spec-language PR #1149: run_with_requirements call-aware requirement-item association (QSL-938)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@80a871782a879f25764737aa4b0c72a68bccef89; PR #1149 diff against origin/main: qsl-replay/src/call_requirements.rs; qsl-replay/src/lib.rs; qsl-replay/src/spine.rs; qsl-replay/src/spine/call.rs; qsl-replay/tests/call_requirements_facade.rs; qsl-replay/tests/call_requirements_root_only.rs; qsl-semantics/src/check/claims.rs; qsl-semantics/src/check/claims/tests.rs; qsl-semantics/src/check/mod.rs; tests/it/call_requirements_items.rs; tests/it/main.rs"
review_set: subset
---
# Code review of quire-spec-language PR #1149

## Summary

Ticket: QSL-938 (step 2; blocks IR-672 in quire-driver). One code-review
pass with the rust-review lane folded in, over `git diff origin/main...HEAD`
(11 files, +993/-2), against FR-395, TC-1089, FR-062-AC-14 and FR-075-AC-17.
Run: `cargo test -p qsl-replay --test call_requirements_facade` (8 passed)
and `arch-lint api-surface --qsl . --qsl-only` (all rules PASS, FR-100-AC-8
PASS). One throwaway probe test (not committed) reproduced FND-001.

Checked and clean:

- `run` keeps its signature and result. It now delegates to `run_reading`
  with a no-op read, so its behaviour is unchanged.
- One compile: `run_reading` reads `package.graph()` of the same checked
  package the call ran against, after `convert_outcome`. Every refusal
  before the call, including a cancel, and every outcome conversion fault,
  returns before the read. So the `requirement-owner-known` fault can only
  arise after the call, as FR-395 requires.
- The association rule: the members are the records whose owner equals the
  selected function name. They come in `BTreeMap` (ascending occurrence-key)
  order, and the request index is the enumerate position over the whole
  record map, which is FR-075's package order. Callee records are excluded,
  because the owner is the declaring function, not the caller.
  `operation-contract` records are built with `owner: None`. The probe and
  the AC-14 test confirm that `temporal-satisfaction` records have no owner
  either.
- The owner is stamped once, in `check/mod.rs`, from the function being
  checked (`claim.owned_by(&function.name)`). `key_claims` faults
  (`unkeyable`) when a claim has no owner. No other path feeds claims into
  `key_claims`, so this adds no new refusal for a valid program.
- Root facade: the entry, its three result types and every type the FR
  lists are re-exported at the root. `call_requirements_root_only.rs` names
  no `spine`, `qsl_semantics` or `quire_exact` path.
- Rust lane: no `unsafe`. The production code has no `unwrap`, `expect` or
  indexing. Faults are typed (`InternalFault` inside `RunRefusal::Fault`).
  The `too_many_arguments` allows carry reasons. `with_owner` is gated on
  `cfg(any(test, feature = "test-support"))`, and only dev-dependencies
  enable that feature.

## Verdict

FAIL, not mergeable as it stands. There is one major finding.
`associate` checks a `value-validity` owner with `graph.callable`, which
excludes clause-built functions. An operation `body` with a scalar
application produces a `value-validity` record whose owner is such a
function, so `run_with_requirements` faults on every call of a valid
package that contains one. There is also one medium finding: `same_call` is
not reflexive for population arguments.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `associate` accepts a `value-validity` owner only when `graph.callable(name)` is `Some`, and `callable` filters on `callable_by_name`. The functions dispatch linking synthesizes from a unit's `body` declarations and `pre` clauses (`FunctionDeclaration::clause`, `callable_by_name = false`) are checked by `ValueFunctionFamily` in the same loop, so their scalar applications produce `value-validity` records that this PR stamps with the clause function's name. Reproduced at this head: a unit with `body renamed using v on Config::ConfigVersion::add(n) { n + deref(self).versionNumber }` and `function inc ... { x + 1 }` checks fine, with records `ValueValidity owner=Some("ix://example/config-version/ConfigVersion/add") callable=false` and `ValueValidity owner=Some("inc")`. Then `run_with_requirements` for `inc` returns `Fault { stage: "spine-run", invariant: "requirement-owner-known" }`. Every call of every package with such a body faults, which breaks the outcome independence and association rules for valid input. Fix: check the owner against all of the package's declared functions, clause functions included (a public `CheckedGraph` accessor over `functions`/`declares`), not only the callable ones. Add a TC-1089 test over a unit with an operation body holding a scalar application, asserting that a `function` call returns its members and that the body's record belongs to no function's call. If FR-062 means a body's record to have a different owner spelling, or no owner, settle that in the spec in this PR. | qsl-replay/src/call_requirements.rs:206-209; qsl-semantics/src/check/mod.rs:3872-3880; qsl-semantics/src/check/mod.rs:1415-1419; qsl-semantics/src/check/dispatch_link.rs:385 |
| FND-002 | medium | `CallRequirements::same_call` compares argument values with `quire_exact::compare_keys(..) == Some(Equal)`. `compare_keys` has no arm for `Population` (or `Float`) and returns `None`. FR-100's `argument_value` admits `Value::Population` for a `Population` parameter, so a call with a population argument is never `same_call` to itself: `c.same_call(&c)` is false. FR-395 Terms define a call's identity and equality as its function name and its set of (parameter, value) pairs, and an equality must be reflexive. The doc comment ("a value with no key order (a float, a population) is never the same call as another") documents behaviour the FR does not state. Fix: compare values with quire-exact's value equality (`plan_equality`/`planned_equality`), or add an explicit population-identity arm. Correct the doc comment, and add a test that a population-argument call is the same call as itself and as its reordered twin. | qsl-replay/src/call_requirements.rs:84-102; qsl-replay/src/spine/call.rs:686-689 |

## Dispositions

| Finding | Disposition |
| --- | --- |
| FND-001 | fixed: `associate` checks the owner with the new `CheckedGraph::declares_function`, which includes clause-built functions. FR-062 "Owner" and FR-395 "Broken ownership" now say a function synthesized from an operation `body` or `pre` clause is a function of the package; new FR-395-AC-12 and TC-1089 step 8 are tested by `a_clause_function_is_a_known_owner_and_no_fault` in `qsl-replay/tests/dispatch_link_compile.rs`, which faulted before the fix. |
| FND-002 | fixed: `same_call` compares values with the kernel's `planned_equality` (a population by its id, since the kernel refuses a population pair); the doc comment is corrected; `a_population_argument_is_equal_to_itself_and_not_to_another` covers it. |

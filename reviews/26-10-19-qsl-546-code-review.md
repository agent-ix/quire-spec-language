---
id: SR-13119
title: "Code review of quire-spec-language PR #1035: QSL-546 E13d, population-valued refinement mapping (FR-139)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@214d15b31; PR #1035 (git diff origin/main...HEAD, base f84e65de6, merge-base 337871901): qsl-cst/src/{cst,grammar,token}.rs, qsl-forms/src/{lib,syntax,value}.rs, qsl-foundation/src/diagnostic.rs, qsl-semantics/src/check/{check,check/typing,refinement,ir,lowering,refusal,checked_dispatch,family,layer,assemble,claims,hyper_clause,hyper_typing,references,mod}.rs, qsl-semantics/src/model/{object_environment,observation/initial}.rs, qsl-eval/src/value/{mod,expression/mod,expression/evaluate,expression/causes}.rs, qsl-replay/src/spine/refinement.rs, tests/support/ring_queue.rs, qsl-{cst,forms,semantics,eval}/tests/it/*"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-139
    type: reviews
---
# SR-13119: Code review of quire-spec-language PR #1035

## Summary

Ticket: QSL-546 (E13d). Code review with the rust-review lane folded in.

The PR adds the six population-valued forms (`count`, `sum`, `exists`,
`all`, `only`, `seq`) and a concrete population name as a value. It covers
S1/S2 (`PopulationCall`, `PopulationBinder`, the `..` token, and the
`ExprNode::Population` and `ExprNode::Seq` nodes), S3 typing inside refinement
rows, a recorded size-dependent bound per row (`PopulationUse`), and
`CheckedRefinement::admit_universes`. It also adds evaluation through the one
clause evaluator (`evaluate_row`, `NodeKind::PopulationObjects`,
`NodeKind::Seq`, `Visit::Only`), population membership filled by
`observe_state`, and a new undefined reason `only-not-one`.

Gates run locally in a scratch copy with its own `CARGO_TARGET_DIR` (not `make ci`):

- `xtask string-edge`: clean.
- `arch-lint api-surface --qsl-only`: T12-B to T12-F (T12-C included) and
  FR-100-AC-8 pass. T12-A is skipped because it needs the CG clone.
- `arch-lint canonical-encoder`: pass.
- `arch-lint qualified-core` (import graph direction, ambient input): pass.
- Probe tests, added in the scratch copy only (`qsl-semantics --all-features
  --test it`, `qsl-eval --test it`). The results are cited in the findings.

There is no compatibility shim, re-export or fallback. The PEG grammar
change is safe: `Peek` only guards, and `Choice` backtracks, so the existing
`exists(x in e: b)` and `count<T>(...)` collection calls still parse (probe:
`exists(s in R::slots: s.value = 1)` checks). `..` was not a token sequence
anywhere before. The new `ExprNode` and `NodeKind` variants are handled in
every exhaustive match. Lowering refuses them, which is correct because S4
emits no refinement node yet. `admit_universes` has no production caller.
That is by design, and the PR body says so. See the gap analysis
(SR-13120) for what QSL-544 has to wire.

The coder's open questions are classified in SR-13120 and in the planner
note: (1) `only-not-one` is a spec gap, (2) the `sum` bound is a spec gap
and the coder's bound is the sound one, (3) the `mod` row is a pre-existing
language rule plus a spec gap, and (4) shared-grammar.md is a QSpec spec
gap.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `PopulationUse::fits_at` compares a population row's admitted type to its use with `actual == self.required` for `PopulationValue::Objects`. So `cells = R::slots` into `Set<Reference<Cell>>[0, 2]` is refused `PopulationBound` for any universe smaller than 2: `admit_universes` returns `Err` at `slots = 0` and `slots = 1`, and `Ok` only at exactly 2 (probed). `Set[0, 1]` conforms to `Set[0, 2]`. FR-139 says "refusing ... when a result's bounds do not conform to its use", which is conformance, not equality. A subject with fewer objects than the abstract field's maximum is wrongly refused. The fix is to check bound containment (or `conforms`) as the `Int` arm already does. No test covers a universe below the bound. | qsl-semantics/src/check/refinement.rs:334-344 |
| FND-002 | medium | A concrete population name outside a refinement declaration refuses `missing_declaration`/`missing-name`, not `unsupported_construct`/`expression-form`. Probe: `invariant ... { size(R::slots) = 2 }` gives `MissingDeclaration/missing-name` at `R::slots`. FR-139 Behavior says "A population name or one of the six forms in any other clause body SHALL refuse `unsupported_construct`/`expression-form` at its span", and the PR body says "Elsewhere they refuse `unsupported_construct`/`expression-form`". The `ExprNode::Name` arm consults `population_named` only when a `PopulationScope` is set, so outside a refinement the name falls through to `resolve_target`. | qsl-semantics/src/check/check/typing.rs:870-872 |
| FND-003 | medium | `seq(i in lo .. hi: e)` is not refused at S3 outside a refinement. The `ExprNode::Seq` arm has no `self.populations.is_none()` guard, unlike `population_form`. The form types, and is refused only later by lowering as `unknown_required_feature`/`unsupported-feature` (probe: `invariant ... { size(seq(i in 0 .. 1: i)) = 2 }`). This contradicts FR-139 ("one of the six forms in any other clause body SHALL refuse ... `expression-form`"), QSpec FR-376-AC-4, and the PR body. A clause path that is not lowered (for example evaluation of an unemitted body) would also accept it. Only `count` is tested outside a refinement. | qsl-semantics/src/check/check/typing.rs:1223-1242; qsl-semantics/src/check/lowering.rs:3448-3455 |
| FND-004 | medium | Metering does not match "charging one work unit per object visited and per `seq` element". `sum` and `only` with a `where` clause lower to a `Filter` query feeding a `Sum`/`Only` query, so each matching object is visited twice. Probes: `items` at `head = 1, size = 2` charges 8 `collection.visit` units, against 6 (2 elements + 2 x 2 objects). `sum(s in R::slots where s.value = 1: s.value)` over two matching slots charges 4, against 2. The only metering test (`a_form_charges_one_work_unit_per_object_visited`) uses `count`, which does not filter. This matters because FR-140-AC-5 and every budgeted run depend on these counts. Either fuse the filter into the `Sum`/`Only` visit, or state the charge in FR-139. | qsl-semantics/src/check/check/typing.rs:2674-2694; qsl-eval/src/value/expression/evaluate.rs:2770-2792 |
| FND-005 | medium | The PR adds the public `qsl_eval::value::evaluate_row(graph, row, current, arguments, meter)`. PR #1032 (origin/qsl-544 @ 11a735627) adds `evaluate_refinement_row(package, row, current, bindings, meter)`, which has the same body (`Machine::with_pre(.., None, Some(&empty), ..).run(row.body(), row.slots(), ..)`), at the same insertion point, and reflows the same `pub use` list. `git merge-tree origin/qsl-544 HEAD` reports content conflicts in `qsl-eval/src/value/expression/mod.rs` and `qsl-eval/src/value/mod.rs`. Whichever PR lands second must delete one of the two entry points, not keep both. The other overlap points do not conflict: `ObjectEnvironment::population_members`, `observe_state`, and `tests/support/ring_queue.rs` versus `qsl-replay/tests/it/refinement_fixture.rs`. | qsl-eval/src/value/expression/mod.rs:107-129; qsl-eval/src/value/mod.rs:18-25 |
| FND-006 | low | `Checker::population_scope()` runs inside `check_row`. It re-resolves the concrete model's populations and object types for every field row and step argument, then clones the scope into the `Typer`. The scope depends only on the refinement, so compute it once per declaration. | qsl-semantics/src/check/refinement.rs:799-823; qsl-semantics/src/check/refinement.rs:1257-1274 |
| FND-007 | low | `population_of` returns `String::new()` when the `only` source is not a population under filters. The `OnlyNotOne` payload then carries an empty `population` instead of raising an evaluator invariant. Typing makes this unreachable today, but if it is ever reached it produces a malformed diagnostic instead of a BROKEN stop. | qsl-eval/src/value/expression/evaluate.rs:368-384 |

## Verdict

**Not mergeable yet.** FND-001 is a wrong refusal on the main admission path,
and FND-002 and FND-003 contradict an FR-139 SHALL that the PR body claims is
built. FND-004 and FND-005 should be fixed in the same round. FND-005 needs a
coordination decision with the QSL-544 lane about which entry point
survives.

The rest is sound:

- S3 typing of `count`, `exists`, `all` and `only`.
- `seq`'s binder `Int[lo_min, hi_max]` and its maximum
  `max(0, hi_max - lo_min + 1)`, which matches QSpec FR-376.
- The `sum` admission bound `[min(0, n*lo), max(0, n*hi)]`.
- The `Count`/`Sum` `Int` containment.
- `only` undefinedness before the body reads.
- The `seq` iteration, which is heap-stacked with no recursion.
- `observe_state` membership in key order.
- The refusal codes for `PopulationBound` and `PopulationFormNotChecked`.
- The rust-review checks: no `unwrap` or `expect` in production paths,
  checked integer conversions (`u64::try_from(..).unwrap_or(u64::MAX)` and
  `to_u64` with refusal), and no new `unsafe`.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | still-open | Fix commit e8a43b6e6 fixes this for state clauses only: `with_known_populations` is wired in `state_clause::check_clause` alone. Probes at e8a43b6e6 show `R::slots` in a `temporal` clause body (`always holds(size(R::slots) = 2)`) still refuses `missing_declaration`/`missing-name`, and so does `R::slots` in a `function` body. A temporal clause body is a clause body under FR-139, so this is still a defect there. A function body is not clearly a clause body: the six forms and `seq` already refuse `expression-form` there, and only the bare name differs. Making the guard intrinsic to every non-refinement `Typer` would cover both. |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | deferred | Planner re-plan: QSL-544 (#1032) lands first. After it lands, #1035 must rebase and delete `evaluate_row` in favour of `evaluate_refinement_row`, or the reverse. At e8a43b6e6, `git merge-tree origin/qsl-544 HEAD` still reports content conflicts, in `qsl-eval/src/value/expression/mod.rs` and now also `qsl-eval/src/simulation/model.rs`. |
| FND-006 | fixed | - |
| FND-007 | fixed | - |
| FND-002 | fixed | - |

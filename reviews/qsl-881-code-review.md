---
id: SR-12177
title: "Code review of quire-spec-language PR #1003: FR-393 time language S1/S2 forms"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@e65c0d627fea52f1cdf034401bf7cd4f91f5c5a3; qsl-cst/src/cst.rs, qsl-cst/src/grammar.rs, qsl-cst/src/parser.rs, qsl-cst/tests/it/complete_grammar.rs, qsl-forms/src/dispatch.rs, qsl-forms/src/lib.rs, qsl-forms/src/syntax.rs, qsl-forms/src/temporal_clause.rs, qsl-forms/src/time_model.rs, qsl-forms/src/value.rs, qsl-forms/tests/it/*.rs, qsl-replay/src/spine.rs, qsl-replay/src/spine/refinement.rs, qsl-semantics/src/check/{assemble,family,mod,refusal,region,temporal_clause,type_form}.rs, qsl-semantics/src/check/temporal_clause/tests.rs, qsl-semantics/tests/it/timed_declarations.rs"
review_set: subset
---
# Code review of quire-spec-language PR #1003

## Summary

Ticket: QSL-881 (QSL-580a). Code review with the rust-review lane, scoped to
`git diff origin/main...HEAD` (28 files, head `e65c0d627`, which merges main
after #1000). Static review only: no cargo or make was run, per the dispatch
brief. QSpec's shared grammar (`time-decl`, `time-invariant`, `clock-bound`,
`urgent-decl`, `type-ref =/ 'Clock'`, `timed-interval`, `interval`,
`quantity-literal`) was read as the normative source.

Examined:
- S1: the five new productions, `Terminal::ClockType` and
  `Terminal::TypeIdentifier`, the type-introducing declarations (`enum`,
  `record`, `tuple`, `union`, `type`) switched to `type_ident()`, the
  `TemporalUnary`/`TemporalRelation` rewrite from `opt(Interval)` to two
  ordered alternatives, the widened `Interval` (lower `uint`, upper
  `QuantityLiteral | uint | *`), and `parser::interval_bounds`.
- S2: `time_model.rs` (`time_member`, `time_declaration`, `time_invariant`,
  `clock_bound`, `urgent_declaration`), `timed_interval_form`, the
  `OperatorInterval` loop in `Formula::operator`, `LeadingTokenKind::{Time,
  Urgent}` and `from_spelling`, `BuiltinType::Clock` in `type_head`.
- S3: `Unit::timed` and its refusal at the top of `PackageDeclarations::new`,
  `check_names`' `Clock` arm, `resolve_builtin`'s `Clock` arm and
  `TypeFormFault::ClockNotChecked`, `check_clause`'s `Timed` arm,
  `formula_horizon`, `TimedConstruct`, the `TemporalCause`/`CheckCause`/
  `AssemblyCause` code and tag tables; `qsl-replay` messages and limit maps.
- Tests: `time_forms.rs` (all seven), the updated `temporal_clause_forms.rs`
  helpers, `timed_declarations.rs`,
  `a_timed_interval_refuses_not_yet_implemented_at_the_interval`.
- Context only: FR-393, TC-1078, FR-325, TC-835, FR-337 emission path
  (`build_interval`, `build_formula`, `CheckedOperator`), `probabilistic::check`,
  `typing.rs`'s `ExprNode::Quantity` refusal from #1000.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Two S3 stopgap tests carry `#[trace("TC-1078")]`, but TC-1078's scope is FR-393-AC-1 to AC-7 (S1 parse and S2 build) and none of its seven steps describes an S3 `not-yet-implemented` refusal. The tag adds the two tests to TC-1078's evidence for behaviour the TC does not state. The repo's precedent tags such a stopgap with the owning S3 test case (`probabilistic_declarations.rs` uses TC-620). Tag them with the FR-230/FR-234 test case that will replace them, or leave them without a TC. No AC is over-claimed. | qsl-semantics/tests/it/timed_declarations.rs:33, qsl-semantics/src/check/temporal_clause/tests.rs:466 |
| FND-002 | low | The FR-393-AC-6 test's "no facet selected" side declares `profile t = "quire.value.complete/v1"`, which is a complete facet selection, and its "complete facets" side selects only `quire.temporal.timed/v1`. The doc comment says "no facet selected". The binding still holds, because the test fails if S2 gated a time form on the timed facet. But the fixture does not match the AC's "no facet" case. Use a header with no time-related selection (as FR-392-AC-6's test does with `quire.state.core/v1`), or correct the doc comment. | qsl-forms/tests/it/time_forms.rs:399-410 |

## Verdict

APPROVE. No high or medium findings; both lows are recorded and do not block.

What holds:
- Nothing timed compiles silently. Every `TimeSource`, `TimeInvariant` and
  `Urgent` form is collected in `Unit::new` and pushed as
  `AssemblyCause::TimedNotChecked` before any other assembly work. The first
  `if !errors.is_empty()` return in `PackageDeclarations::new` is reached with
  no `Ok` path before it. `Unit::new` is the only consumer of the three new
  `DeclarationForm` variants in `qsl-semantics`. A `Clock` type refuses in
  both `check_names` (assembly) and `resolve_builtin` (any check-time
  resolution, such as a temporal `over` parameter, via `type_fault_class` to
  `unsupported_construct`/`not-yet-implemented`). A `Timed` operator interval
  refuses in `check_clause` before `build_formula`, `formula_horizon` and
  emission, under bounded and unbounded profiles alike. The `Timed` arm in
  `formula_horizon` cannot be reached. Probabilistic event formulas that hold
  a timed interval go through `probabilistic::check`, which refuses every
  probabilistic declaration. Quantity literals keep #1000's typing refusal.
- S2 is defensive on its own. `interval_form` takes only direct `Integer`
  tokens and `timed_interval_form` needs exactly two `QuantityLiteral` nodes,
  so a mixed or overflowing interval that reached S2 without S1's diagnostic
  would still be refused, not mis-built.
- Emission identity is unchanged. Step intervals go through
  `OperatorInterval::Steps(interval_form(..))` and then the same
  `build_interval` into `TemporalInterval`. Lowering and emission read only
  the checked `TemporalInterval`, never `TemporalOperatorForm.interval`. The
  CST shape of `TemporalUnary`/`TemporalRelation` is unchanged (keyword
  tokens, then an `Interval` node, then the operand), and so are operator
  spans. The FR-337/#1008 emission tests are not touched by the diff. `[0,5]`
  and `[3,*]` still parse as `Interval`, because the `QuantityLiteral` upper
  alternative needs a unit name after the digits.
- `parser::interval_bounds` puts each bad bound's diagnostic at that bound's
  span. It runs on every CST, because `parse` is the only `LosslessCst::new`
  site. Because `Interval` is shared, it also covers protocol `within`,
  `await` and state-graph intervals.
- `time`, `urgent`, `dense`, `tick` and `period` use `literal()` and are never
  reserved. `Clock` is excluded only by `TypeIdentifier`, which otherwise
  matches `Identifier`'s own test. `ClockType`'s `::` lookahead keeps
  `Clock::T` a qualified name. The `terminal` rule from #961 is untouched and
  is still in the grammar corpus.
- The dispatch reads the production that S1 built, not the tokens, so
  `time invariant` and `time` reach exactly one builder each.
- The declared departures (`TypeForm{head: Builtin(Clock)}` for
  `TypeForm::Clock{span}`, `DeclaredName`/`NameForm`/`ExpressionForm`, and
  `Box`ed members) are listed in the PR body and keep the meaning of the spec.
- #1000's `ExactValueForm`/`QuantityForm` are reused. No duplicate definition,
  `DecimalScaleBeyondSource` or `rational_bits` remains in the tree.
- Rust lane: no `unsafe`, no new panic outside tests, and no `as` casts.
  Large variants are boxed. `interval_bounds`' `let [..] = significant[..]
  else { continue }` skips a malformed node, which is safe because S2 refuses
  that node itself.

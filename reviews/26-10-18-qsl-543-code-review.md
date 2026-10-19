---
id: SR-12814
title: "Code review of quire-spec-language PR #970: refinement declaration S1/S2/S3 (FR-135, FR-136, FR-137)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@3d0b0a27672bd3af0661f9c502be871147439284; PR #970; qsl-cst/src/{cst.rs,grammar.rs,lib.rs}, qsl-forms/src/refinement.rs, qsl-semantics/src/check/{refinement.rs,mod.rs,assemble.rs,refusal.rs,region.rs}, qsl-replay/src/spine*.rs"
review_set: subset
evaluated_revision: "3d0b0a27672bd3af0661f9c502be871147439284"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-135
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-137
    type: reviews
---
# Code review of quire-spec-language PR #970

## Summary

Ticket: QSL-543. Code review with the rust-review lane folded in. Diff:
`git diff origin/main...HEAD` (5 commits, 27 files). I ran the focused
refinement tests (qsl-cst, qsl-forms, qsl-semantics `--test it refinement`
with `test-support`) in a separate target dir. All 28 passed.

What I checked: the grammar for the `->` compound and the `recursion_limit`,
the S2 form builder, the S3 checker, the not-emitted refusal, how refusals
are classified in the spine and regions, and the fairness defaults.

## Verdict

No high finding. The S3 checker is structured well: it reports every refusal
in source order, it stops on resource exhaustion and cancellation, and the
packages-build refusal (`RefinementNotEmitted`) is reached whenever the
`with_unwritten_refinements` test flag is not set. The `->` compound only
changes how the leaf budget groups `-` with a directly following `>`. No
existing production spells `->`, and the `every_declarative_compound_is_one_budgeted_cst_leaf`
test covers it. `recursion_limit = "256"` only affects macro expansion in
qsl-cst and leaves room for the productions the sibling lanes (QSL-550/568)
add. The only expected conflict is textual (the exhaustive production lists),
and the compiler catches it. Reading a missing granularity as `Whole` matches
ADR-020 RM-6, FR-137 and FR-123. The FR-310 sentence is awkward but does not
contradict them.

One medium finding: the expression typing uses a stricter conformance than
the spec's. The rest are low.

Owner questions (not guessed): (1) QSpec FR-375 has only `weak` in fairness
rows, while QSL ADR-020 §1 and FR-137 have `weak | strong`. The code follows
QSL's spec. (2) A fairness row with no kind is accepted and read as `Weak`
(FND-004).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Field rows, step receivers and step arguments are typed with `Typer::check_as`, which ends in `Typer::coerce`. That function admits a reference only when the type is identical. It has no `Reference`/subtype arm, so it does not apply the `TypeEnvironment::conforms` (ADR-016 SC-2) that FR-135 and FR-136 require. Example: an object row maps `Spec::Counter from Impl::Counter`, and `Impl::SubCounter <: Impl::Counter` has its own operation. `step Impl::SubCounter::op -> Spec::Counter::inc(self)` refuses `ill_typed`/`type-mismatch`, because `self: Reference<SubCounter>` is not equal to the lifted `Reference<Impl::Counter>`. The same happens for a field `Reference<Q::Queue>` mapped by an expression of a subtype of `R::Ring`. The other direction is too loose: `coerce` admits an `Int[0,10]` expression into an `Int[0,3]` field through a range-obligated `Coerce` node, where SC-2 conformance refuses it. The same file also has its own `conforms` helper with reference subtyping, used only for `binds_result`. So the check has two different conformance relations. Fix: type through the argument/upcast path (`Expected::Argument`), or check the inferred type with one shared SC-2 conformance. | qsl-semantics/src/check/refinement.rs:828-845, 930, 1115-1131, 398-408; qsl-semantics/src/check/check.rs:1509-1543 |
| FND-002 | low | A concrete step row whose left side resolves to `OperationLookup::Ambiguous` is skipped with no refusal (`let ... else { continue; }`). FR-136 says a row that names no concrete operation refuses `missing-name` at its span. The author sees only `MissingRow` refusals for the inherited operations, and nothing at the row they wrote. | qsl-semantics/src/check/refinement.rs:972-978 |
| FND-003 | low | Rows can be dropped without any refusal. `Side::populations` drops a population whose `type_identity_segment` is `None` (`filter_map`), so it gets no `MissingRow`. `object_row` returns `None` with no refusal when `types.attributes(target.id)` is `None`. Either one would let a refinement check with an abstract population or object row silently unmapped. These should be `fault(..)` internal faults, as the key lookups next to them are. | qsl-semantics/src/check/refinement.rs:329, 860 |
| FND-004 | low | The fairness row grammar makes the kind optional (`opt(weak|strong)`), and S3 reads an unwritten kind as `Weak`. ADR-020 §1's surface line `assume fair (weak | strong) [each | whole] ...` and FR-137's Inputs (`fair <kind> [<granularity>]`) both require a kind. The CST test explicitly accepts `ensure fair Spec::Counter::inc;`. This is a spec question for the owner. | qsl-cst/src/grammar.rs:1697; qsl-semantics/src/check/refinement.rs:1200-1203; qsl-cst/tests/it/refinement_declaration.rs:59 |
| FND-005 | low | A population or object row whose name spells nothing in either model refuses `missing_declaration`/`missing-name` at the name. FR-135 says a row "whose left side is not an abstract population, or whose right side is not a concrete population, SHALL refuse `invalid_model_binding`/`malformed-declaration` at the row's span". Only the swapped-sides case gets that code. This may be a spec-reading question. | qsl-semantics/src/check/refinement.rs:544-571, 784-788 |
| FND-006 | low | `maps_complete` is `findings.refusals.is_empty()`, so an unrelated `WrongProfile` refusal also stops every step argument from being typed. The step-argument diagnostics are lost when the only error is the profile. Smaller points: a type error whose location maps to no span falls back to `Span{0,0}` (byte 0), and a unit with both a `terminal` and a `refinement` reports only the terminal's not-emitted refusal. | qsl-semantics/src/check/refinement.rs:763, 940-942; qsl-semantics/src/check/mod.rs:2181-2196 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | low | The FND-001 fix replaces `check_as` with `infer` plus the file's own `conforms`. `conforms` has no `Composite` arm, and its fallback is equality. `Typer::coerce` admitted a record into a record type it conforms to (FR-359, `composite_conforms`), and state-clause bodies still do. Since the fix, an abstract field of record type `R` mapped by an expression of a conforming record subtype refuses `ill_typed`/`type-mismatch`. The same equality fallback applies to collections whose element types differ only by an integer range or a subtype. This is an edge case and no fixture uses records. Fix: add a `Composite` arm (`types.composite_conforms`), and either recurse element-wise for collections or state that they must be invariant. | qsl-semantics/src/check/refinement.rs:415-425, 986-996; qsl-semantics/src/check/check.rs:1518-1526 |

## Dispositions

Round 1, reviewed at `84309fc22546a5adec7ac758a9dfb2ed5513d3ec`.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 84309fc22546a5adec7ac758a9dfb2ed5513d3ec |
| FND-002 | fixed | 84309fc22546a5adec7ac758a9dfb2ed5513d3ec |
| FND-003 | fixed | 84309fc22546a5adec7ac758a9dfb2ed5513d3ec |
| FND-004 | deferred | owner question (planner holds: is the fairness kind optional; QSpec FR-375 `weak` vs QSL `weak \| strong`) |
| FND-005 | deferred | owner question (planner holds: `missing-name` vs `malformed-declaration` for a row naming nothing) |
| FND-006 | fixed | 84309fc22546a5adec7ac758a9dfb2ed5513d3ec |

FND-001: every row expression is now typed with `typer.infer(.., Some(required), ..)` and then checked with the file's one `conforms` (references by `TypeEnvironment::conforms`, integer ranges by containment, options element-wise). The range-checked `Coerce` path is gone. Three new tests cover it: `a_subtype_receiver_conforms_to_the_supertype_it_lifts_to` (checks), `a_supertype_receiver_does_not_conform_to_the_subtype_it_lifts_to` (refuses `type-mismatch` at `self`) and `an_integer_range_must_lie_inside_the_range_it_defines` (`Int[0,3]` into `Int[0,10]` checks, the reverse refuses at `self.value`). The fix also dropped record conformance (FND-007, new).

FND-002: `operation_named` now refuses `OperationLookup::Ambiguous` at the name with `RefinementCause::Ambiguous` (`ambiguous_declaration`/`ambiguous-name`, the same catalog pair `protocol_controls` uses). The concrete and abstract sides share this path, and `check_limit` classifies the new arm. No test covers it. That is acceptable for a low finding: the arm is a direct match on the lookup result with no logic of its own. A diamond fixture would now be cheap to add, because `object_type_under` accepts several supertypes.

FND-003: `Side::populations` and `Side::object_types` return `Result`, and `or_fault` turns a broken invariant into an internal fault. A missing attribute list faults as `abstract-object-type-has-attributes`. Nothing is dropped silently any more.

FND-006: `maps_complete` now counts only refusals after the header (`baseline`), so a `WrongProfile` refusal no longer suppresses step-argument typing. A row error with no span is now a fault, not `Span{0,0}`. Terminal and refinement not-emitted refusals are both reported, which `a_terminal_and_a_refinement_are_each_refused_not_emitted` tests.

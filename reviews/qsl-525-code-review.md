---
id: SR-12135
title: "Code review of quire-spec-language PR #960: S3 fairness, property forms and terminal declarations"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@f05318e1b0dac554afaa789bc0fa39ed3f521492; Cargo.lock, qsl-eval/src/temporal/outcome.rs, qsl-eval/tests/it/model_simulation.rs, qsl-eval/tests/it/temporal_trace.rs, qsl-replay/src/spine.rs, qsl-replay/src/spine/refinement.rs, qsl-semantics/src/check/assemble.rs, qsl-semantics/src/check/assemble/tests.rs, qsl-semantics/src/check/check.rs, qsl-semantics/src/check/lowering/model.rs, qsl-semantics/src/check/mod.rs, qsl-semantics/src/check/refusal.rs, qsl-semantics/src/check/region.rs, qsl-semantics/src/check/state_clause.rs, qsl-semantics/src/check/temporal_clause.rs, qsl-semantics/src/check/terminal.rs, qsl-semantics/tests/it/main.rs, qsl-semantics/tests/it/model_operations.rs, qsl-semantics/tests/it/temporal_fairness.rs, qsl-semantics/tests/it/terminal_declarations.rs, spec/functional/FR-382-build-the-terminal-declaration-form-at-s2.md, spec/test-cases/TC-1059-s2-builds-the-terminal-declaration-form.md"
review_set: subset
---
# Code review of quire-spec-language PR #960

## Summary

Ticket: QSL-525. Code review with the rust-review lane, scoped to
`git diff origin/main...HEAD` (22 files). Static review only: no cargo or make
was run, per the dispatch brief.

Examined:
- `temporal_clause.rs`: `checked_fairness`, `resolve_fairness_operation`,
  `property_form`, `is_reachable_invariant`, `is_closed` and
  `in_safety_fragment`, against FR-123 and ADR-018 IV-4.
- `terminal.rs`, `assemble.rs` `terminal_declarations`, and the terminal loop
  in `check/mod.rs` `PackageDeclarations::check`, against FR-124 and FR-382.
- `refusal.rs` (new `TemporalCause` variants and `CheckCause::Terminal`),
  `region.rs` (terminal spans), `qsl-replay` messages and limit mapping.
- The removal of the local fairness types and their re-exports in favour of
  `quire_semantic_value::fairness`.
- `Cargo.lock`.
- Context only: `qsl-forms/src/temporal_clause.rs` `formula_form`/`Formula::add`
  (the arena puts each operand after its parent and each node once), the
  `not_emitted` path at `check/mod.rs:1555-1576`, ADR-014 A-4, ADR-018 TP-1 to
  TP-4 and IV-4, and the quire-exact commits `21349174..b83cab9e`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | A unit with a `terminal` declaration now passes `PackageDeclarations::check` with nothing that emits it or refuses it. No S4 code reads `CheckedGraph::terminals()`, and no `not_emitted` refusal applies here as it does for temporal clauses. So a unit with `terminal Halt ... any;` or `terminal Done ... when {..}` compiles to the same package identity as one with no declaration. This breaks FR-124 ("include the TerminalDeclaration in the checked package, so two models that differ only in it have different package identities"). The PR also deletes FR-382's "the declaration never compiles as silently ignored" guard. | qsl-semantics/src/check/mod.rs:1391-1445; spec/functional/FR-382-build-the-terminal-declaration-form-at-s2.md:70-72 |
| FND-002 | medium | The open choice: a `terminal when` predicate's `self` binds to the one object type the model's populations range over. A model with no population refuses `missing-name`, and one with several object types refuses `ambiguous-name`. This does not contradict the spec. FR-124 says only "a state predicate over the model's state", and AC-1's "holds exactly when every counter is at 3" matches an invariant read over every object. But FR-124 states neither the binding nor the two refusals, and no test reaches either refusal branch. | qsl-semantics/src/check/assemble.rs:3253-3266 |
| FND-003 | low | `Cargo.lock` also moves `quire-exact` from `21349174` to `b83cab9e` (IR-873, adding `ChargePoint::TemporalVisit`), although the PR body says only the quire-semantic-value entry moved. It is harmless: quire-exact is branch=main and nothing in QSL matches exhaustively on `ChargePoint`. | Cargo.lock:1623 |
| FND-004 | low | A terminal's `using` alias is resolved to any profile selection, so `using t0` with a temporal profile checks the predicate under non-complete value semantics. The spec does not say which profile kind `using` takes, and this is not refused. | qsl-semantics/src/check/assemble.rs:3211-3222 |

## Verdict

FAIL on FND-001 (high). Without it, the change is sound:

- The safety-fragment classifier matches ADR-018 IV-4. One top-down pass over
  the arena tracks polarity without recursion. `Not` flips its operand, and
  `Implies` flips its antecedent only. Positive `always`/`release` and
  negated `eventually`/`until` are admitted with any interval. Positive
  `eventually`/`until` and negated `always`/`release` are admitted only with a
  closed `[a,b]`. So `always[a,*]` is safety and `eventually[a,*]` is not. Past
  operators are admitted under both polarities. The arena invariant the pass
  relies on (each node once, operands after their parent) holds in
  `qsl-forms` `Formula::add`.
- The property forms are classified in FR-123's order. The fixed-sample and
  timestamped profiles get `None`.
- Fairness resolves through `OperationLookup` and keeps the declaring
  `EffectiveId`, so same-named operations on different types stay distinct.
  Ambiguous and missing names refuse with the spec's codes, and a bounded
  profile refuses `unsupported_construct`/`expression-form`.
- The SV types are used directly, and no local copy or re-export of
  `FairnessKind`, `FairnessGranularity`, `FairnessConstraint`,
  `FairnessOperation` or `PropertyForm` is left. The `qsl-forms` syntax enums
  are the S2 forms, not copies.
- No panic surface, `unsafe` or unchecked integer conversion was added.

## Dispositions

Round 2, reviewed at `059cb1466e5b46cb36bb0528acaf222d40fa1594`. The fix
commit is `3fd335c7f`; the merge of `origin/main` was not reviewed.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 3fd335c7f: `check_with_cancel`, which `check` and `qsl-replay` lifecycle both call, now refuses each checked `terminal` with `CheckCause::TerminalNotEmitted` (`unsupported_construct`/`not-yet-implemented`, at the declaration span) after the checks pass. A package with no terminal returns its graph unchanged. The only bypass is the cfg(test-support) `check_unemitted`. FR-382 and TC-1059 step 6 state the refusal. |
| FND-002 | fixed | 3fd335c7f: `a_when_predicate_needs_exactly_one_state_object_type` covers the no-population (`missing-name`) and several-types (`ambiguous-name`) refusals, with code, tag and span. FR-124 still does not state the `self` binding; the PR body records it as the open choice. |
| FND-003 | fixed | 3fd335c7f: against `origin/main`, `Cargo.lock` now moves only `quire-semantic-value` (74bd0afc -> 02e5cf5c). |
| FND-004 | deferred | Low, recorded only (ONE-reviewer, major-bugs-only rule): the spec does not say which profile kind a terminal's `using` takes. |

---
id: SR-736
title: "QSL-277 code and Rust review of PR 491 (S3 state checker, FR-104)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@e278a3bdbde466895dc438b93138b2897028ee5d; qsl-semantics/src/check/state_clause.rs; qsl-semantics/src/check/observation.rs; qsl-semantics/src/check/lowering/state.rs; qsl-semantics/src/check/lowering/model.rs; qsl-semantics/src/check/lowering.rs; qsl-semantics/src/check/assemble.rs; qsl-semantics/src/check/assemble/tests.rs; qsl-semantics/src/check/check.rs; qsl-semantics/src/check/check/typing.rs; qsl-semantics/src/check/claims.rs; qsl-semantics/src/check/facts.rs; qsl-semantics/src/check/ir.rs; qsl-semantics/src/check/mod.rs; qsl-semantics/src/check/refusal.rs; qsl-semantics/src/check/region.rs; qsl-semantics/src/value/declaration.rs; qsl-semantics/tests/it/main.rs; qsl-semantics/tests/it/model_operations.rs; qsl-semantics/tests/it/state_clauses.rs; qsl-eval/src/value/expression/evaluate.rs; qsl-forms/src/syntax.rs; qsl-replay/src/spine.rs; src/command/output.rs; src/command/output/types.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: reviews
---
## Summary

Ticket: QSL-277. PR: quire-spec-language#491 at e278a3bd (base f05b526c).
This is a code review with the rust-review lane. It covers only the PR diff:
24 files, +2742/-161.

Sound:

- #486's placeholder refusal is gone. No `UnsupportedStateClause` is left in
  any `.rs` or spec file. The assembler keeps each `StateClauseForm` and
  resolves its alias, its context `M::T` and its operation. S3 checks the
  body through `ProtocolClauseFamily::check`.
- `self`, `result` and `reaches` are typed in the typer, and the old blanket
  refusals are gone (typing.rs:759-782). A field read through a reference in
  a state clause becomes the `Attribute` node (typing.rs:1364-1374).
- `state_pre_eligible` implements FR-104's `pre(e)` rule. It admits `self`,
  field or `deref` reads through a parameter reference, `allInstances`,
  `lookup` and nested `pre`. It refuses bare parameters, `result`, and `let`
  captures bound outside the `pre` (check.rs:1759-1812).
- `Observations` computes one observation per model read. A reference keeps
  its observation through `value`, `let` and binders. `pre(e)` retags its
  operand (observation.rs:75-155). Definedness keys each attribute step by
  (field, observation), so a fact at `pre` never discharges an obligation at
  `post` (facts.rs:444-458, 607).
- Clause and function names share one duplicate check that refuses at every
  declaration (mod.rs:577-632).
- SR-723 FND-007 (operations visible on subtypes) is closed in the code.
  `TypeEnvironment::operation` resolves through ancestry and keeps the
  declaring type (declaration.rs:896-928). A scratch probe at e278a3bd
  checked `post A on Config::Sub::attemptUpdate` with `Sub` specializing
  `ConfigVersion`. It passed, and `declaring != context`. No test in the PR
  covers this path (FND-006).
- No production `unwrap`, `expect` or `panic` was added. Every
  `u32::try_from` is mapped to a fault. The new `NodeKind::Reaches` has an
  explicit arm in each exhaustive match, including qsl-eval's.
- `cargo clippy -p qsl-semantics --all-targets -D warnings` is clean. The 21
  `state_clauses` and `model_operations` tests pass at e278a3bd.

Scratch probes were run in a detached worktree at e278a3bd, since removed,
with extra tests only:

1. Two operations with empty frames, `isStable(): Boolean` and
   `versionTotal(): Integer`, each named by one `post` clause. The check
   refuses `runtime_invariant`/`established-invariant-broken`.
2. The same setup with two `Boolean` operations checks, but yields 3
   requirement records instead of 4.
3. A clause on the subtype `Sub` has a `Bounded` clause record, with no
   population domain.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | A frame record is keyed by the `frame` node, and that node is content-keyed on {modifies, creates, deletes} and the declaring type only (FR-105). Two different operations with equal frames therefore share one key. Two read-only operations on one type are enough. If their records differ (probe 1: result `Integer` vs `Boolean`), the `Occupied(_)` arm makes a valid unit refuse with an internal fault (`runtime_invariant`). If their records are equal (probe 2), the two frames merge silently into one record, although FR-104 asks for one record "for each frame of an operation that a clause names". Fix: FR-104 must say which key to use. Either key the frame record by the operation's `operation_anchor` occurrence, which is per operation, or state that there is one record per distinct frame node and define whose roots its extent covers. Then change the code to match and add a test with two operations that have equal frames. | qsl-semantics/src/check/mod.rs:1216-1237; qsl-semantics/src/check/lowering/state.rs:329-367; qsl-semantics/src/check/state_clause.rs:555-583 |
| FND-002 | medium | Population membership uses exact type identity: `population.member_types.contains(member)`. A clause whose context is a subtype of a population's member type finds no population, and its record is silently `Bounded` (probe 3). A clause over `Sub` still ranges over the `Sub` objects of `config_history`, so its extent under-reports an unbounded domain. The case with no population at all is also silently `Bounded`. FR-104 ("the one population ... whose member types include `T`") does not say whether conformance counts. Fix: settle this in FR-104, probably by conformance, to match FR-084's `allInstances<T>`. Then test it. | qsl-semantics/src/check/lowering/model.rs:117-140; qsl-semantics/src/check/state_clause.rs:183-204 |
| FND-003 | medium | Population resolution is written twice, a likely result of the parallel sub-forks. The assembler refuses an ambiguous population for the context and the declaring type at the `on` span (`AssemblyCause::AmbiguousPopulation`). `check_clause` then recomputes both and has its own `ambiguous_population` refusal. That refusal cannot be reached for the context or the frame, because the assembler already refused. It is live only for a `reaches` over a parameter of another type, and there it refuses at the body root rather than the `on`. It also packs the population names into `AmbiguousName.name` with `format!("{name} ({..})")` instead of a typed field. Fix: resolve populations in one place, carry the resolved domains on `StateClauseDeclaration`, and delete the dead branches. | qsl-semantics/src/check/assemble.rs:1647-1666; qsl-semantics/src/check/state_clause.rs:206-226, 296-324 |
| FND-004 | low | `clause_records` builds its classify limit by hand as `StageLimitCause { stage: Typing, kind: Nodes, region: None }`. It does not reuse the `limit_cause` helper that this same PR extracted in mod.rs, so it hard-codes the kind and drops the region. `record()` also repeats `classify_extent`'s mapping from root index to `DomainKey` instead of calling it (it differs only in the frame prefix). | qsl-semantics/src/check/state_clause.rs:423-457, 509-519; qsl-semantics/src/check/mod.rs:444-458; qsl-semantics/src/family/requirements.rs:218-236 |
| FND-005 | low | The `WrongSnapshotCause` doc is now false. It says every checking-time `wrong_snapshot` is `ForbiddenPreRead`, and that `WrongAnchor` "is reserved for the one case the checker cannot see at all". The PR adds a checking-time `wrong-anchor` through the new `CheckCause::UnanchoredResult`. Keep the separate variant, because it carries the clause kind and operation, and `WrongSnapshotCause` is the FR-090/ADR-013 evaluation-cause contract. Fix the doc so it names `UnanchoredResult` as the checking-time path to the same catalog cause. | qsl-semantics/src/check/refusal.rs:197-221, 292-305 |
| FND-006 | low | `TypeEnvironment::operation` adds a rule where a more-derived declaration hides an ancestor's, plus an `Ambiguous` outcome that refuses `AssemblyCause::AmbiguousOperation`. No spec states either. FR-103 says only "visible on a subtype through FR-081's effective view, as a field is", and fields hide only through explicit redefinition. No test covers the inherited path (SR-723 FND-007), the hiding rule or the ambiguous outcome. Fix: add one line to FR-104 or FR-103 for the rule, and tests for the inherited and ambiguous cases. | qsl-semantics/src/value/declaration.rs:890-928; qsl-semantics/src/check/assemble.rs:1634-1644 |
| FND-007 | low | The `UnanchoredResult { clause, operation }` payload, which FR-104 and TC-459 step 3 require ("naming the clause kind and the operation"), is not shown in any message: the spine's `check_message` has no arm for it. No test reads it either. | qsl-replay/src/spine.rs:392-407; qsl-semantics/src/check/check.rs:1668-1682 |

## Verdict

Request changes. FND-001 gives a wrong result on valid input: an internal
fault, or a merged record. It needs a one-line spec ruling on the frame key
before the code fix. FND-002 and FND-003 should be fixed in this PR. The low
findings are small.

Only targeted tests were run, as the brief asked. The full gates were not
run.

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

## Dispositions

Round 2, checked against ab6a987c (fix commit, rebased on origin/main
2df75ab6) on 2026-09-26. Probes were run in a detached scratch worktree at
ab6a987c, since removed. `cargo clippy -p qsl-semantics -p qsl-replay
--all-targets -D warnings` is clean, and the 24 `state_clauses` and
`model_operations` tests pass. The rebase kept #490's `evaluate.rs` and
`output.rs` intact: `git diff origin/main HEAD` on those files shows only
this PR's additions (the `Reaches` arm and the `StateClause` origin).

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed ab6a987c | Each frame record is keyed by (frame node, an occurrence minted once per operation identity (declaring, name)), in `Lowering::frame_occurrence` (lowering.rs:1305-1324, lowering/state.rs:218-219). Both probes are now tests, and each gives 4 records (state_clauses.rs:723-770). The `Occupied(_)` fault is now unreachable. Clause keys are (state_clause node, claim ordinal), unique per clause. Frame keys are unique per operation. Clauses that name one operation build equal records, because roots, `frame_population` and the Boolean node all come from the operation. A probe with `post` clauses on `ConfigVersion::attemptUpdate` and on the inherited `Sub::attemptUpdate` gives 3 records and no fault. The ordinal is order-dependent, which is recorded as a new finding, FND-008. |
| FND-002 | fixed ab6a987c | `populations_of` covers by `conforms` (lowering/model.rs:117-150). FR-104 states conformance and the no-population case. Tested at state_clauses.rs:778-810. See the new FND-010 on domain keying. |
| FND-003 | fixed ab6a987c | Only the assembler resolves the context and frame populations (assemble.rs:1646-1690), carried on `StateClauseDeclaration.{context,frame}_population`. The S3 copies are deleted. The `reaches`-target resolution stays in S3, as ruled. It still refuses at the `reaches` locus and packs the names with `format!`; that part is accepted under the ruling. |
| FND-004 | fixed ab6a987c | `clause_records` uses `limit_cause` (state_clause.rs:534-546). The clause path reuses `classify_extent`. Only the frame path, which has a prefix, walks `classify_domains`. |
| FND-005 | fixed ab6a987c | The `WrongSnapshotCause` doc now names `UnanchoredResult` as the checking-time path to `wrong-anchor` (refusal.rs:213-223). |
| FND-006 | fixed ab6a987c | FR-104 Resolution states the hiding and ambiguity rule. Inheritance and ambiguity are tested (state_clauses.rs:815-840). |
| FND-007 | fixed ab6a987c | The spine renders the clause kind and operation (spine.rs:405-421). The payload is asserted at the cause level (state_clauses.rs:286-296). The spine arm itself has no test; this is minor and not reopened. |

### New findings (round 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | medium | The frame-occurrence ordinal is minted in source order ("the first clause naming that operation ... mints it"), so the key for an operation's frame record depends on clause order. A probe used `isStable(): Boolean` and `versionTotal(): Integer` with empty frames, then reversed the two clauses. The key sets are equal, but key-to-record is not: the two operations swap ordinals 0 and 1. The same key now names a different operation's record. This breaks FR-104-AC-6's intent (record keys stable under reordering), and the existing AC-6 test compares key sets only, so it cannot see this. Fix: mint each frame's ordinals in a deterministic operation order, for example ascending (declaring `EffectiveId`, name) over the unit's named operations that share the node. Alternatively, key the frame record by the per-operation `operation_anchor` node. Then amend FR-104's "first clause ... in source order" sentence and add a reorder test that compares key-to-record. | qsl-semantics/src/check/lowering.rs:1295-1324; spec/functional/FR-104-check-state-clauses.md:176-184; qsl-semantics/tests/it/state_clauses.rs:626-657 |
| FND-009 | low | The frame occurrence is recorded with `OccurrenceRole::Anchor`. FR-105's Outputs row gives a `frame` node the occurrence `generated`, 0, and `anchor` is the role of the `operation_anchor` node. The frame node now carries `anchor` occurrences that FR-105 does not list, which will show up when S4 emits occurrences (QSL-279). Record this in FR-105 and FR-104, or use a role FR-105 names. | qsl-semantics/src/check/lowering.rs:1319-1321; spec/functional/FR-105-emit-state-clause-operation-anchor-and-frame-nodes.md:67 |
| FND-010 | low | With conformance, one population gets a different `DomainKey` depending on the clause's context. Probe: an invariant on `Sub` keys `config_history` by the `Sub` node, and an invariant on `ConfigVersion` keys it by the `ConfigVersion` node, both with path `[0]`. FR-104 keys by "the population's member type `T` (the clause's context type ...)", which no longer fixes one node. A `Cardinality` bound on `config_history` must then match two keys. Fix: key by the population's declared member type, so one population is one domain key, or state the per-context keying in FR-104. | qsl-semantics/src/check/state_clause.rs:193-216; spec/functional/FR-104-check-state-clauses.md:192-203 |

### Round 3 dispositions

Checked against e2e5ffdc (fix round 2 6b144f75, plus the clippy fix
e2e5ffdc) on 2026-09-26. The 26 `state_clauses` and `model_operations` tests
pass. `cargo clippy -p qsl-semantics -p qsl-replay --all-targets -D
warnings` is clean. The `qsl-277-ci-r6.log` file's first line is the head
SHA and its last line is `exit=0`. The diff of `evaluate.rs` and `output.rs`
against origin/main is still this PR's additions only.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-008 | fixed 6b144f75 | `register_frame_occurrences` (lowering/state.rs:263-286) dedups the named operations by (EffectiveId, name). It sorts them by (declaring `DeclarationKey`, name) and mints every frame occurrence before any clause is lowered (mod.rs:1117-1134). A later per-clause `frame_occurrence` call only reads the cache. `frame_node` adds no occurrences of its own (`object_node`, `frame_field` and `text_literal` record none). If registration fails, the refusal is pushed, so a source-order mint afterwards cannot reach a checked graph. The test compares the whole key-to-record map across clause orders (state_clauses.rs:871-917), and would fail on the round-2 source-order mint. FR-104:176-189 is amended. |
| FND-009 | fixed 6b144f75 | The frame occurrence is `OccurrenceRole::Generated` (lowering.rs:1325), recorded at `generated_location()`. `finish` then adds no second `generated` occurrence, because `has(frame)` is true. FR-105:67 is amended. The test asserts role `generated` with ordinals {0, 1}. |
| FND-010 | fixed 6b144f75 | `populations_of` returns the covering member's `EffectiveId`, and `population_of` keys the domain by it (lowering/model.rs:131-158, state_clause.rs:195-219). A test shows that `Sub` and `ConfigVersion` clauses key `config_history` identically (state_clauses.rs:965-996). FR-104:197-206 is amended. See the new FND-011 for populations with several member types. |

### New findings (round 3)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-011 | low | A population with two or more declared member types still gets one `DomainKey` per covering member, not one per population. `covering_member` is the first `member_types` entry, in declared order, that covers the context. So with members {A, B}, clauses on A and on B key the same population by different nodes. With members [Sub, ConfigVersion], a `Sub` clause keys it by `Sub` and a `ConfigVersion` clause by `ConfigVersion`. FR-104:197-206 ("the population's own declared member type that covers `T`") allows this, which contradicts the ruling "a population has one `DomainKey`". Fix: pick one canonical node per population, for example the least member type in `DeclarationKey` order, whatever the clause context, and add that to FR-104. It is not reachable with today's single-member fixtures. | qsl-semantics/src/check/lowering/model.rs:131-158; spec/functional/FR-104-check-state-clauses.md:197-206 |

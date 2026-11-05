---
id: SR-2348001
title: 'Code review of PR #2348: qsl-analyze node-to-family reader claim_family (QSL-1702), code-review'
type: SpecReview
analysis: code-review
scope: 'agent-ix/quire-spec-language@b90258f05; git diff origin/main...b90258f05 (PR #2348, branch peter/qsl-1702-node-family-reader): qsl-analyze/src/claim_family.rs; qsl-analyze/src/analyze.rs; qsl-analyze/src/analyze/items.rs; qsl-analyze/src/contract_check/mod.rs; qsl-analyze/src/value_check/mod.rs; qsl-analyze/src/lib.rs; qsl-analyze/tests/it/{claim_family,hyper_settle,operation_contract,value_validity,refinement_protocol,main}.rs; spec/functional/FR-507 Dependencies bullet; ticket QSL-1702'
review_set: subset
date: '2026-11-05'
---

## Summary

Ticket: QSL-1702. PR: quire-spec-language#2348, head b90258f05, merge base 28604a044, trial-merged with origin/main 823b9a9aa. Method: code-review with the rust-review lane, one pass scoped to the PR diff. QSpec was read by ID from a detached worktree at quire-specification origin/main e85daed5 (FR-290, FR-477, FR-484). The dispatching lead relayed the spec lane's ruling (QSL-1714, QSL-1715) that the code is wrong on both disclosed discrepancies; QSpec FR-477 and FR-484 were read and agree with that ruling.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| claim_family item_kind arm and node_family table | qsl-analyze/src/claim_family.rs:34-86 | examined |
| node_form: graph node, refinements(), task_sets() lookup | qsl-analyze/src/claim_family.rs:92-114 | examined |
| Engine::probe, AnalyzeItem::probe (test-support), AnalyzeItem::at_claim | qsl-analyze/src/analyze.rs:85-113, 598-630 | examined |
| hyper_form_item package arg, CLAUSE_FAMILY, protocol_item default, derived_* families | qsl-analyze/src/analyze/items.rs:48-79, 117-215, 547-720 | examined |
| contract_item / value_item (package, claim) args | qsl-analyze/src/contract_check/mod.rs:280-296; qsl-analyze/src/value_check/mod.rs:452-467 | examined |
| New tests | qsl-analyze/tests/it/claim_family.rs | examined |
| Modified tests (routed_hp4, two "selects no form" tests) | hyper_settle.rs:660-725; operation_contract.rs:324-353; value_validity.rs:1121-1160 | examined |
| FR-507 Dependencies bullet | spec/functional/FR-507-apply-each-capability-kind-to-its-declaration-family-only.md:210-212 | examined |
| QSpec FR-290 "Reading the family from a request item" | quire-specification FR-290 | context_only |
| QSpec FR-477 occurrences, FR-484 claim forms | quire-specification FR-477, FR-484 | context_only |
| string-edge detector pattern handling | qsl-string-edge/src/detector.rs:223-242 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The Boolean value clause row is wrong. QSpec FR-290/FR-484 put it on a Boolean `expression` node at role `claim`/`declaration`. The reader maps `(NodeTag::Function, _, claim, declaration)` with a Boolean callable result to `value` and has no row for a Boolean `expression` node, which reads `None`. The AC-11 test pins the function-node shape at roles claim and declaration and tags FR-507-AC-11 for it. Fix: map the Boolean `expression` row and drop the function row, together with the producer fix (QSL-1714/1715), so value items keep a family. Otherwise take this row out of the AC-11 test and its tag, and leave a `Remaining work:` note on the AC-11 row in FR-507. | qsl-analyze/src/claim_family.rs:56-61; qsl-analyze/tests/it/claim_family.rs:478-496 |
| FND-002 | high | The relation test asserts the wrong occurrence shape. QSpec FR-477 and FR-507-AC-7 say a `step_relation` has one `claim` occurrence, shared by the model and code claims, which are told apart by kind. `relation_claims` finds two occurrences and asserts `assert_ne!(model, code, "two claims of one node")`, under an AC-7 tag. A correct producer would fail that test. Fix: once the producer writes one occurrence, assert `model == code` and settle both items (and `schedulability`) on that one occurrence. Until then, drop the shared-occurrence half from the AC-7 tag. | qsl-analyze/tests/it/claim_family.rs:187-210, 239 |
| FND-003 | medium | The reader matches wire strings outside any `#[string_edge]` site. `node_family` matches `semantic_form` words (`"state_clause"`, `"operation_anchor"`, `"temporal_clause"`, `"state_graph"`, `"hyperproperty"`, `"step_relation"`, `"refinement"`, `"probabilistic_claim"`, `"schedulability"`) and role strings (CLAIM, DECLARATION, EXPRESSION, ANCHOR) as tuple sub-patterns. `xtask string-edge` reports 0 findings only because `pattern_has_string_operand` does not descend into `Pat::Tuple`. The gate's intent (#2345, ADR-012 section 9) is that a string selects behaviour only at a marked edge. Fix: decode the form and the role once, in a `#[ix_trace_rs::string_edge]` fn, into closed enums (like qsl-package's `WireForm::from_wire`, which is pub(crate) over a different node type and cannot be reused directly), then match the enums. | qsl-analyze/src/claim_family.rs:18-25, 51-85 |
| FND-004 | medium | The `protocol_item` clause default changed from `protocol` to `temporal-trace` with no test. On main this was a latent bug: `temporal-satisfaction` never applies to `protocol`, so every protocol clause item would have been refused `inapplicable-kind`. Measured: reverting only that line to `Some(DeclarationFamily::Protocol)` still passes the whole `qsl-analyze --test it` suite (1279 passed). Fix: add a test that runs a `protocol_item` clause item through `analyze_items` and asserts that EN-1 settles it, not `inapplicable-kind`. | qsl-analyze/src/analyze/items.rs:195 |
| FND-005 | medium | Two rows of the reader have no test that fails when they change. (a) The `item_kind` `refinement` row: no test passes `ItemKind::Refinement`. (b) The occurrence role on the `state`/`operation_anchor` row: no test reads that node at a role other than `anchor`. Measured on the trial merge: `ItemKind::Refinement => Some(TemporalTrace)` and `(State, "operation_anchor", _)` together (with the FND-004 revert) pass the whole `qsl-analyze --test it` suite, 1279 passed. Fix: assert `claim_family(package, ItemKind::Refinement, claim) == Some(RelationClaim)` on a node of another family, and assert `None` for the `operation_anchor` node at role `claim` (and the `state_clause` node at role `anchor`). | qsl-analyze/src/claim_family.rs:43, 70 |
| FND-006 | medium | AC-7's "records ... in caller order" is not exercised. Each item goes through its own `analyze_items` call with one item, so no test sees two or four records in one request. The refinement loop also reuses `position` as the request index for the `schedulability` item. Fix: in each AC-7 test, settle the model, code and `global-conformance` items, and the four refinement items, in one `analyze_items` call, then assert record order and indices. | qsl-analyze/tests/it/claim_family.rs:243-258, 369-385 |
| FND-007 | medium | The FR-507 Dependencies bullet now says the reader "is `qsl-analyze`'s `claim_family`". It drops the statement that AC-7, AC-8 and AC-11 are not yet satisfied. But `model_item`, `protocol_item`, `zone_item`, `statistical_item` and `timed_refinement_item` still hardcode their family, and FND-001/FND-002 are open, so the spec now over-claims. Fix: keep one line, `Remaining work: QSL-1702 (adapters that still hardcode a family), QSL-1714/QSL-1715 (Boolean clause node, shared relation occurrence)`. | spec/functional/FR-507-apply-each-capability-kind-to-its-declaration-family-only.md:210-212 |
| FND-008 | low | AC-8 says the two protocol-anchored derived items "run on a `temporal-trace` engine". AC-7's model claim is "settled by an engine". Both are verified only on `Engine::probe`, which advertises every kind. The family read and the routing are real, but no temporal-trace engine runs. This is disclosed in the PR body but not in the test docs. Fix: name the stand-in in the AC-7 and AC-8 test doc comments, or leave a `Remaining work:` note against FR-397. | qsl-analyze/tests/it/claim_family.rs:229-287, 388-468 |
| FND-009 | low | `node_form` calls `Refinement::v2_emission()` for every refinement on every lookup of a node outside the semantic graph. That rebuilds each refinement's whole v2 emission and its identity just to compare a key, so one `claim_family` call costs O(refinements x emission). Fix: compare against an identity the refinement already holds, or build a node-key index once per package. | qsl-analyze/src/claim_family.rs:98-105 |

## Verdict

CONDITIONAL. The PR is mergeable as a first slice once the test, tag and spec-text items below change. The reader's two carrier rows can wait for QSL-1714/QSL-1715.

Must change in this PR:
- FND-002: delete `assert_ne!(model, code, "two claims of one node")` (claim_family.rs:239). It pins the code's two-occurrence carrier, which QSpec FR-477 and QSL FR-507-AC-7 contradict, so the fix for QSL-1715 would fail it. Say in the `relation_claims` doc that the code claim is found on its own occurrence until QSL-1715 lands.
- FND-001: keep the function-node assertion, because it is what the producer writes today. But the AC-11 tag must not claim the Boolean-clause unit, which AC-11 and QSpec FR-290/FR-484 put on an `expression` node. Move that half into an untagged test, or note `Remaining work: QSL-1714` in the test doc. Add a comment on the reader's `Function` row saying it is the interim carrier and that QSpec's row is a Boolean `expression` node (QSL-1714).
- FND-007: restore a `Remaining work:` line in FR-507 Dependencies.
- FND-004, FND-005, FND-006: these are cheap tests. Add them in the same round.
- FND-003: decode the form and role at a marked string edge. This is new code. The gate passes only because the detector is blind to tuple patterns.

Can wait for QSL-1714/QSL-1715: changing the reader's `Function` row to the Boolean `expression` row. Removing the row now would read every `value_item` clause item as having no family and refuse it `inapplicable-kind`, so it has to land with the producer change. Asserting `model == code` in the relation test also waits. The `step_relation` row is already role-agnostic, so the reader needs no change for QSL-1715. FND-008 and FND-009 (low) can wait.

What is sound (measured):
- The rest of the node-to-family table matches FR-290 row for row: the `state_clause`/claim and `operation_anchor`/anchor roles, `temporal_clause`, `state_graph`, `hyperproperty`, `step_relation`, `refinement`, `probabilistic_claim`, `schedulability`, and any `protocol` node. The scalar-operation row is an `expression` node at role `expression` whose `value-validity` requirement record has an owner (FR-062: the owner is the `Value` function whose body holds it). Any `item_kind` other than `formula` decides the family. Any other node, and an absent node, reads `None` (FR-507-AC-9).
- Refinement and task-set nodes are found by identity, through `refinements()` (`v2_emission().identity == key`) and `task_sets()` (`node_id() == Some(key)`). There is no join on name, span or order.
- The reader has no panics, unwrap or indexing. `Engine::probe`'s `expect` is behind test-support and documents its panic.
- Every caller of `contract_item`, `value_item`, `hyper_form_item`, `protocol_item` and `at_claim` was grepped on the trial merge. All are tests in qsl-analyze/tests/it, and none is in a shipped crate. Each test caller passes the real claim of the request it runs.
- The three modified old tests are not weakened. Each keeps the absent-node `select_*` = `NoForm` assertion and now runs a real-family claim that still selects no form, asserting `declined` `invalid-input`. `routed_hp4` names the world's real `hyperproperty` node, and its unsupported-requested-capability assertions are unchanged.
- `Engine::probe` and `AnalyzeItem::probe` are `cfg(feature = "test-support")`. Only qsl-analyze's own dev-dependency enables the feature (resolver 2). qsl-inspect and xtask depend on qsl-analyze without it.
- No compatibility layer. No Cargo.lock or Cargo.toml change.
- The `protocol_item` clause default change from `protocol` to `temporal-trace` is correct by FR-290: the clause is a `temporal_clause` node. On main the old default was a latent bug, because `temporal-satisfaction` never applies to `protocol`. It is untested (FND-004).

Mutations (real, on the trial merge, then reverted):
- The `Function` row ignoring its role was killed at claim_family.rs:508.
- Dropping `hyperproperty` from its row was killed by `routed_hp4` and the AC-11 test.
- Swapping `step_relation` to `temporal-trace` was killed by two tests (prior partial run).
- `ItemKind::Refinement` to `TemporalTrace`, `operation_anchor` at any role, and the `protocol_item` default back to `Protocol` all survived the full `it` suite, 1279 passed (FND-004, FND-005).

Tags: the AC-7 model-claim half and the AC-8 last sentence run only on `Engine::probe` (FND-008). The AC-11 hybrid row is exercised through a temporal claim over a hybrid subject, which is what AC-11 asks. The PR body calls the protocol-abstract refinement uncovered, but it is covered: the AC-11 test's last case uses `cas_twice`, whose abstract side `Spec::Twice` is a protocol.

Gates on the trial merge with origin/main 823b9a9aa (clean auto-merge):
- `cargo run --locked -p xtask -- string-edge`: 0 findings. That result is vacuous for this file (FND-003). `qsl-string-edge/src/detector.rs:223-242` `pattern_has_string_operand` does not descend into `Pat::Tuple`, `Pat::TupleStruct`, `Pat::Struct` or `Pat::Slice`. That detector gap is outside this diff and needs its own ticket.
- qsl-semantics lib scan tests (`--features test-support`): `check_stage_sources_have_no_global_or_thread_local_state`, `the_assembler_reads_no_cst` and `no_qsl_cst_type_is_re_exported_to_the_assembler` passed (3).
- Focused `qsl-analyze --test it` (claim_family, hyper_settle, operation_contract, value_validity, refinement_protocol) with QSPEC_DIR: 91 passed.
- `cargo clippy -p qsl-analyze --all-targets -- -D warnings`, default and `--all-features`: clean.
- `cargo check --locked --workspace --all-targets`: clean.

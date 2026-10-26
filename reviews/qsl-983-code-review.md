---
id: SR-1595001
title: "code review and rust review of QSL-983 terminal declaration emission (QSL PR 1595)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@5ae5da93c5309a0dd686c40ae564be264b37d2c4; git diff origin/qsl-ir-562-pin...origin/qsl-983-terminal-emission; qsl-semantics/src/check/{mod,check,refusal,region,terminal}.rs, qsl-semantics/src/check/lowering/{terminal,model}.rs, qsl-package/src/checked_v2/terminal.rs, qsl-package/src/emit/tests/terminal.rs, qsl-analyze/tests/it/model_settle.rs, qsl-semantics/tests/it/{terminal_declarations,refinement_declarations}.rs"
review_set: subset
date: '2026-10-26'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-411
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-124
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-128
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: references
---
# SR-1595001: code review and rust review of QSL-983 (QSL PR 1595)

## Summary

Ticket: QSL-983. The scope is the stacked diff `origin/qsl-ir-562-pin...origin/qsl-983-terminal-emission`
at head 5ae5da93c: 32 files. The rust-review lane is folded into this file. The oracles are
FR-411 AC-1 to AC-4, the terminal half of FR-128-AC-6, FR-124, FR-093 and QSpec FR-476 (at
QSpec origin/main 23dc1ce3).

S4 lowers each checked `terminal` declaration to a `temporal`/`terminal` node. The body is
`model`, `mode`, `parameters`, `condition`, with no `declaration` and one `claim` occurrence at
the declaration span. QSL's I2 reader reads the node back as node ids. `TerminalNotEmitted`, its
match arms, the `with_unwritten_terminals` hook and the refusal-only test are deleted.

Focused tests were run on the head in the coder's target dir, and all passed:

- qsl-package `emit::tests::terminal` and `admission_corpus` (8 tests).
- qsl-semantics `terminal_declarations`, `refinement_declarations` and `symmetric_population`
  with `test-support` (47 tests), plus the `assemble::tests::a_terminal*` tests (2).
- qsl-analyze `model_settle`, `counter`, `deadlock_item` and `model_check` (105 tests).
- qsl-eval `smt_query` and `sampled`, and qsl-replay `vault_world` and `spine` (all features).

The stack also merges onto current origin/main 350524f18 with no textual conflict.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Replay and the deadlock classifier do not read the terminal node. They still read the S3 value `package.graph().terminals()`, and `read_terminal`/`read_terminals` have no production caller; only the qsl-package unit test calls them. FR-411 Behavior says "Replay of a deadlock-freedom counterexample (FR-128) SHALL read the model's TerminalDeclaration from the recompiled package's temporal/terminal node". QSpec FR-476 says the node "is the only carrier of the declaration: a deadlock-freedom item, its classifier and its replay read the model's intended terminal states from this node and from nothing else". This gives no wrong result today, because the recompile builds the node and the S3 value from one checked declaration and the `package_id` binds the node (AC-3 holds). But the stated read path is not built, and no test can detect drift between the two. The reader returning node ids (`ReadTerminalDeclaration`) rather than a checked `TerminalDeclaration` is sound, and matches the `resolve_clause_root` and `resolve_state_graph_root` precedent. Two fixes are possible. One is to resolve the model's terminal through the I2 read on the replay recompile path, as `model_check/identity.rs` does for clause roots, and check that it agrees with the S3 value. The other is to amend FR-411's Behavior and Outputs to say that the reader returns the node's mode and node ids, and that replay reads the S3 value of the recompiled package whose identity carries the node. | qsl-eval/src/simulation/model/terminal.rs:79, qsl-eval/src/smt/query.rs:746, qsl-package/src/checked_v2/terminal.rs:118-147, spec/functional/FR-411-emit-a-checked-model-terminal-declaration-as-a-v2-terminal-node.md:81-87 |
| FND-002 | medium | The admission corpus still classifies `temporal`/`terminal` as `NOT_EMITTED_BY_QSL`, with the reason "no S4 emission writes a `terminal` declaration (terminal::not_emitted)". This PR deletes that function and makes S4 emit the family. The FR-093-AC-19 test still passes, but only because no corpus fixture holds a terminal declaration. The classification is now false, and the corpus never runs its row checks (admission at the emitted `package_id`, the stratified-grammar walk) over a terminal node. Add a terminal row with a fixture that emits one, for example the ConfigVersion unit with `terminal Done using v on Config when { self.versionNumber = 3 };`, and delete the `NOT_EMITTED_BY_QSL` entry. | qsl-package/src/emit/tests/admission_corpus.rs:581-584 |
| FND-003 | medium | S4 raises an internal fault on a declaration that S3 accepted. `check_terminal` returns `Any` without inspecting the model. If the selected model declares no object type, `AdmittedModel::terminal_object` returns `None`, and `Lowering::terminal_object` turns that into `CheckCause::InternalFault(UnkeyableRequirements)`. The package then fails with an internal failure for a source declaration that checked. The invariant the fault names, `requirements-item-keyed`, belongs to requirements items and does not describe this case. This was found by reading the code; no probe was run. Either refuse the case at S3 with a specified cause, or define the node FR-476 should reference here (see FND-004) and add a test. The other lowering faults in `terminal.rs` should name their own invariants through `stage_fault`, as `hyper.rs` does. | qsl-semantics/src/check/lowering/terminal.rs:62-73, qsl-semantics/src/check/lowering/terminal.rs:96-111, qsl-semantics/src/check/lowering/model.rs:651-661, qsl-semantics/src/check/terminal.rs:143-150 |
| FND-004 | medium | The `model` node choice is a spec gap and needs a QSpec ticket against FR-476. FR-476 says `model` is "a reference to the model node of the model alias the declaration's on names, the node an FR-402 subjects binding names for that alias". That presumes one node per alias. In QSL a model alias has one object-type `model` node per object type, and FR-402's lowering picks the object type of the alias's first object parameter (FR-488 Q-2). A terminal declaration has no such parameter. When the model's populations range over exactly one object type, the PR uses that type, which agrees with FR-366's binding of `self`. Otherwise it takes the least object type by `DeclarationKey`. The otherwise case covers `terminal any`, and a `when` that does not read `self`, over a model with zero or several state object types (FR-124-AC-6's Counter and Gate model). No requirement states that choice. A second producer could pick another type and get a different node id and `package_id` from the same source. FR-476 reader rule 5 ("one per model") and any consumer that looks up the terminal by model node depend on the choice. The QSpec ticket should state the node for these cases, including a model with no object type (FND-003). | qsl-semantics/src/check/lowering/model.rs:647-661, qsl-semantics/src/check/lowering/terminal.rs:91-111 |
| FND-005 | low | Stale comment: the `package` helper's doc still says a `terminal` declaration is checked "though no S4 emission writes it yet". | qsl-analyze/tests/it/counter.rs:253-255 |
| FND-006 | low | The span registration builds a `Location` from `terminal_location` and then destructures it with `if let Origin::TypeDeclaration { name } = ...`. That pattern cannot fail as written, but if `terminal_location` ever changed, the span would be skipped silently with no error. A `terminal_site(index, site) -> String` helper, as `temporal_site` is for temporal clauses, would give the name directly and remove both `if let`s. `terminal_predicate` also rebuilds `claim_at`, which its caller already holds. | qsl-semantics/src/check/mod.rs:2196-2210, qsl-semantics/src/check/lowering/terminal.rs:116-125 |

## Verdict

No high finding, and no wrong result on any path the ACs name. The emission is correct against
FR-476's body:

- The four bindings are in order, and `mode` is a text literal.
- `self` is bound at level 0, typed `Reference<T>`, only when slot 0 is read.
- `condition` is a reference for `when` and an empty aggregate for `any`.
- The node has `semantic_type` boolean, no `declaration`, and one `claim` occurrence at the
  declaration span.
- The written name enters no key.

Assembly refuses a second `terminal` on one model (`AmbiguousTerminal`), so the emission loop
lowers exactly one node per model.

On the coder's notes:

- Note 1, `ReadTerminalDeclaration`: the shape is sound, but nothing reads it. See FND-001.
- Note 2, the model node choice: this is a QSpec gap that needs a ticket. See FND-004.
- Note 3, the ConfigVersion fixture: the AC-1 and AC-2 shape is asserted. The fixture
  differences are recorded in SR-1595002.
- Note 4, the deletions: nothing still depends on `TerminalNotEmitted` or
  `with_unwritten_terminals`. A grep of the head finds both only in historical `reviews/` files.
  The three commits on origin/main since the stack's base (#1584, #1589, #1580) add no new use.
- Note 5, the refinement test rename: correct; see SR-1595002 FND-004.
- Note 6: making `site_spans` mutable is needed and safe, because the terminal sites use their
  own `terminal <i>` names.

Mergeable once FND-002 is fixed and FND-001 is settled, either by code or by an FR-411
amendment. FND-003 and FND-004 can go to the QSpec ticket, with the edge refused or tested in
this PR.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | high | This is a capability regression. `resolve_terminals` runs `emit_checked` over the whole package to find the terminal nodes, and `ModelSystem::new` and `encode_smt_query` call it whenever a unit declares a terminal. `emit_checked` refuses every unit with a probabilistic declaration (`EmitRefusal::ProbabilisticModelNotEmitted`, emit.rs:1481), and it refuses on any other emission defect. `ModelSystem::new` maps that refusal to the internal fault `terminal-nodes-resolve`. So a model with a `terminal` declaration plus a `random`, `workload` or `reward` declaration can no longer be built at all; on main it was admitted. FR-187-AC-2's `Link` model is exactly this case: the AC itself declares `terminal when self.delivered or self.attempts = 2`. The fix round deleted `terminal Stop ...` from `sampled/world.rs` so the tests pass. That makes the TC-622 and TC-623 tests (FR-187-AC-2, the stutter test) run a model the AC does not describe, which hides the regression. Fix: read the terminal nodes from the checked graph's own `SemanticGraph` (`package.graph().semantic_graph()`), which holds the lowered nodes emission would serialize, with no emission and no dependence on unrelated features. Then restore `terminal Stop` in `link()`. | qsl-package/src/checked_v2/terminal.rs:290-316, qsl-eval/src/simulation/model.rs:606-607, qsl-eval/src/smt/query.rs:409-410, qsl-eval/src/simulation/sampled/world.rs:444-447, qsl-package/src/emit.rs:1481-1488 |
| FND-008 | low | The new optional `serde_json` dependency in qsl-package's `std` feature exists only to parse back the bytes `emit_checked` has just written. It is also declared with an inline version instead of the workspace entry. With FND-007's fix (reading `SemanticGraph` nodes) it has no use, so delete it. | qsl-package/Cargo.toml:19, qsl-package/Cargo.toml:59-60 |
| FND-009 | low | When the number of terminal nodes and checked declarations differs, `resolve_terminals` returns `Disagrees { model: "" }`, which names no model. Name the node that has no checked declaration. | qsl-package/src/checked_v2/terminal.rs:349-351 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | The model checker and the SMT encoder now read presence and mode through `resolve_terminals`, but the read does not meet FR-411/FR-476. It emits the whole package, which is the cause of the FND-007 regression. It also parses the emitted JSON without IR admission (`read_checked_package_v2`). The `when` predicate is the S3 body, found by an occurrence computed from the same lowering key, so the node check can only catch a dropped node; no test makes the S3 value and the node disagree. `qsl-route` deadlock.rs `opts_out`, which derives the deadlock item, still reads S3 alone. The checked body cannot be rebuilt from the wire, so the predicate will always come from S3. To close this: read presence, mode and model from the `SemanticGraph` terminal nodes (FND-007's fix), and use them in `opts_out` too. Then amend FR-411 to say the `when` predicate is the checked body of the node's claim occurrence. |
| FND-002 | fixed | admission_corpus.rs now has a `Terminal` fixture and the row `row(K::Temporal(TemporalForm::Terminal), F::Terminal)`. The `NOT_EMITTED_BY_QSL` entry for `temporal`/`terminal` is gone. |
| FND-003 | fixed | mod.rs now refuses `terminal any` on a model with no object type at S3, before lowering: `CheckCause::TerminalWithoutObjectType`, `unsupported_construct`/`declaration-form`, at the declaration span. Assembly already requires exactly one population object type for every `when`, so `terminal_object` cannot return `None` for `when`. The cause and its placement are right for an interim refusal: it is located at the declaration, and replay classifies it as `unsupported`. No requirement names this refusal yet; record it under STD-492. |
| FND-004 | deferred | Filed upstream as QSpec STD-492. lowering/model.rs now cites STD-492 at the least-object-type choice. |
| FND-005 | fixed | counter.rs: the doc is now "The checked package of `body` over `document`." |
| FND-006 | fixed | `terminal_site(index, site) -> String` was added. mod.rs inserts both spans by name with no `if let` on a constructed `Location`. |
| FND-001 | fixed | Commit 76352428c reads presence and mode from the checked graph's own nodes: `CheckedGraph::terminal_nodes()` scans `semantic_graph()` for `temporal`/`terminal` nodes. `resolve_terminals` matches each declaration to its node by the claim occurrence, refuses a declaration without a node or a node without a declaration, and takes the `when` predicate from the checked body that the occurrence names. qsl-route `opts_out` now reads `terminal_nodes()`. The only remaining production read of `graph().terminals()` is inside `resolve_terminals`. The FR-411 Behavior amendment states this read path, and `quire validate` exits 0 on it. `resolution_follows_the_node_not_the_checked_declaration` removes the node and keeps the S3 declaration, and expects `Disagrees { model: "Config" }`. A read of the S3 value alone would return one terminal, so that test would fail. |
| FND-007 | fixed | `resolve_terminals` no longer emits, so a probabilistic package with a terminal builds again. `terminal Stop using v on M when { self.delivered or self.attempts = 2 }` is restored in `link()`. `link_steps_weigh_operations_then_draws` (FR-187-AC-2) and `a_terminal_state_continues_on_stutter_steps` pass with it. |
| FND-008 | fixed | The optional `serde_json` dependency and its entry in the `std` feature are removed from qsl-package/Cargo.toml. |
| FND-009 | fixed | A node that no declaration claims now gives `Disagrees { model: <the node's id> }`. |

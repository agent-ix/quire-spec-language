---
id: SR-13657
title: "Code review of quire-spec-language PR #1137: QSL-563 protocol refinement rows, the S3 half of FR-214"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@c63e977a4; PR #1137 (git diff origin/main...c63e977a4): qsl-semantics/src/check/{mod,refinement,step_simulation}.rs, qsl-semantics/src/check/refinement/{protocol,step_label}.rs, qsl-semantics/src/check/refinement/protocol/tests.rs, qsl-semantics/src/check/lowering/protocol.rs, qsl-semantics/tests/it/refinement_declarations.rs, qsl-analyze/src/model_check/{check,refine}.rs, qsl-analyze/tests/it/{main,refinement_protocol}.rs, qsl-replay/src/certificate/{closure,simulation}.rs, qsl-replay/src/refinement/{step,subject}.rs, tests/support/refinement_cas.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-214
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-135
    type: reviews
---
# SR-13657: Code review of quire-spec-language PR #1137: QSL-563 protocol refinement rows, the S3 half of FR-214

## Summary

Ticket: QSL-563 (first slice: FR-214's S3 half). This is the code review, with the rust-review lane folded in.

Gates run at c63e977a4 in a fresh clone with its own `CARGO_TARGET_DIR`. I did not run `make ci`. Every gate passed (5,547 tests passed, 0 failed):

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p qsl-eval -p qsl-replay -p qsl-analyze -p qsl-semantics -p qsl-cst -p qsl-forms -p qsl-package`
- root `cargo test --test it`
- `make string-edge`
- `make arch-lint-api-surface-qsl`, `make arch-lint-canonical-encoder`, `make arch-lint-qualified-core`

The existing model-subject refinement tests (qsl-semantics `refinement_declarations`, qsl-replay, qsl-analyze) all pass, so the two-pass split keeps model-subject verdicts. Diff review of the split: a model subject is still checked and refused at its old place, and its refusal still stops the unit at the same points. Two model-subject changes remain: a keyworded or unknown-middle-segment row now refuses at a segment, not the whole name (FND-004), and the classify refusals are pushed before the operation-row refusals.

Independent oracle. I wrote my own generator and oracle from the merged FR-136 text: random protocols with 0 to 2 channels of each delivery policy, 0 to 2 replicated roles of each lifetime (`workflow`, `scope`, `until`), `on origin` or `on each`, an optional compensation template (with its effect and commit), events, attempts of both operations, and `parallel` nodes under none, `sc`, `tso` and `ra`, some with a nested `parallel`. For each one it draws random row subsets: the class rows, operation rows, `-> any` duplicates, invalid rows (a `lose` on an `at-least-once` channel, `retire` on a non-`until` role, `fork`/`join` on an attempt or sequence, `cend` on an attempt, `activate` on an `on origin` clause, `memory flush` without `tso`, `memory nope`), in random order. The clause is placed before or after the refinement. About 1 case in 15 drops every row that names the clause, which gives a model subject. For each case the oracle derives the full multiset of (code, cause, located text, missing-row name) and compares it with the checker's. 7,800 cases over six seeds agreed exactly, with 0 invalid generated protocols: 423 checked, 7,377 refused, 536 model subjects, 1,938 with a template.

Mutation testing: 55 mutants against the PR's own tests (qsl-semantics `check::refinement::protocol::tests`, plus qsl-analyze `refinement_protocol` for the engine arm). 50 were killed and 5 survived. Killed: class derivation per node kind (event, effect, commit, send, receive, await, fork, join, finish, trigger, template cattempt and cend), each delivery rule for `duplicate` and `lose`, spawn, retire-only-for-`until` both ways, `ra` and `tso` memory kinds, ignoring the memory model, first parallel only, operation-row coverage both ways, operation rows required under a protocol, selection and conflict-once, malformed-once, conflict naming itself, `acts_on` always true, refusal locations (segment offset, whole name, missing row at first row), types-first classification, coverage over a refused clause, a refused clause dropping pending refusals, class conflict, `memory` deferral under a model subject, `concrete_protocol` unset, missing-selection dropped, ambiguity ignored, the three observation arms, the `open_systems` arm and the `step_row_refs` arm. The survivors:

- M06 (all `parallel` nodes instead of the outermost) is equivalent, because a nested `parallel` cannot differ from its outermost one's model (the clause refuses that).
- M28 (no source sort of `steps`) is not required by the spec, since `CheckedRefinement.steps` may keep source order.
- M33, M34 and M47 are real gaps (FND-003).

Spot probes: FND-001 reproduced. Also run: the FR-136-AC-4 scenario as written (the gap-analysis), a `fence` with and without a row (the gap-analysis), a mix of malformed and conflicting protocols (correct), and a keyworded row naming a type (FND-004).

Rename `StepRowOverflow` to `StepRowRefusal`: this is justified, because the error gains a second cause. Every caller is updated (two test call sites; there is no production caller). No spec text names the old type, and there is no compat alias. The engine arms refuse with typed causes (`SubjectRefusal::ProtocolSubject` maps to `ModelCheckRefusal::ProtocolRefinement` and `ClosureRefusal::ProtocolRefinement`; `StepRowRefusal::ProtocolSubject`), with no panic. No new production `unwrap`, `expect` or `panic`. The slice indexing in `classify` and `NameSegments::span` is bounded by the slice pattern that precedes it. No dead code found.

## Verdict

**Not mergeable as it stands.** FND-001 is a real defect, and FND-002's tags misstate coverage. The rest of the slice is sound: the coverage rule matches an independent oracle over 7,800 random cases, and 50 of 55 mutants are killed.

- **FND-001 (medium).** A protocol refinement's own refusals are lost when the unit stops at the refusal check before the protocols loop.
- **FND-002 (medium).** Five tests carry FR-136 AC tags whose text they do not assert.
- **FND-003 (low).** Three code paths have no killing test.
- **FND-004 to FND-008 (low).** Diagnostic location, an honest-but-premature refusal, a duplicated keyword table, an AC-led doc comment, and a broadened shared fixture.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | A protocol refinement's own refusals are dropped when the unit stops before the protocols loop. `check_refinement` keeps a protocol subject's refusals in the pending `ProtocolRefinement`, not in `refusals`. But `if !refusals.is_empty() { return Err(refusals); }` at mod.rs:1738 returns as soon as any other refusal exists, such as a model-subject refinement's or an FR-218 repeated reference. The pending refusals are then never reported. Reproduced: a unit with `OverFill` (rows of `Fill` with `step M::Node::step -> A::Acc::add(self, 1);`) reports `ill_typed`/`operator-ineligible` alone. Add a refinement `OverModel` (model subject, missing the `undo` row), in either order, and the unit reports only `OverModel`'s `missing-name`: the `operator-ineligible` refusal disappears. FR-136 requires every refinement's header, population, object and operation rows to be checked "wherever the refinement is reached", and FR-136-AC-8 that "a refusal of a row not drawn from the clause is still reported". The `cancel` return at mod.rs:2044 drops them the same way. Fix: drain `protocol_refinements`' refusals into `refusals` before each early return after the refinements loop (or keep the refusals in `refusals` and the pending state alone in the slot), and add a test with a refusing model subject beside a refusing protocol subject. | qsl-semantics/src/check/mod.rs:1738, qsl-semantics/src/check/mod.rs:2044, qsl-semantics/src/check/refinement.rs:1003-1013 |
| FND-002 | medium | Five tests carry FR-136 AC tags whose text they do not assert. FR-136-AC-4 states fork-first refusal order, the attempt-node row `-> any` taking precedence, and two `join` rows conflicting, over incA/incB. But `a_cattempt_is_covered_by_its_operations_row_or_its_templates` (:917), `a_replicated_role_needs_a_spawn_row_and_an_until_role_a_retire_row` (:1045), `a_send_and_a_receive_need_node_rows` (:1221) and `an_await_needs_a_timeout_row_and_its_matched_event_a_row` (:1256) test cattempt, spawn/retire, send/receive and timeout coverage, which no FR-136 AC states (gap-analysis FND-004). `a_row_naming_no_class_of_the_clause_refuses_at_its_name` (:948) is tagged FR-136-AC-6 but tests judgement call 3 (a `fork` of an attempt, a `spawn` of a static role, an `activate` of an `on origin` clause), which AC-6 does not state. The assertions AC-4 does state are made in `a_missing_or_doubled_row_refuses` and `an_attempt_node_row_takes_precedence_over_its_operation_row`, which carry only FR-214-AC-2 and FR-214-AC-3. Fix: tag the five with TC-541 alone, and add FR-136-AC-4 to the two tests that assert its text. | qsl-semantics/src/check/refinement/protocol/tests.rs:917, :948, :1045, :1221, :1256, :427, :516 |
| FND-003 | low | Three code paths have no killing test (mutants M33, M34, M47 survive). (a) A protocol row whose target is an abstract node (`StepTargetForm::Node`, malformed while the abstract side is a model subject) can be accepted as `stutter` with every test green. (b) An application target on a protocol row (`UnsupportedFeature::ProtocolStepApplication`) can be accepted as `any` with every test green. (c) `complete` can be handed the first checked clause instead of the selected one (`checked_protocols.get(place)` to `.get(0)`). It survives because the only two-protocol test, `each_refinement_records_the_node_of_its_own_clause`, uses two clauses of the same shape. Fix: a test per refusal, and a two-clause test whose clauses need different rows. | qsl-semantics/src/check/refinement/protocol.rs:679-697, qsl-semantics/src/check/mod.rs:2063 |
| FND-004 | low | Under a model subject, the new classification moves some refusals to a misleading segment. `step fork M::Node::step -> stutter;`, in a unit with no protocol named `Node`, refuses `missing-name` at `Node`, which is an existing object type. Before this PR the refusal was at the whole name. `step M::Nope::step` now refuses at `Nope`, but FR-136's model-subject bullet says 'a row whose left side names no concrete operation SHALL refuse `missing_declaration`/`missing-name` at its span'. Fix: when the middle segment resolves to a concrete type, refuse a keyworded row at the whole name (or as malformed). Ask /specify which location wins for an unknown middle segment under a model subject. | qsl-semantics/src/check/refinement/protocol.rs:165-182, qsl-semantics/src/check/refinement/protocol.rs:193-196 |
| FND-005 | low | An application on an attempt-node row or a template (cattempt) row refuses `unsupported_construct`/`not-yet-implemented` (`ProtocolStepApplication`). FR-136 'Right sides' types an application 'with `self` bound to the concrete receiver, the concrete step's arguments in scope by parameter name'. For an attempt or a cattempt the concrete step is an application of a known operation, so that context exists. Only event, control, channel, role, trigger and memory rows lack one. The refusal is honest, but it refuses a spelling the spec admits. Fix: type attempt-node and template rows as their operation's row is typed, or record the restriction as remaining work on QSL-563. | qsl-semantics/src/check/refinement/protocol.rs:685-697 |
| FND-006 | low | The step-keyword spelling is written twice. `spelled` maps `StepClassKeyword` to `fork`, `join`, `cend` and the rest by hand, beside qsl-forms' parse table (`b"duplicate" => StepClassKeyword::Duplicate`, …). A keyword added to one place can silently miss the other. Fix: one `StepClassKeyword::as_str` in qsl-forms, used by both. | qsl-semantics/src/check/refinement/protocol.rs:537-546, qsl-forms/src/refinement.rs:430-445 |
| FND-007 | low | A doc comment opens with an AC id: `/// FR-214-AC-1's refinement of `A::Acc::add` by `Fill`, …` on the `refinement` helper. Fix: open with what the helper builds, and cite the AC later in the sentence. | qsl-semantics/src/check/refinement/protocol/tests.rs:272 |
| FND-008 | low | The shared CAS fixture now selects the protocol profile (`profile pp = "quire.protocol.finite-global/v1"`) for every unit it builds, so every CAS-based model-subject test (qsl-semantics, qsl-replay, qsl-analyze and root) runs under a profile that only `refinement_protocol.rs` needs. Fix: add the profile in `protocol_refinement()`'s unit, or as a fixture option. | tests/support/refinement_cas.rs:371 |

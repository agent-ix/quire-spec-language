---
id: SR-13658
title: "Gap analysis of QSL-563 PR #1137 (protocol refinement rows, S3 half of FR-214) against FR-135, FR-136, FR-146, FR-147, FR-214, FR-310"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@c63e977a4; PR #1137 (git diff origin/main...c63e977a4): qsl-semantics/src/check/{mod,refinement,step_simulation}.rs, qsl-semantics/src/check/refinement/{protocol,step_label}.rs, qsl-semantics/src/check/refinement/protocol/tests.rs, qsl-analyze/tests/it/refinement_protocol.rs, qsl-replay/src/refinement/subject.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-214
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-135
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-146
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-541
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-659
    type: reviews
---
# SR-13658: Gap analysis of QSL-563 PR #1137 (protocol refinement rows, S3 half of FR-214) against FR-135, FR-136, FR-146, FR-147, FR-214, FR-310

## Summary

Ticket: QSL-563 (first slice). Tools: `quire matrix --scope . --format tsv` (quire 0.36.2, engine 0.50.2), exit 0, at c63e977a4. Plan completion: not assessed.

Matrix at c63e977a4 for the ACs in scope:

| AC | Status | Honest? |
| --- | --- | --- |
| FR-214-AC-1, AC-2, AC-3, AC-5 | tagged | yes |
| FR-214-AC-4 | tagged (:759) | first sentence only; the internal-list half is not built (FND-001) |
| FR-136-AC-4 | tagged (5 tests) | four of the five tag the wrong text (SR-13657 FND-002); AC-4's own assertions are under FR-214 tags |
| FR-136-AC-6 | tagged | yes, except :948 (SR-13657 FND-002) |
| FR-136-AC-8 | tagged (:710) | yes |
| FR-136-AC-9 | tagged (:558) | first sentence only; the other-package sentence is untested (FND-001) |
| FR-136-AC-10 | tagged (:1084) | S3 half only; the request half is not built (FND-001) |
| FR-136-AC-7, FR-146-AC-5 | untagged | honest: sort key not built |
| FR-135-AC-10 | untagged | honest, but its second half (`concrete_protocol`) is built and tested at :398 without the tag |
| FR-310-AC-5, FR-147-AC-1..13 | untagged | honest: declared not delivered |

Code with no owning requirement: the engine arms (`SubjectRefusal::ProtocolSubject`, `ModelCheckRefusal::ProtocolRefinement`, `ClosureRefusal::ProtocolRefinement`, `StepRowRefusal::ProtocolSubject`) are stopgap refusals that no AC states. They are tagged by TC only (TC-547, TC-556, TC-551), which is acceptable as interim behaviour, but each is the opposite of what FR-141-AC-5, FR-145 and FR-146-AC-5 will require. No stubs. The coverage rule matches an independent brute-force oracle over 7,800 random protocols, and 50 of 55 mutants are killed (SR-13657).

Classification of the coder's five spec gaps (merged #1126 text):

1. **FR-136-AC-4 contradicts 'Selecting the concrete protocol': a spec defect.** The probe confirms it: with rows for `step` and `undo` only, `Fill`'s refinement checks as a model subject with `concrete_protocol` `None`. AC-4's first sentence describes a state the rules cannot reach (FND-002).
2. **'Protocol nodes have no NodeKey': a coder misreading.** The merged FR-113 (intro) says 'the package holds the protocol nodes FR-383 specifies for it: the `protocol_clause` node … and the nodes of its roles, channels, queues, `run` tree, `finish`, `terminal` and compensation templates'. FR-383 'Nodes and keys' says '`check` SHALL mint the key of every protocol node by FR-092 rule 2'. The sentence the coder quoted is a stale module doc in qsl-semantics/src/check/lowering/protocol.rs:11-12, not spec text. What is missing is FR-383's node minting in the code, a prerequisite, not a spec defect. FR-136-AC-7 and FR-146-AC-5 are blocked on it (FND-006).
3. **missing-selection without a protocol profile: an unspecified edge, not a defect.** The coder's choice matches lowering's existing rule for protocol references (lowering/protocol.rs:61-67). A sentence in FR-135 or FR-136 would settle it (FND-005).
4. **`fence`: a spec defect.** ADR-027 SE-5 says a fence is a step 'which refinement rows (PR-1) … see', and FR-136's sort key lists the 'fence node'. But PR-1 and FR-136's coverage list name no fence row, and qsl-forms' `StepSourceForm::Named` doc already lists 'a fence' among unkeyworded names. Probe: `Fill` under `tso` with `fence Barrier ordering seq_cst;` checks with no fence row, and a `step M::Fill::Barrier` row refuses `missing-name` (FND-003).
5. **Typing of an application on a protocol row: a real gap for event, control, channel, role, trigger and memory rows, but not for attempt-node or template rows.** FR-136's 'self bound to the concrete receiver, the concrete step's arguments' does give a context for an attempt or cattempt step (SR-13657 FND-005). For the other rows, ADR-027 PR-2 needs 'a row to an abstract node or operation' to be typable once the abstract side is a protocol, and FR-136 says nothing (FND-004).

The PR body's spec gap 5 (FR-136-AC-9's other-package sentence) is not a spec defect. It holds by construction, and only a fixture is missing (FND-001).

## Verdict

**Not mergeable for coverage until FND-001 and SR-13657 FND-002 are fixed.** Both are tag and doc fixes in this PR. FND-002 to FND-005 are spec follow-ups for /specify and do not block this slice. FND-006 is remaining work, to record on QSL-563.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Three ACs read as tagged in full, but only part of each is built or tested, and no test doc says so. FR-214-AC-4 has two sentences. The test at :759 asserts only the first ('A row `-> any` gives `Either`'). The second ('An abstract protocol whose internal list names `fork` as internal gives that abstract step internal') needs FR-147's internal list, which is declared not delivered. FR-136-AC-10's test (:1084) covers the S3 half, but not 'A check request that selects `tso` for the `sc` protocol … is refused when admitted' or 'a request that selects `sc` for the `tso` protocol admits'. FR-136-AC-9's test (:558) covers P, Q, R, but not 'A protocol clause declared in a package other than the declaring one … refuses `missing_declaration`/`missing-name` at its segment'. Fix: say in each test doc which half it covers, as earlier slices did, and list the rest on QSL-563. Also tag :398 with FR-135-AC-10, whose second half it asserts. | qsl-semantics/src/check/refinement/protocol/tests.rs:759, :1084, :558, :398 |
| FND-002 | medium | FR-136-AC-4's first scenario cannot occur. With 'rows for both operations and no `fork`, `join` or `finish` row', no row names the protocol clause, so by 'Selecting the concrete protocol' the subject is a model subject. It checks, where the AC says it refuses naming the `fork` node. Reproduced: the same rows over `Fill` check with `concrete_protocol` `None`. The PR's tests use a `finish` row to select the clause. Fix: route to /specify. Add the `finish` row (or an attempt-node row) to AC-4's first scenario, and the expected refusals become `fork` then `join`. | spec/functional/FR-136-check-a-refinement-s-step-rows.md:FR-136-AC-4 |
| FND-003 | medium | ADR-027 says a `fence` step is seen by refinement rows, but no FR-136 coverage rule requires a row for it. SE-5 says a fence 'must be a position at which the memory model acts and which refinement rows (PR-1) and intervals (PB-3) see under every memory model'. FR-136's protocol sort key lists 'the attempt, event, send, receive, timeout, finish or fence node'. But PR-1's list and FR-136's 'Coverage over a protocol subject' list name no fence row. The code follows the coverage list, so a protocol with a `fence` checks with no row for it, and a row naming the fence refuses `missing-name`. Under RU-5 ('every step class keeps an explicit row'), a fence step would pass a refinement unmapped. Fix: route to /specify. Either add 'each `fence` node SHALL have a node row' to PR-1 and FR-136 with an AC, or strike the fence from SE-5's refinement clause and FR-136's sort key. | spec/functional/FR-136-check-a-refinement-s-step-rows.md:Coverage over a protocol subject |
| FND-004 | low | Several FR-136 coverage bullets have no acceptance criterion: a `cattempt` covered by its operation or template row, a `send`, `receive` or `timeout` node row, a `spawn` row and an `until` role's `retire` row, and the `activate` trigger row. Only `cend` and channel rows appear in FR-214-AC-3. The PR tests them all and tags them FR-136-AC-4 (SR-13657 FND-002). FR-136 'Right sides' also does not give the typing context for an application on a non-operation protocol row (spec gap 5). Fix: route to /specify. Add ACs for these coverage classes. State the typing context of an application on an event, control, channel, role, trigger or memory row, or that such rows admit only `stutter` and `any` while the abstract side is a model subject. | spec/functional/FR-136-check-a-refinement-s-step-rows.md:Coverage over a protocol subject |
| FND-005 | low | FR-135 and FR-136 do not say what a refinement whose rows select a clause does in a unit that selects no protocol profile. There the clause has no node, so `concrete_protocol: Option<NodeKey>` has nothing to hold. The PR refuses `missing_declaration`/`missing-selection` at the first row naming the clause, as lowering already does for a protocol reference. That is a sound reading, but it is not stated. Fix: route to /specify, one sentence and an AC. | spec/functional/FR-136-check-a-refinement-s-step-rows.md:Selecting the concrete protocol |
| FND-006 | low | The new docs give the wrong reason why a protocol subject's rows have no order. They say a protocol node 'has no node key of its own (FR-113)': `StepClass`'s doc, `StepRowRefusal::ProtocolSubject`'s doc and its error text, and the PR body's judgement call 1. The merged FR-113 and FR-383 say the opposite: check mints a key for every protocol node. The real blocker is that FR-383's node minting is not built, so FR-136-AC-7 and FR-146-AC-5 wait on it, and `StepClass` names nodes by `ProtocolNodeId` and names, not FR-136's `NodeKey`s, until then. Fix: cite the missing FR-383 implementation in the docs, and record on QSL-563 that `StepClass`'s node fields become `NodeKey`s with FR-136-AC-7. | qsl-semantics/src/check/refinement.rs:504-509, qsl-semantics/src/check/step_simulation.rs:41-46 |

---
id: SR-13623
title: 'Spec review of PR #1126: protocol refinement rows, integrity'
type: SpecReview
analysis: integrity
scope: 'agent-ix/quire-spec-language@74e87f43e; git diff origin/main...74e87f43e (PR #1126, branch qsl-563-spec, base main 6eceb158b): ADR-015, FR-027, FR-098, FR-099, FR-135, FR-136, FR-146, FR-147, FR-214, FR-310, TC-444, TC-446, TC-540, TC-541, TC-551, TC-659, TC-889; read against origin/main code (qsl-semantics check, qsl-cst grammar, qsl-replay request, src/command/wire.rs) and specs (FR-071, FR-141 to FR-148, ADR-015, ADR-027) and QSpec main 8736706 (FR-322, FR-323, FR-375, FR-432, shared grammar); ticket QSL-563'
review_set: subset
date: '2026-10-21'
relationships:
- target: ix://agent-ix/quire-spec-language/ADR-015
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-027
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-098
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-099
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-135
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-136
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-146
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-147
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-214
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-310
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-444
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-446
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-540
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-541
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-551
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-659
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-889
  type: reviews
---

## Summary

Ticket: QSL-563. PR quire-spec-language#1126, head 74e87f43e. Integrity lens: whether FR-135, FR-136, FR-146, FR-147, FR-214, FR-310, FR-099, FR-098, FR-027 and ADR-015 still agree with each other, with the merged ADR-027 PR-1 and PS-1 and FR-141 to FR-148, and with QSpec main 8736706 (FR-375, FR-432, FR-322, FR-323).

These agree: the protocol sort key in FR-136, FR-146 and FR-146-AC-5; the selection rule in FR-135, FR-136 and FR-214; the `concrete_protocol` field (FR-135) and how FR-136-AC-6 and FR-135-AC-10 use it; FR-310's production and refusals with FR-375 and FR-375-AC-11; FR-214-AC-5 with FR-136-AC-8; and the TC steps with their ACs. The one exception is the AC-7 fixture in FND-001. The refusal reuses fit the siblings and FR-272. No other merged spec still says 'pending a QSpec FR-375/FR-432 ruling' or 'STD-182'.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-136 sort key | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-136 StepRowRef | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-136 select rows | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-136 one protocol | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-136 two protocols | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-136 unknown protocol | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-136 memory row | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-136 order models | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-136 order protocol | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-136 unused operation | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-136-AC-4 | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-136-AC-6 | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-136-AC-7 | spec/functional/FR-136-check-a-refinement-s-step-rows.md | examined |
| FR-146 protocol order | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-146-AC-5 | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-147 order | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md | examined |
| FR-147 library defects | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md | examined |
| FR-099 library defects | spec/functional/FR-099-compile-against-supplied-libraries.md | examined |
| FR-099-AC-9 | spec/functional/FR-099-compile-against-supplied-libraries.md | examined |
| FR-098 read internals | spec/functional/FR-098-execute-a-replay-request.md | examined |
| FR-098-AC-20 | spec/functional/FR-098-execute-a-replay-request.md | examined |
| FR-135 header | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md | examined |
| FR-135 order | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md | examined |
| FR-214 select | spec/functional/FR-214-require-an-explicit-refinement-row-for-every-protocol-step-class.md | examined |
| FR-214-AC-5 | spec/functional/FR-214-require-an-explicit-refinement-row-for-every-protocol-step-class.md | examined |
| FR-310 header | spec/functional/FR-310-build-the-s2-form-of-a-refinement-declaration.md | examined |
| FR-310 syntax refusals | spec/functional/FR-310-build-the-s2-form-of-a-refinement-declaration.md | examined |
| FR-310-AC-5 | spec/functional/FR-310-build-the-s2-form-of-a-refinement-declaration.md | examined |
| TC-541 | spec/test-cases/TC-541-s3-checks-a-refinement-s-step-rows.md | examined |
| TC-551 | spec/test-cases/TC-551-per-step-simulation-records-are-written-and-settled.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-7's fixture contradicts AC-6. AC-7 runs 'Over FR-136-AC-4's protocol subject' with 'a `duplicate` and a `lose` row for one channel and a `memory flush` row'. AC-4's protocol is a `parallel` of two attempts, with no channel and no memory clause, so it runs under `sc`, which has no `flush` kind. AC-6 and the memory-row bullet say a row 'naming a kind the selected memory model lacks' refuses `missing-name` at the kind. A row for a channel that does not exist cannot resolve either. As written, AC-7 refuses instead of emitting the order it asserts. FR-146-AC-5 and TC-551 step 5 inherit the same fixture. Define the AC-7 fixture: add a channel whose delivery is `unknown` (so that both `duplicate` and `lose` are classes, as in FR-214-AC-3's `Chatter`), and put the `parallel` under `tso`. | spec/functional/FR-136-check-a-refinement-s-step-rows.md:232; spec/functional/FR-136-check-a-refinement-s-step-rows.md:229; spec/functional/FR-136-check-a-refinement-s-step-rows.md:231; spec/functional/FR-136-check-a-refinement-s-step-rows.md:132-135; spec/test-cases/TC-541-s3-checks-a-refinement-s-step-rows.md:33-42; spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md:175; spec/test-cases/TC-551-per-step-simulation-records-are-written-and-settled.md:32-33 |
| FND-002 | medium | FR-147's new order rule contradicts FR-136, and no AC tests it. FR-147 says 'an abstract protocol declared in the declaring package is checked before the refinement that names it is resolved'. So for an abstract protocol subject the header waits for the protocol. FR-136 'Order of the checks' says 'A refinement's header, population, object, fairness and history rows ... depend on the models only and SHALL be checked wherever the refinement is reached', and FR-135 delays only concrete coverage. Two coders would put abstract-header resolution in different passes. Say in FR-136 that resolving an abstract protocol header also waits for the clause, and add an AC (in FR-147 or FR-214-AC-5) for an abstract protocol declared after the refinement. | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:84-87; spec/functional/FR-136-check-a-refinement-s-step-rows.md:144-146; spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md:109-111 |
| FND-003 | medium | The replay locus does not match QSpec. QSpec FR-323-AC-17 locates a recompile refusal of the list 'at `/package/refinement_internals/<pointer>` for the proved package's value and at `/package/dependencies/<n>/refinement_internals/<pointer>`', which is one request pointer. FR-098 says the defect is 'located by the input and pointer (FR-147)', and FR-098-AC-20 says it is 'located at the root pointer of the package's value'. That is the pair (input, ""), and FR-147 says the pair 'names no member of the compile request'. QSL states no rule that turns the pair into the request pointer FR-323 requires. Say that `ReplayRefusal::Recompile` reports the pair as `/package/refinement_internals` (or `/package/dependencies/<n>/refinement_internals`) followed by the pointer, and assert it in AC-20. | spec/functional/FR-098-execute-a-replay-request.md:451; spec/functional/FR-098-execute-a-replay-request.md:97-104; spec/functional/FR-099-compile-against-supplied-libraries.md:93-99 |
| FND-004 | medium | The carve-out for an unreached library goes against QSpec, and the PR body's QSpec follow-up leaves it out. FR-099 runs the source-dependent checks ('that a `refinement` names a refinement of it and that an entry resolves against it') 'only for a library that is compiled'. FR-099-AC-9 says that a value naming a refinement an unimported library does not declare 'refuses nothing'. QSpec FR-375 says 'A refusal from a supplied library's value refuses the whole compilation: the unit is not admitted, whether or not it imports that library'. QSpec FR-375-AC-12 says a value naming a refinement the library does not declare refuses `missing-name` at `/0/refinement` and the unit is not admitted. Merged FR-147 states the same 'whether or not' rule with no exception. The split may be the right reading of ADR-015 D-1, where an unreached library is not compiled. But it changes a QSpec result, so it belongs in the PR's 'Proposed QSpec wording' as an FR-375 change. FR-147's paragraph should name the exception too. | spec/functional/FR-099-compile-against-supplied-libraries.md:83-89; spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:166-167; spec/functional/FR-099-compile-against-supplied-libraries.md:183 |
| FND-005 | medium | 'protocol clause of the concrete model's package' names the wrong package. A protocol clause is declared in the unit (`protocol Fill using p over (k: M::Cell)`, ADR-027 §7), and its roles act `on M`. QSpec FR-375 says the concrete subject is 'the declaring package'. The concrete model alias, though, can select a model from a domain package, which declares no protocol clause of the unit. Read literally, 'the concrete model's package' is that domain package, and then every `<C>::<protocol>` would refuse `missing-name`. The same phrasing is in 'names the protocol clause `<protocol>` of the concrete model' and 'among the concrete model's object types and protocol clauses'. The proposed FR-432 wording says 'the declaring model' and 'the concrete package'. Use 'a protocol clause of the declaring package whose roles act on `<C>`' throughout, as FR-375 does for the abstract side. | spec/functional/FR-136-check-a-refinement-s-step-rows.md:127-131; spec/functional/FR-136-check-a-refinement-s-step-rows.md:107-115; spec/functional/FR-136-check-a-refinement-s-step-rows.md:123-126 |
| FND-006 | low | The admitted-operation-row bullet leaves out `cattempt`. It says 'An operation row whose operation no attempt of the protocol uses decides nothing' and 'an operation row is required only where an attempt of that operation has no node row'. FR-432 and ADR-027 PR-1 say a `cattempt` is decided 'by its template row or else its operation row'. An operation used only by a compensation template is used by no attempt, yet its row does decide that `cattempt`. And when the template has no row, the operation row is required. Say 'no attempt or compensation template', and 'where an attempt or `cattempt` of that operation has no node or template row'. | spec/functional/FR-136-check-a-refinement-s-step-rows.md:189-191; spec/functional/FR-136-check-a-refinement-s-step-rows.md:231 |

## Verdict

Not mergeable as it stands. FND-001 is an AC whose fixture fails by the PR's own rules. FND-002 is an order rule two FRs state differently. FND-003 and FND-004 are differences from QSpec that the PR neither fixes nor lists for follow-up. FND-005 is a term two implementers would read differently. All are text fixes in this PR.

## New findings (disposition pass 1)

Head 3ca0af76e (PR #1126 round 1).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | low | The coverage list still says 'each memory step kind the memory model has SHALL have a memory-step row', singular. The new memory bullet makes the required kinds 'the union of those models' kinds' over the outermost `parallel` nodes, and says one row covers a kind for all of them. Write 'each memory step kind of the protocol's memory models (above)', so the two bullets agree. | spec/functional/FR-136-check-a-refinement-s-step-rows.md:210 |

## Dispositions

Round 1, head 3ca0af76e (origin/main merged at 4f0c65d6e), checked against origin/main code and QSpec main 8736706.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Fixed in 3ca0af76e: FR-136-AC-7 \| Over FR-136-AC-4's protocol extended with a channel `ch` whose delivery is `unknown` (so `duplicate` and `lose` are both classes) and with its outermost `parallel` under `tso` (so `flush` is a memory step kind), with `fork`, `join` and `finish` rows, a `duplicate` and a `lose` row for  |
| FND-002 | fixed | Fixed in 3ca0af76e: S3 SHALL check the protocol clauses of the unit before it checks the coverage of a refinement over one of them, and before it resolves the abstract protocol header of a refinement that names one declared in the unit. |
| FND-003 | fixed | Fixed in 3ca0af76e: The executor SHALL report the locus of such a refusal as the request pointer, `/package/refinement_internals` for the proved package's value or `/package/dependencies/<n>/refinement_internals` for the `n`th dependency entry's, followed by the pointer into the value that FR-147 gives (QSpec FR-323-AC |
| FND-004 | fixed | Fixed in 3ca0af76e: whole compilation, the unit included, whether or not the unit imports the library (QSpec FR-375, FR-375-AC-12; FR-147-AC-10). This holds for every defect, including a `refinement` that names no refinement of the library. |
| FND-005 | fixed | Fixed in 3ca0af76e: <C>::<protocol>` (QSpec FR-432), names the protocol clause `<protocol>`, a protocol clause of the declaring package whose roles act on `<C>`. |
| FND-006 | fixed | Fixed in 3ca0af76e: The checker SHALL require an operation row only where an attempt or `cattempt` of that operation has no node row or template row. An operation row that no such step needs decides nothing and is admitted. |

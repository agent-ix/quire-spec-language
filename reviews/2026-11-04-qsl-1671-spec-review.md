---
id: SR-2331001
title: 'Spec review of PR #2331 (batch S6): FR-507-AC-7/8/11, FR-362 hyper emission, FR-218-AC-35 await'
type: SpecReview
analysis: base
scope: 'agent-ix/quire-spec-language@314999788; git diff origin/main...HEAD (PR #2331, branch peter/qsl-spec-s6); spec/functional FR-218 FR-362 FR-507; spec/test-cases TC-663 TC-1015 TC-1026 TC-1302; source PR #2329 STOPS; QSpec FR-290 FR-322 FR-331 FR-368 FR-464 FR-466'
review_set: subset
---

## Summary

Tickets: QSL-1671 (FR-507-AC-7/8/11), QSL-1677 (FR-218 await), QSL-1680 (FR-362-AC-7). One pass: spec-review with the EARS, integrity, dependency and criterion-strength sub-analyses by hand (no Jev client). Ids: FR-218-AC-35 is free on main and on every open PR head; FR-507 and FR-362 rows keep their ids. `quire validate --scope .` exit 0 on all seven changed files. No ticket id in any added line.

Verified against the code at the PR base (origin/main 7e88fe60c): there is no node-to-family reader in `qsl-analyze`. `AnalyzeItem::new(index, kind, Some(family), ...)` takes the family from the adapter (items.rs: temporal-trace for a hyper/derived item, protocol for a protocol clause, relation-claim only for `timed_refinement_item`, which is `temporal-satisfaction`; contract_check supplies state-model, value_check value); `AnalyzeItem::unadapted(.., None)` is the AC-9 test's only route. So the Dependencies note (the reader is a code deliverable; no adapter yields `operation-contract` on `relation-claim`; AC-7/8/11 not yet satisfied) is accurate. The stated records are what QSpec says they should be: FR-290's node-to-family table maps `claim`/`step_relation`, `relation`/`refinement`, `claim`/`state_graph`, `claim`/`hyperproperty`, `temporal`/`temporal_clause`, `state`/`operation_anchor` role `anchor`, and an `expression` at role `claim`, `declaration` or `expression`, to the families AC-11 lists; `item_kind` deadlock-freedom, time-lock-freedom and race-freedom decide `temporal-trace` whatever the anchor; FR-322-AC-88 puts the `declaration` occurrence at 0 and `generated` occurrences at ordinals 0, 1, 2 on a proof-eligible refinement node; FR-331 gives `settled_by` `disposition` or `result`.

FR-362: the production code emits a checked `hyper` clause (`qsl-semantics/src/check/hyper_clause/emission.rs`), and `TemporalCause::NotEmitted` is raised there only for the five cases of FR-488 "Clauses S4 does not emit" (timed profile, alias binding no object parameter, fairness set under a bounded profile, systems-interface constraint, unencodable read), in that order; FR-362 item 8, AC-7 and TC-1026/TC-1015 now say so. Not updated: FR-361-AC-4 (FND-001) and ADR-033 (FND-009).

FR-218-AC-35 against `qsl-semantics/src/check/protocol_clause/tests/graph.rs` (`an_await_matches_an_event_in_a_closed_window_under_a_temporal_profile`, `an_await_after_a_later_control_is_a_cycle`) and `refusal.rs`: every code and cause exists with the stated trigger. `AwaitedForm` is `unsupported_construct`/`declaration-form` (node, event, form); `AwaitWindow` is `ill_typed`/`type-mismatch` retaining lower and the written upper (5 and 1; 0 and none); `MissingTemporalProfile` is `missing_declaration`/`missing-selection` at the await; `CausalCycle` is `invalid_package`/`definition-cycle` with Wait and Later among the nodes; the repeat back edge is `EdgeKind::RepeatBack`; the checked await record is `CheckedAwait { node, clock "wall", within {2,5}, profile }`. The undeclared `using` alias is `ProtocolAnchorCause::MissingProfile`, catalog `missing_declaration`/`missing-name`, located at the alias; the AC names it only by position (FND-002). QSpec FR-466 "Per-operator checks" 2 and 6 and FR-464 "Profiles and laws" (await needs a temporal profile, `missing-selection`) own the rest; FR-466-AC-6 states the awaited forms and the cycle.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-507-AC-7 | spec/functional/FR-507-apply-each-capability-kind-to-its-declaration-family-only.md:193 | examined |
| FR-507-AC-8 | spec/functional/FR-507-apply-each-capability-kind-to-its-declaration-family-only.md:194 | examined |
| FR-507-AC-11 | spec/functional/FR-507-apply-each-capability-kind-to-its-declaration-family-only.md:197 | examined |
| FR-507 Dependencies note | spec/functional/FR-507-apply-each-capability-kind-to-its-declaration-family-only.md:210 | examined |
| FR-362 item 8 | spec/functional/FR-362-produce-the-checked-graph-and-the-type-report-from-one-source.md:76 | examined |
| FR-362-AC-7 | spec/functional/FR-362-produce-the-checked-graph-and-the-type-report-from-one-source.md:143 | examined |
| FR-218 await bullet | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:135 | examined |
| FR-218-AC-35 | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:614 | examined |
| TC-1302 rows 7, 8, 11 | spec/test-cases/TC-1302-apply-each-capability-kind-to-its-declaration-family-only.md | examined |
| TC-1026 step 4 | spec/test-cases/TC-1026-one-source-yields-both-and-a-checked-unit-is.md | examined |
| TC-1015 step 2 | spec/test-cases/TC-1015-forms-not-gated-raise-no-model-gate-refusal.md | examined |
| TC-663 scope and step 9 | spec/test-cases/TC-663-s3-checks-protocol-control-constructs.md | examined |
| FR-361-AC-4 | spec/functional/FR-361-gate-model-constructs-on-the-model-profile-selection.md:255 | context_only |
| ADR-033 moved families | spec/decisions/ADR-033-one-source-text-for-the-checked-graph-and-the-type-report.md:230 | context_only |
| QSpec FR-290, FR-322-AC-88..91, FR-331, FR-368, FR-464, FR-466 | quire-specification | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-361-AC-4 still says "a `hyper` clause is refused only as `NotEmitted`". This PR changes its verifying case TC-1015 step 2 (and FR-362 item 8, AC-7, TC-1026) to say a checked hyper clause over `behaviours` is emitted, so AC-4 now contradicts the test that verifies it. The author's search missed it. | spec/functional/FR-361-gate-model-constructs-on-the-model-profile-selection.md:255 |
| FND-002 | medium | FR-218's await bullet and AC-35 refuse "the `using` alias when no profile of the header declares it" and "the undeclared `using` alias at the alias" with no code. The real refusal is `missing_declaration`/`missing-name` (`ProtocolAnchorCause::MissingProfile`, refusal.rs); neither QSpec FR-466 nor FR-464 owns this QSL-level alias check, so the code cannot be left to the owner. As written any refusal at the alias passes. | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:614 |
| FND-003 | medium | The last clause of FR-507-AC-8 (and TC-1302 row 8's last item) puts `item_kind` `deadlock-freedom` on a `global-conformance` item. QSpec carries `item_kind` on `temporal-satisfaction` items over a model subject only (FR-331 line 135, FR-368 `item_kind` list, FR-368-AC-13/FR-331-AC-44 vectors); QSpec has no vector for the combination, so the request may be refused at the request schema before the family check and the criterion may be unsatisfiable. FR-290's "item_kind decides" rule is the only support. | spec/functional/FR-507-apply-each-capability-kind-to-its-declaration-family-only.md:194 |
| FND-004 | low | FR-507-AC-7's refinement half says "one model refinement (a `relation`/`refinement` node) with step rows R0 and R1" and names the `generated` ordinals 0, 1, 2. FR-322-AC-91 gives a refinement that is not proof-eligible (history field, `any` target, field without a row) only the `declaration` occurrence and refuses a `generated` entry, so the AC holds only for a proof-eligible refinement; the AC and TC-1302 fixture `M` do not say so. | spec/functional/FR-507-apply-each-capability-kind-to-its-declaration-family-only.md:193 |
| FND-005 | low | AC-7 counts "five claim occurrences" (the relation's and the refinement's four) while TC-1302 row 7 lists `schedulability` on each of `R`, `R'` and `M`'s four: six items, two of them (the model and code claims of one `claim`/`step_relation` node) on one occurrence. The two counts disagree and the AC does not say the model and code claims share an occurrence for the schedulability item. | spec/test-cases/TC-1302-apply-each-capability-kind-to-its-declaration-family-only.md:42 |
| FND-006 | low | FR-218 cites QSpec FR-466 item 2 as owner, which names "the form and the selected protocol profile" in the awaited-form refusal; the AC and the code name the await, the matched event and the form, and no profile. The two differ and the AC does not say QSL's is intended. | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:135 |
| FND-007 | low | The statement says an await matches a `receive`, an `effect` or an `event`; AC-35 and the code test exercise only `event`. A `receive` or `effect` match is untested and unverified, so that half of the statement cannot fail. | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:614 |
| FND-008 | low | TC-1026 step 4's fourth read is under-specified: the header it lists has no `ev` alias or declared event-position clock (FR-488-AC-25 refuses an absent clock selection), no operation named in the `fair weak` constraint, and the third read's "checking match and body" is not pinned to an encodable read (FR-488 cause 5). A tester can hit a different refusal than the `NotEmitted` the step expects. FR-362-AC-7 says the refusal names "the clause and its span" where the step says the constraint's span. | spec/test-cases/TC-1026-one-source-yields-both-and-a-checked-unit-is.md:20 |
| FND-009 | low | ADR-033 "Moved and unmoved families" still says a checked relation or hyper clause "is refused only as `NotEmitted` ... until an S4 emission writes it". FR-362 item 8 now contradicts it. FR-362 also gains citations of FR-488 with no relationship edge to it. | spec/decisions/ADR-033-one-source-text-for-the-checked-graph-and-the-type-report.md:234 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-010 | medium | TC-1026 step 4's fourth read reuses the hyper clause's body `always holds(...)` (no interval) under the event-position false-extension profile `ev`. Under a bounded profile every temporal operator needs a closed interval; an operator with none is refused `unsupported_construct`/`expression-form` at the operator (qsl-semantics `temporal_clause.rs` header rules, QSpec TC-238), so the read stops at S3 and never reaches the `NotEmitted` the step expects. The #2331 package tests for FR-488-AC-19 and AC-20 use `always [0, 3]` and `eventually [0, 2]` for this reason. | spec/test-cases/TC-1026-one-source-yields-both-and-a-checked-unit-is.md:20 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | FR-361-AC-4 now says a checked hyper clause over `behaviours` is emitted as `claim`/`hyperproperty` and refused `NotEmitted` only in FR-488's cases; it agrees with TC-1015 step 2 and FR-362. |
| FND-002 | fixed | The await bullet and AC-35 name `missing_declaration`/`missing-name` at the undeclared alias, which is `ProtocolAnchorCause::MissingProfile` in the code; TC-663 step 9 agrees. |
| FND-003 | fixed | AC-8 and TC-1302 row 8 drop the `global-conformance` item carrying `item_kind`. The four derived `temporal-satisfaction` items carry their `item_kind` and read `temporal-trace`; the two anchored on the protocol clause node are said to run although `temporal-satisfaction` applies to `protocol` nowhere (FR-290 family table: protocol kinds are finite-replay, global-conformance, monitorability, local-projection, refinement, realizability, composition), so an anchor-node reader would settle them `inapplicable-kind` with `family` `protocol`. Schema-valid: QSpec FR-368 allows `deadlock-freedom` over a protocol subject (FR-429) and `race-freedom` over a protocol subject, empty `fairness`, anchored on the subject's anchor; TC-271 NG-23 lists both anchored on a `protocol`/`protocol_clause` node; FR-290 "Reading the family from a request item" fixes `temporal-trace` whatever the anchor. The `global-conformance` `formula` item stays the protocol-family item. Row 8 counts five running items. |
| FND-004 | fixed | AC-7 and fixture `M` say proof-eligible (FR-322-AC-91) with the reason. |
| FND-005 | fixed | AC-7 and TC-1302 row 7 agree: five occurrences, five `schedulability` items (R and R' share one). |
| FND-006 | fixed | The await bullet and AC-35 add the selected protocol profile to the awaited-form refusal; the code's `AwaitedForm` carries `profile` (awaits.rs, refusal.rs). |
| FND-007 | fixed | AC-35 and TC-663 step 9 add a `receive` (over a channel, of a `send`) and an `effect` (of an `attempt`) match, each checking and recording `Wait`; `awaits.rs` admits `Receive`, `Effect` and `Event` (no #2331 test yet, a coder deliverable). |
| FND-008 | still-open | The `ev` alias, the declared clock (FR-488-AC-25), the `fair weak V::ConfigVersion::step` constraint, the third read's `match { a.step.op = b.step.op }` and body, and the constraint's span in AC-7 are now stated. The fourth read's body is still the unbounded `always holds(...)`, which a bounded profile refuses before emission: FND-010. |
| FND-009 | fixed | ADR-033 now says S4 emits relation and hyper clauses and `NotEmitted` covers FR-337's and FR-488's cases; FR-362 gains `references` edges to FR-488 and FR-180. |

Verified this round: `quire validate --scope .` exit 0 for all nine spec files and the author's SR file; no ticket id in added spec lines; FR-218-AC-35 unique.

## Verdict

Round 1: MERGEABLE after FND-010 (one-line fix: the fourth read's body is `always [0, 3] holds(v.versionNumber @ a = v.versionNumber @ b)` as the FR-488-AC-19/AC-20 tests write it). FND-001 to FND-007 and FND-009 are fixed and verified; FND-008 closes with FND-010. FND-003's new reading is right against FR-290, FR-368 and TC-271.

## Round 0 verdict

MERGEABLE after FND-001 and FND-002 are fixed. FND-003 should be fixed or dropped (a clause with no QSpec support is the riskiest part of the PR); FND-004 to FND-009 are low. The FR-507 Dependencies note, the FR-362 hyper rewording against FR-488 and the code, and FR-218-AC-35's vectors (including the alias refusal's existence and position, the closed-window and open-end refusals, the cycle and the repeat back edge) were verified against the code and QSpec. All three criteria rewritten in FR-507 can fail; they cannot be satisfied by the code until the node-to-family reader and a `relation-claim` `operation-contract` adapter land, as the note says.

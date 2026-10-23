---
id: SR-12365
title: "Gap analysis of quire-spec-language PR #1361: call_site selects process and state-machine operations (QSL-976)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@f25a87280; git diff origin/main(e5554bc9f)...f25a87280; FR-121-AC-25, FR-121-AC-26, FR-121-AC-27, TC-1162, FR-121 operation and field selection bullets, FR-115-AC-6, FR-116 Inputs and replay target resolution; QSpec FR-323 frame_operation"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-121
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1162
    type: reviews
---
# Gap analysis of quire-spec-language PR #1361 (QSL-976)

## Summary

Ticket: QSL-976. Plan completion: not assessed. Sources: Linear QSL-976
(data), FR-121 and TC-1162 as merged by STD-272 (#1245), FR-115, FR-116,
and the code at the head.

| Criterion | Test | Status |
| --- | --- | --- |
| FR-121-AC-25 (process `Flow::go`; `stop` refuses `UnknownOperation`) | `call_site_selects_a_process_operation` | covered; the frame assertion is weak (FND-004) |
| FR-121-AC-26 (state machine `place`, inherited `ExpressLifecycle::place`, unnamed refuses) | `call_site_selects_a_state_machine_operation_and_its_inherited_one` | covered; fails if inheritance or the declarer is wrong |
| FR-121-AC-27 non-owners | `call_site_refuses_a_non_owner_and_an_ambiguous_operation` | partly covered: no enum (FND-001); the mapping checks only `Declined` (FND-003) |
| FR-121-AC-27 diamond `C::touch` | same test | covered; fails if the frame is looked up before ambiguity is decided |
| FR-121 field selection, object types only | none | violated at the head (SR-12364 FND-001) |
| FR-115-AC-6 state-machine clause | none | untested (FND-002) |
| FR-116 process or state-machine `frame_operation` target | none | conforms to the spec, untested (FND-005) |

Refusal order was checked against wrong implementations. If
`NotAnOperationOwner` came after resolution, non-owners would be
`UnknownOperation`. If ambiguity were checked after the frame lookup, `C`
would be `UnknownOperation`. The AC-27 test fails in both cases.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-121-AC-27 and TC-1162 step 3 list an enum among the non-owners. The test's owner list is record value type, event type, scalar type, persistence interface and population, with no enum. The fixture `owners_document` declares no enum or variant type. The code would refuse an enum through `non_owner_declarations`, but no test shows it. Fix: add a `variant_type` declaration to `owners_document` and to the `owners` list. | qsl-replay/src/spine/clause/tests/call_site_owners.rs:467-473 |
| FND-002 | medium | STD-272 amended FR-115-AC-6: over TC-1045's package, `M::OrderLifecycle::place` resolves to the state machine's key and selects its frame. This PR makes that true through the shared `resolve_operation` and `operation_frame`. The FR-115-AC-6 tests (`frame.rs:970`, `frame.rs:1013`) cover only the `Sub` and two-package cases, so the state-machine clause has no test. Fix: add a `#[trace("TC-514", "FR-115-AC-6")]` case that resolves `OrderLifecycle::place` to the state machine's `DeclarationKey` and selects its frame. | qsl-replay/src/spine/clause/tests/frame.rs:965-1007 |
| FND-003 | low | AC-27 and TC-1162 require each refusal to map to `Declined` with `InvalidInput` and its code. The test matches only `TerminalValue::Declined { .. }`, so a wrong cause or a wrong `DeclineCode` would pass. Fix: assert the cause and `DeclineCode::Qsl(refusal.code())`. | qsl-replay/src/spine/clause/tests/call_site_owners.rs:494-500,522-525 |
| FND-004 | low | `assert_generated_frame` checks only that `site.frame` is one of the graph's `frame` nodes. The unit also holds `VersionUnchanged`'s `ConfigVersion` frame, so the test passes if the wrong operation's frame is returned. Fix: assert that the frame belongs to the selected operation, for example that it is not the `ConfigVersion::attemptUpdate` frame, or that it is the frame node whose operation anchor is `site.anchor`. | qsl-replay/src/spine/clause/tests/call_site_owners.rs:349-371 |
| FND-005 | low | `names_non_object_declaration` no longer flags processes or state machines, so `replay_frame` (`execute/frame.rs:382`) now resolves a process or state-machine `frame_operation`. FR-116 Inputs allow this ("the object type, process or state machine that declares the operation"), so the change conforms to the spec. FR-116 has no AC for it and no test covers it. Fix: add a TC-515 case that replays a frame counterexample for `Config::Flow::go`, or record the gap on FR-116. | qsl-replay/src/execute/frame.rs:382-398 |

## Verdict

**Not mergeable** as a set with SR-12364. On this side, FND-001 and FND-002
are untested branches of ACs this PR implements. FND-003 to FND-005 are
weak assertions or coverage gaps. The `call_site` ACs that are tested are
tested well: each test fails on the wrong implementations listed above.

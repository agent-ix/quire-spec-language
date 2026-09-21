---
id: TM-001
title: "Rust fixture audit test matrix"
type: TestMatrix
---

## Overview

Scoped to FR-012/NFR-005 and IT-004. Status: locally verified after
implementation. This matrix does not assert coverage of the earlier
compiler/evaluator scope. US-004/StR-001 are the driving lineage; their full
operational validation remains outside this audit-only plan.

## Requirements Traceability

### Functional Requirement Coverage

| Functional Req | Acceptance Criteria | Test Cases | Coverage Status |
| --- | --- | --- | --- |
| FR-012 | FR-012-AC-1 | TC-001 | ✅ Passed locally |
| FR-012 | FR-012-AC-2 | TC-002 | ✅ Passed locally |
| FR-012 | FR-012-AC-3 | TC-003 | ✅ Passed locally |
| FR-012 | FR-012-AC-5 | TC-005 | ✅ Passed locally |
| FR-012 | FR-012-AC-6 | TC-006 | ✅ Passed locally |
| FR-012 | FR-012-AC-7 | TC-007 | ✅ Passed locally |
| FR-012 | FR-012-AC-8 | TC-005 | ✅ Passed locally |
| FR-012 | FR-012-AC-9 | TC-008 | ✅ Passed locally |
| FR-012 | FR-012-AC-10 | TC-009 | ✅ Passed locally |

## Test Case Summary

| Test ID | Title | Type | Priority | Traces To | Status |
| --- | --- | --- | --- | --- | --- |
| TC-001 | Self-contained identity negative controls | Unit | P1 | FR-012-AC-1 | ✅ Passed locally |
| TC-002 | Selected invocation packet audit | Integration | P1 | FR-012-AC-2 | ✅ Passed locally |
| TC-003 | Selected role and source correspondence audit | Integration | P1 | FR-012-AC-3 | ✅ Passed locally |
| TC-005 | Real native rule syntax outcomes | Integration | P1 | FR-012-AC-5, FR-012-AC-8 | ✅ Passed locally |
| TC-006 | Malformed JSON and field types | Property | P1 | FR-012-AC-6 | ✅ Passed locally |
| TC-007 | Fixture root containment | Property | P1 | FR-012-AC-7 | ✅ Passed locally |
| TC-008 | Audit resource ceilings | Property | P1 | FR-012-AC-9 | ✅ Passed locally |
| TC-009 | CLI encoding and Rust-only execution | E2E | P1 | FR-012-AC-10, NFR-005-M-2 | ✅ Passed locally |
| TC-010 | Owned verification language inventory | Manual | P1 | NFR-005-M-1 | ✅ Inspected locally |
| TC-193 | Candidate set matches registered backends advertising the requested kind | Unit | P1 | FR-075-AC-1, FR-075-AC-5 | 🚧 Planned; #185 |
| TC-194 | Registry candidate sets are invariant under registration-order permutation | Property | P1 | FR-075-AC-2, FR-080-AC-1 | 🚧 Planned; #185 |
| TC-195 | An unregistered named backend yields a distinct unknown-backend marker | Unit | P1 | FR-075-AC-3 | 🚧 Planned; #185 |
| TC-196 | A duplicate backend identity registration is refused and the original stands | Unit | P1 | FR-075-AC-4 | 🚧 Planned; #185 |
| TC-197 | Empty candidate set carries the data an unsupported warning needs | Unit | P1 | FR-076-AC-1, FR-076-AC-2 | 🚧 Planned; #185 |
| TC-198 | Backend absence never settles as a refusal or a hold at the registry | Unit | P1 | FR-076-AC-3 | 🚧 Planned; #185 |
| TC-199 | requests::report takes no Backend parameter and has no capability/family disposition | Unit | P1 | FR-077-AC-1, FR-077-AC-2 | 🚧 Planned; #185 |
| TC-200 | Requests are still recorded as data after negotiation removal | Unit | P1 | FR-077-AC-3 | 🚧 Planned; #185 |
| TC-201 | value::ieee and value::division carry no negotiate_* function | Unit | P1 | FR-078-AC-1, FR-078-AC-2 | 🚧 Planned; #213 S-1 |
| TC-202 | value::ieee and value::division evaluation is unchanged by negotiate_* removal | Unit | P1 | FR-078-AC-3 | 🚧 Planned; #213 S-1 |
| TC-203 | Existing Kani lowering corpus output is byte-identical before and after the registry swap | Integration | P1 | FR-079-AC-1 | 🚧 Planned; #185 |
| TC-204 | The three legacy lowering target names resolve identically through the registry | Unit | P1 | FR-079-AC-2 | 🚧 Planned; #185 |
| TC-205 | cargo-deny denies inventory, linkme and ctor | Integration | P1 | FR-080-AC-2 | 🚧 Planned; #185 |
| TC-206 | The registry module lint gate finds no static, OnceLock or thread_local | Integration | P1 | FR-080-AC-3 | 🚧 Planned; #185 |
| TC-207 | One unit test exists and passes per ADR-012 §5.2 row | Unit | P1 | FR-080-AC-4 | 🚧 Planned; #185 |
| TC-208 | The S7 seam probe fails to compile the registry arm on an unhandled capability-kind variant | Integration | P1 | FR-080-AC-5 | 🚧 Planned; #185 |
| TC-177 | The proof-result envelope maps every FR-331 outcome to its exact O-16 category | Property | P1 | FR-069-AC-1 | 🚧 Planned; #231 |
| TC-178 | The proof-result reader refuses an unknown version, vocabulary, or oversized envelope before consumption | Unit | P1 | FR-069-AC-2, FR-069-AC-4 | 🚧 Planned; #231 |
| TC-179 | A positive proof-result envelope round-trips its backend identity, tool pin and dispositions exactly | Unit | P1 | FR-069-AC-3 | 🚧 Planned; #231 |
| TC-180 | The witness envelope stores the transcript once and derives every other fact from it | Unit | P1 | FR-070-AC-1 | 🚧 Planned; #231 |
| TC-181 | The witness envelope refuses a malformed transcript, an out-of-domain digest, or an oversized encoding | Property | P1 | FR-070-AC-2, FR-070-AC-6, FR-070-AC-7 | 🚧 Planned; #231 |
| TC-182 | A positive witness envelope round-trips its transcript and every O-25 member exactly | Unit | P1 | FR-070-AC-3 | 🚧 Planned; #231 |
| TC-183 | The witness envelope refuses reconstruction when any one O-25 member is missing | Property | P1 | FR-070-AC-4 | 🚧 Planned; #231 |
| TC-184 | A family adds a typed witness payload through a typed extension point, not an untyped map | Unit | P1 | FR-070-AC-5 | 🚧 Planned; #231 |
| TC-185 | The replay request carries exactly the O-26 members and round-trips them exactly | Unit | P1 | FR-071-AC-1 | 🚧 Planned; #231 |
| TC-186 | The replay request's byte provision is reachable only by digest, never by path, is complete, and stays within the size bound | Property | P1 | FR-071-AC-2, FR-071-AC-5, FR-071-AC-6, FR-071-AC-7 | 🚧 Planned; #231 |
| TC-187 | The replay request's function selection accepts only a typed QualifiedName, never a bare string | Unit | P1 | FR-071-AC-3 | 🚧 Planned; #231 |
| TC-188 | The replay request refuses an unknown version or an out-of-set profile/capability identifier before recompilation | Unit | P1 | FR-071-AC-4 | 🚧 Planned; #231 |
| TC-189 | The replay result keeps the Witness arm and Input arm distinct, each with its own settlement | Unit | P1 | FR-072-AC-1 | 🚧 Planned; #231 |
| TC-190 | A replay disagreement settles inconclusive with a typed cause and is never repairable | Unit | P1 | FR-072-AC-2 | 🚧 Planned; #231 |
| TC-191 | A replay result's nested witness record round-trips exactly, compares without display-text interpretation, and refuses an oversized encoding | Unit | P1 | FR-072-AC-3, FR-072-AC-5 | 🚧 Planned; #231 |
| TC-192 | #217's function exemplar builds on the existing result/request/witness types with no new type | Integration | P1 | FR-072-AC-4 | 🚧 Planned; #231 |
| TC-209 | The witness envelope's Debug and Display rendering never reproduces the full transcript | Unit | P1 | FR-073-AC-1 | 🚧 Planned; #231 |
| TC-210 | The replay request's Debug and Display rendering never reproduces a byte-provision entry's raw bytes | Unit | P1 | FR-073-AC-2 | 🚧 Planned; #231 |
| TC-211 | A refusal cause from any of the four envelopes renders with no unredacted transcript, byte or value content, while the typed accessor stays fully readable | Unit | P1 | FR-073-AC-3 | 🚧 Planned; #231 |

## Six coverage rules

All nine FR-012 ACs and both NFR-005 metric obligations are verified above.
Modes are mutually exclusive, not combinable options. Valid/unknown modes,
missing/extra arguments, contained/foreign paths, zero/exact/over resource
limits, malformed fields and identity changes are covered. No asynchronous
state machine is introduced; the identity registry's first binding/repeated
binding/conflicting binding transitions have explicit controls. The generated
input rows require actual generation and retained failures, not a Property
label on one example. The implemented Property cases enumerate bounded JSON,
field, path and budget families deterministically; no randomized campaign or
fuzzing result is claimed. Independent fixture mutations reach real audit code.

## Integration Test Matrix

IT-004 is a command/file boundary, not a service/browser/event/database system.
TC-002, TC-003 and TC-005 execute real filesystem/parser integration, and TC-009
executes the compiled binary. The private standard packet remains an explicit
local lane; the default local suite uses self-contained negative controls. No substitute network/daemon classification is invented.

## Coverage gaps

The preexisting 21 Rust tests and older FR/NFR obligations still need their own
formal TC/evidence remediation. TC-010's inspection is recorded in
[the remediation inventory](../docs/rust-verification-remediation.md); the module
classifies Manual as no_source_symbol. Hosted CI is manual-dispatch only.

## Execution record

The audit unit and default audit integration tests pass locally. All three named
private-packet tests were explicitly executed against specification revision
36293bae7f5bcb7ca3b2389ed166e525dc9dba87 and passed. Quire resolves every audit
test symbol, every FR-012 AC and every executable TC. TC-010 has manual evidence. These counts establish
the selected scope; they are not full compiler or semantic qualification.

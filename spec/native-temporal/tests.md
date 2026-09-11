---
id: TM-008
title: "Native temporal evaluation matrix"
type: TestMatrix
---

## Overview

Scoped to [FR-043](../functional/FR-043-evaluate-bounded-native-temporal.md),
[FR-044](../functional/FR-044-activate-temporal-obligations.md) and
[NFR-008](../non-functional/NFR-008-bound-temporal-evaluation.md), which specify
the compiler's evaluation of the temporal body emitted by
[FR-042](../functional/FR-042-publish-compiled-protocol-artifacts.md).

Parsing, linking, type admission and protocol-artifact emission of temporal
declarations were delivered earlier and are covered by TM-002, TM-005 and TM-007;
this matrix adds no claim about them. Native-to-TL lowering remains outside the
matrix: the shared bridge requirement `ix://agent-ix/quire-specification/FR-095`
depends on `quire-contract-ir#63`/`#64` and on actual TL capability, so the only
obligation carried here is FR-044-AC-7's refusal with a retained native subject.

Status values are set from local runs only; hosted workflows remain
manual-dispatch. `Coverage Status` and `Status` headers follow
[matrix-status.md](../../docs/matrix-status.md) and are not renamed.

## Requirements Traceability

### Functional Requirement Coverage

| Functional Req | Acceptance Criteria | Test Cases | Coverage Status |
| --- | --- | --- | --- |
| FR-043 | FR-043-AC-1 | TC-122 | 🚧 Planned |
| FR-043 | FR-043-AC-2 | TC-122 | 🚧 Planned |
| FR-043 | FR-043-AC-3 | TC-122 | 🚧 Planned |
| FR-043 | FR-043-AC-4 | TC-122 | 🚧 Planned |
| FR-043 | FR-043-AC-5 | TC-122 | 🚧 Planned |
| FR-043 | FR-043-AC-6 | TC-122 | 🚧 Planned |
| FR-043 | FR-043-AC-7 | TC-122 | 🚧 Planned |
| FR-043 | FR-043-AC-8 | TC-122 | 🚧 Planned |
| FR-044 | FR-044-AC-1 | TC-123 | 🚧 Planned |
| FR-044 | FR-044-AC-2 | TC-123 | 🚧 Planned |
| FR-044 | FR-044-AC-3 | TC-123 | 🚧 Planned |
| FR-044 | FR-044-AC-4 | TC-123 | 🚧 Planned |
| FR-044 | FR-044-AC-5 | TC-123 | 🚧 Planned |
| FR-044 | FR-044-AC-6 | TC-123 | 🚧 Planned |
| FR-044 | FR-044-AC-7 | TC-123 | 🚧 Planned |
| NFR-008 | NFR-008-AC-1 | TC-124 | 🚧 Planned |
| NFR-008 | NFR-008-AC-2 | TC-124 | 🚧 Planned |
| NFR-008 | NFR-008-AC-3 | TC-124 | 🚧 Planned |
| NFR-008 | NFR-008-AC-4 | TC-124 | 🚧 Planned |
| NFR-008 | NFR-008-AC-5 | TC-124 | 🚧 Planned |

## Test Case Summary

| Test ID | Title | Type | Priority | Traces To | Status |
| --- | --- | --- | --- | --- | --- |
| TC-122 | Bounded temporal truth under each selected profile | Integration | P1 | FR-043 | 🚧 Planned |
| TC-123 | Activation dispositions and immutable captures | Integration | P1 | FR-044 | 🚧 Planned |
| TC-124 | Checked bounds, exhaustion and retained state | Property | P1 | NFR-008 | 🚧 Planned |

## Six coverage rules

All twenty acceptance criteria across FR-043, FR-044 and NFR-008 appear above
exactly once, each bound to one test case so a single trace attribute carries a
single TC identity.

Modes are mutually exclusive selections, not combinable options: the three
temporal profiles are three source selections, and a trace carries exactly one.
Valid and invalid profile discriminants, matching and mismatched clock bindings,
closed-complete and open decision scopes, present and absent valuations, admitted
and absent order keys, and zero, exact, insufficient and clamped ceilings each
have explicit controls.

The state machine under test is activation: its inactive, unknown-open,
unknown-incomplete, unknown-refused and active transitions each have a control,
as do first delivery, repeated delivery and distinct-trigger delivery.

`true`, `false`, `pending`, incomplete and refused are asserted as five distinct
outcomes; no control accepts a Boolean produced by an exhaustion, overflow,
eviction or unsupported-mapping path.

TC-124's property groups enumerate bounded formula, interval and instance
families deterministically from the published charging contract. No randomized
campaign, fuzzing result or mutation-adequacy score is claimed; mutation testing
of the exhaustion paths remains outstanding assurance work on the owning ticket.

Independent fixture mutations reach real evaluator code through the public Rust
API. No control asserts against the evaluator's own reported usage as its
expected value.

---
id: TM-008
title: "Native temporal evaluation matrix"
type: TestMatrix
---

## Overview

Scoped to [FR-043](../functional/FR-043-evaluate-bounded-native-temporal.md),
[FR-044](../functional/FR-044-activate-temporal-obligations.md),
[FR-045](../functional/FR-045-classify-temporal-mapping-support.md) and
[NFR-008](../non-functional/NFR-008-bound-temporal-evaluation.md), which specify
the compiler's evaluation of the temporal body emitted by
[FR-042](../functional/FR-042-publish-compiled-protocol-artifacts.md).

FR-051-AC-6 and TC-139 extend this matrix with the production dependency
graph's cycle-freedom, required by epic `tl-syntax#52`'s Contract Model
substrate. FR-051-AC-1 through FR-051-AC-5 (the static checked-predicate and
temporal-subject handoff boundary this matrix previously described under
those ids) are retired by M-6d, which deletes the handoff modules
and their tests, formerly in `tests/it/compiled_protocol_v2.rs`. FR-052/FR-053 and
TC-140/TC-141 (the canonical formula-wide native temporal request/result
owner boundary required by Contract IR FR-026) are retired by the same
change, which deletes `protocol_artifact::native_temporal` and
`tests/it/native_temporal_owner.rs`: with the spine `ProtocolClause` frame
slices landed, the spine `ProtocolClause` path no longer needs either producer/
consumer round trip.

Parsing, linking, type admission and protocol-artifact emission of temporal
declarations were delivered earlier and are covered by TM-002, TM-005 and TM-007;
this matrix adds no claim about them. The emission half of the native-to-TL
bridge remains outside the matrix: it depends on `quire-contract-ir#63`,
`quire-contract-ir#64` and actual TL capability. FR-045 covers only the support
classification, which is decidable from the admitted declaration plus the
requested surrounding-execution closure, with no bridge and no backend report.
The owner ruling on `quire-contract-ir#64` fixes surrounding-execution closure as
the TL row selector. Through a strict
[FR-050](../functional/FR-050-publish-authenticated-temporal-artifacts.md) `/2`
package the classification also retains the definition selection it
authenticated (FR-045-AC-6); authenticated clock parameters and trace premises
are FR-050's claim under TC-138, not this matrix's.

Pointwise Boolean composition, guard/redelivery dispositions and settled-sibling
behavior are ratified shared semantics from `quire-specification` PR #23.

## Requirements Traceability

### Functional Requirement Coverage

| Functional Req | Acceptance Criteria | Test Cases | Status |
| --- | --- | --- | --- |
| FR-043 | FR-043-AC-1 | TC-122 |  |
| FR-043 | FR-043-AC-2 | TC-122 |  |
| FR-043 | FR-043-AC-3 | TC-122 |  |
| FR-043 | FR-043-AC-4 | TC-122 |  |
| FR-043 | FR-043-AC-5 | TC-122 |  |
| FR-043 | FR-043-AC-6 | TC-122 |  |
| FR-043 | FR-043-AC-7 | TC-122 |  |
| FR-043 | FR-043-AC-8 | TC-122 |  |
| FR-043 | FR-043-AC-9 | TC-122 |  |
| FR-043 | FR-043-AC-10 | TC-122 |  |
| FR-043 | FR-043-AC-11 | TC-122 |  |
| FR-043 | FR-043-AC-12 | TC-122 |  |
| FR-043 | FR-043-AC-13 | TC-122 |  |
| FR-044 | FR-044-AC-1 | TC-123 |  |
| FR-044 | FR-044-AC-2 | TC-123 |  |
| FR-044 | FR-044-AC-3 | TC-123 |  |
| FR-044 | FR-044-AC-4 | TC-123 |  |
| FR-044 | FR-044-AC-5 | TC-123 |  |
| FR-044 | FR-044-AC-6 | TC-123 |  |
| FR-044 | FR-044-AC-7 | TC-123 |  |
| FR-044 | FR-044-AC-8 | TC-123 |  |
| FR-044 | FR-044-AC-9 | TC-123 |  |
| FR-045 | FR-045-AC-1 | TC-125 |  |
| FR-045 | FR-045-AC-2 | TC-125 |  |
| FR-045 | FR-045-AC-3 | TC-125 |  |
| FR-045 | FR-045-AC-4 | TC-125 |  |
| FR-045 | FR-045-AC-5 | TC-125 |  |
| FR-045 | FR-045-AC-6 | TC-125 |  |
| FR-051 | FR-051-AC-1, FR-051-AC-2, FR-051-AC-3, FR-051-AC-4, FR-051-AC-5 (retired) | TC-139 |  |
| FR-051 | FR-051-AC-6 | TC-139 |  |
| FR-052 | FR-052-AC-1 through FR-052-AC-8 (retired) | TC-140 |  |
| FR-053 | FR-053-AC-1 through FR-053-AC-6 (retired) | TC-141 |  |
| NFR-008 | NFR-008-AC-1 | TC-124 |  |
| NFR-008 | NFR-008-AC-2 | TC-124 |  |
| NFR-008 | NFR-008-AC-3 | TC-124 |  |
| NFR-008 | NFR-008-AC-4 | TC-124 |  |
| NFR-008 | NFR-008-AC-5 | TC-124 |  |

### Non-Functional Requirement Coverage

| Non-Functional Req | Verification Method | Evidence/Test Cases | Status |
| --- | --- | --- | --- |
| NFR-008 | property-based-testing over the declared nested-horizon population; fault-injection for each forced stop; model-based-test-generation for eviction schedules; integration-testing for result reuse; negative-abuse-testing for ceiling permutations | TC-124 groups 1–5; counters published in docs/native-temporal-evaluation.md |  |
| NFR-005 | Inspection and the existing Rust gates | Rust-only evaluator, tests and fixtures; no shell or foreign-language executable path added |  |
| NFR-003 | Inspection and Test | Five distinct outcomes and five distinct settlement bases asserted in TC-122; no resource stop rendered as Boolean in TC-124 |  |

### Stakeholder and User Story Coverage

StR-001 → US-003 → FR-043/FR-044 → TC-122/123/124, and US-004 → FR-045 → TC-125.
US-003-EX-1 is illustrated by TC-122 groups 1 and 7; US-003-EX-2 by TC-124
groups 2 and 3. Illustrative EX IDs are not minted as acceptance criteria.

## Test Case Summary

| Test ID | Title | Type | Priority | Traces To | Status |
| --- | --- | --- | --- | --- | --- |
| TC-122 | Bounded temporal truth under each selected profile | Integration | P1 | FR-043 | ✅ |
| TC-123 | Activation dispositions and immutable captures | Integration | P1 | FR-044 | ✅ |
| TC-124 | Checked bounds, exhaustion and retained state | Property | P1 | NFR-008 | ✅ |
| TC-125 | Native-to-TL mapping support classification | Unit | P1 | FR-045 | ✅ |
| TC-139 | Publish and read checked native handoffs | Integration | P0 | FR-051 | ✅ |
| TC-140 | Canonical native temporal request/result owner boundary | Integration | P0 | FR-052 | ⛔ |
| TC-141 | Preserve opaque semantic-trigger identity through native temporal v2 | Integration | P0 | FR-053 | ⛔ |

## Six coverage rules

All fifty-three acceptance criteria across FR-043, FR-044, FR-045, FR-051,
FR-052, FR-053 and NFR-008 appear above exactly once, each bound to one test case so a
test's trace attributes carry a single TC identity. Of those, FR-051-AC-1
through FR-051-AC-5, all of FR-052 and all of FR-053 (nineteen criteria) are
retired (M-6d); the rule that each appears exactly once, bound to
one test case, continues to hold for a retired criterion's row.

Modes are mutually exclusive selections, not combinable options: the three
temporal profiles are three source selections and a declaration carries one, so
TC-122 compiles one declaration per profile from identical clause text rather
than reusing one declaration. All eight bounded operators run under all three
profiles. Valid and invalid profile identities and revisions, matching and
mismatched clock binding names, closed-complete and open decision scopes, present
and absent valuations, fully, partially and foreign-keyed admitted orders, an
authoritative origin against a bare history cutoff, and zero, exact, one-short
and raised-above-default limits each have explicit controls.

Two state machines are under test. Activation transitions through inactive,
unknown-open, unknown-incomplete, unknown-refused and active, with a guard-false
trigger controlled in both a closed-complete and an open scope, and with
first delivery, repeated delivery, conflicting redelivery, distinct-trigger
delivery and whole-execution origin each controlled. Settlement transitions
through `closed-scope`, `decisive-witness`, `decisive-counterexample`,
`unsettled` and `unavailable`, with every one-axis substitution refused.

`true`, `false`, `pending`, incomplete and refused are asserted as five distinct
outcomes; no control accepts a Boolean produced by an exhaustion, overflow,
eviction, contradiction or unsupported-mapping path, and no control accepts false
extension or empty-window truth over an incomplete input.

TC-124 groups 1 and 5 enumerate bounded formula, interval and ceiling families
deterministically over declared finite populations, and group 3 generates
obligation and eviction schedules from a declared finite model. No randomized
campaign, fuzzing result or mutation-adequacy score is claimed.

Independent fixture mutations reach real evaluator code through the public Rust
API. No control asserts against the evaluator's own reported usage as its
expected value; expected charges come from the published counter definitions.
Criteria whose obligation is a universal negative or a type-level guarantee carry
Inspection, Analysis or Demonstration alongside their test reference rather than
claiming a test proves the negative.

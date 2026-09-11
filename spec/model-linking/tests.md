---
id: TM-003
title: "Native model-linking and static-typing matrix"
type: TestMatrix
---

## Overview

LC02 verification after the owner's internal adoption of specification PR8 at
e897f81. Ten linking cases, TC-020–024 and TC-030–034, now execute through the
public formal linker API. Their eleven FR-005/013 criteria are backed by the
Rust tests in tests/linking.rs. The five FR-006 typing cases now execute through
the native checker. Model/checker qualification is complete; executable projection
and the full workflow remain open. TM-001/002 retain their existing native/audit evidence.

## Requirements Traceability

### Functional Requirement Coverage

| Functional Req | Acceptance Criteria | Test Cases | Coverage Status |
| --- | --- | --- | --- |
| FR-005 | FR-005-AC-1 | TC-020 | ✅ Passed |
| FR-005 | FR-005-AC-2 | TC-021 | ✅ Passed |
| FR-005 | FR-005-AC-3 | TC-022 | ✅ Passed |
| FR-005 | FR-005-AC-4 | TC-023 | ✅ Passed |
| FR-005 | FR-005-AC-5 | TC-024 | ✅ Passed |
| FR-006 | FR-006-AC-1 | TC-025 | ✅ Passed |
| FR-006 | FR-006-AC-2 | TC-026 | ✅ Passed |
| FR-006 | FR-006-AC-3 | TC-027 | ✅ Passed |
| FR-006 | FR-006-AC-4 | TC-028 | ✅ Passed |
| FR-006 | FR-006-AC-5 | TC-029 | ✅ Passed |
| FR-013 | FR-013-AC-1 | TC-030 | ✅ Passed |
| FR-013 | FR-013-AC-2 | TC-030 | ✅ Passed |
| FR-013 | FR-013-AC-3 | TC-031 | ✅ Passed |
| FR-013 | FR-013-AC-4 | TC-032 | ✅ Passed |
| FR-013 | FR-013-AC-5 | TC-033 | ✅ Passed |
| FR-013 | FR-013-AC-6 | TC-034 | ✅ Passed |
| FR-014 | FR-014-AC-1 | TC-035 | ✅ Passed |
| FR-014 | FR-014-AC-2 | TC-036 | ✅ Passed |
| FR-014 | FR-014-AC-3 | TC-037 | ✅ Passed |
| FR-014 | FR-014-AC-4 | TC-038 | ✅ Passed |
| FR-014 | FR-014-AC-5 | TC-039 | ✅ Passed |
| FR-015 | FR-015-AC-1 | TC-040 | ✅ Passed |
| FR-015 | FR-015-AC-2 | TC-041 | ✅ Passed |
| FR-015 | FR-015-AC-3 | TC-042 | ✅ Passed |
| FR-015 | FR-015-AC-4 | TC-043 | ✅ Passed |
| FR-015 | FR-015-AC-5 | TC-044 | ✅ Passed |
| FR-015 | FR-015-AC-6 | TC-045 | ✅ Passed |
| FR-016 | FR-016-AC-1 | TC-025, TC-053 | ✅ Passed |
| FR-016 | FR-016-AC-2 | TC-026, TC-046 | ✅ Passed |
| FR-016 | FR-016-AC-3 | TC-027, TC-047 | ✅ Passed |
| FR-016 | FR-016-AC-4 | TC-028, TC-048 | ✅ Passed |
| FR-016 | FR-016-AC-5 | TC-029, TC-048 | ✅ Passed |
| FR-016 | FR-016-AC-6 | TC-049 | ✅ Passed |
| FR-016 | FR-016-AC-7 | TC-050, TC-053 | ✅ Passed |
| FR-016 | FR-016-AC-8 | TC-051 | ✅ Passed |
| FR-016 | FR-016-AC-9 | TC-052 | ✅ Passed |
| FR-017 | FR-017-AC-1 | TC-054 | ✅ Passed |
| FR-017 | FR-017-AC-3 | TC-030, TC-031, TC-032, TC-033, TC-034 | ✅ Passed |
| FR-017 | FR-017-AC-4 | TC-001, TC-002, TC-003, TC-004, TC-006 | ✅ Passed |

FR-017-AC-2 uses Inspection rather than a Test Case. SR-083 records the executed
structural ownership inspection and its PASS disposition; no test symbol is
invented for that criterion.

## Test Case Summary

| Test ID | Title | Type | Priority | Traces To | Status |
| --- | --- | --- | --- | --- | --- |
| TC-020 | Exact qualified import | Integration | P1 | FR-005-AC-1 | ✅ Passed |
| TC-021 | Missing selected import | Integration | P1 | FR-005-AC-2 | ✅ Passed |
| TC-022 | Ambiguous exported declaration | Integration | P1 | FR-005-AC-3 | ✅ Passed |
| TC-023 | Stale package closure | Integration | P1 | FR-005-AC-4 | ✅ Passed |
| TC-024 | Failed linkage is atomic | Property | P1 | FR-005-AC-5 | ✅ Passed |
| TC-025 | Unguarded optional unwrap | Integration | P1 | FR-006-AC-1, FR-016-AC-1 | ✅ Passed |
| TC-026 | Presence facts stay with their observation | Integration | P1 | FR-006-AC-2, FR-016-AC-2 | ✅ Passed |
| TC-027 | Guarded bounded addition | Integration | P1 | FR-006-AC-3, FR-016-AC-3 | ✅ Passed |
| TC-028 | Ambiguous scalar inference | Integration | P1 | FR-006-AC-4, FR-016-AC-4 | ✅ Passed |
| TC-029 | Clause roots are Boolean | Integration | P1 | FR-006-AC-5, FR-016-AC-5 | ✅ Passed |
| TC-030 | Exact source and formal artifact binding | Integration | P1 | FR-013-AC-1, FR-013-AC-2 | ✅ Passed |
| TC-031 | Lexical and formal declaration occurrences | Integration | P1 | FR-013-AC-3 | ✅ Passed |
| TC-032 | Unmapped reference and operation forms | Integration | P1 | FR-013-AC-4 | ✅ Passed |
| TC-033 | Native linking resource ceilings | Property | P1 | FR-013-AC-5 | ✅ Passed |
| TC-034 | Ambiguity provenance and atomicity | Property | P1 | FR-013-AC-6 | ✅ Passed |
| TC-035 | Explicit source identity assignment | Integration | P1 | FR-014-AC-1 | ✅ Passed |
| TC-036 | Independent formal coordinate examples | Integration | P1 | FR-014-AC-2 | ✅ Passed |
| TC-037 | Foreign native source requests | Integration | P1 | FR-014-AC-3 | ✅ Passed |
| TC-038 | Inconsistent formal coordinates | Integration | P1 | FR-014-AC-4 | ✅ Passed |
| TC-039 | Bounded generated span correspondence | Property | P1 | FR-014-AC-5 | ✅ Passed |
| TC-040 | Qualify the source-derived native rule model | Integration | P1 | FR-015-AC-1 | ✅ Passed |
| TC-041 | Refuse missing or inconsistent native model roles | Integration | P1 | FR-015-AC-2 | ✅ Passed |
| TC-042 | Bind all native model semantics and provenance | Property | P1 | FR-015-AC-3 | ✅ Passed |
| TC-043 | Verify model loci and inventory identity consistency | Integration | P1 | FR-015-AC-4 | ✅ Passed |
| TC-044 | Resolve explicit references and operation declarations | Integration | P1 | FR-015-AC-5 | ✅ Passed |
| TC-045 | Bound model construction and native linkage | Property | P1 | FR-015-AC-6 | ✅ Passed |
| TC-046 | Check observation and operation value availability | Integration | P1 | FR-016-AC-2 | ✅ Passed |
| TC-047 | Prove signed arithmetic through the actual IR API | Integration | P1 | FR-016-AC-3 | ✅ Passed |
| TC-048 | Solve exact native contextual types | Integration | P1 | FR-016-AC-4, FR-016-AC-5 | ✅ Passed |
| TC-049 | Retain exact checked source and authored clause bindings | Integration | P1 | FR-016-AC-6 | ✅ Passed |
| TC-050 | Check lexical scope and guarded evaluation order | Integration | P1 | FR-016-AC-7 | ✅ Passed |
| TC-051 | Bound constraint checking and shared proof expansion | Property | P1 | FR-016-AC-8 | ✅ Passed |
| TC-052 | Retain population and invocation obligations after checking | Integration | P1 | FR-016-AC-9 | ✅ Passed |
| TC-053 | Independent guard-fact truth-table soundness | Property | P1 | FR-016-AC-1, FR-016-AC-7 | ✅ Passed |
| TC-054 | Exact decoded JSON occurrence provenance | Integration | P1 | FR-017-AC-1 | ✅ Passed |
| TC-113 | Composed syntax and historical grammar | Integration | P1 | FR-035-AC-1..FR-035-AC-6 | ✅ Passed |
| TC-114 | Composed dependencies and declaration-owned roles | Integration | P1 | FR-036-AC-1..FR-036-AC-4, FR-036-AC-7 | 🚧 Planned |
| TC-115 | Static meaning and requested capabilities | Integration | P1 | FR-036-AC-5, FR-036-AC-6, FR-036-AC-8 | 🚧 Planned |
| TC-119 | Composed value types and guarded definedness | Integration | P1 | FR-040-AC-1..FR-040-AC-10 | 🚧 Planned |
| TC-120 | Explicit rational model profile and historical isolation | Integration | P1 | FR-041-AC-1..FR-041-AC-7 | 🚧 Planned |

## Composed language admission (L2)

TC-113 exercises the composed parser through thirteen public Rust tests in
`tests/composed_syntax.rs`; the historical corpus checks compatibility. TC-114's
source namespace and native dependency portions have public tests in
`tests/composed_namespace.rs` and `tests/composed_linking.rs`. The combined path in
`tests/composed_binding.rs` adds exact definition/rule and NativeModel export
selection, lexical/capture scope and protocol references, with dedicated adverse
tests in the corresponding modules. Full typing/runtime-role criteria, TC-115
and [IT-009](../integration/IT-009-composed-package-boundary.md) remain open under
compiler #35; the broad rows below therefore remain Planned. Names resolved at
this stage grant no checked or executable package. Status records local runs,
not engine-verified coverage.

TC-120's public rational-model controls are implemented and pass locally through
the real frontend, admission and composed exports. Its Planned matrix rows retain
the remaining review/assurance acceptance; they do not mean the producer is unbuilt.
TC-119 now has public type-admission controls across predicates, state, temporal
and protocol consumers, including partial upstream reports. Definedness, complete
runtime obligations and remaining acceptance controls stay open. Neither test set
establishes complete compiler #35/#40.

| Functional Req | Acceptance Criteria | Test Cases | Status |
| --- | --- | --- | --- |
| FR-035 | FR-035-AC-1 | TC-113 | ✅ Passed |
| FR-035 | FR-035-AC-2 | TC-113 | ✅ Passed |
| FR-035 | FR-035-AC-3 | TC-113 | ✅ Passed |
| FR-035 | FR-035-AC-4 | TC-113 | ✅ Passed |
| FR-035 | FR-035-AC-5 | TC-113 | ✅ Passed |
| FR-035 | FR-035-AC-6 | TC-113 | ✅ Passed |
| FR-036 | FR-036-AC-1 | TC-114 | 🚧 Planned |
| FR-036 | FR-036-AC-2 | TC-114 | 🚧 Planned |
| FR-036 | FR-036-AC-3 | TC-114 | 🚧 Planned |
| FR-036 | FR-036-AC-4 | TC-114 | 🚧 Planned |
| FR-036 | FR-036-AC-5 | TC-115 | 🚧 Planned |
| FR-036 | FR-036-AC-6 | TC-115 | 🚧 Planned |
| FR-036 | FR-036-AC-7 | TC-114 | 🚧 Planned |
| FR-036 | FR-036-AC-8 | TC-115 | 🚧 Planned |
| FR-040 | FR-040-AC-1 | TC-119 | 🚧 Planned |
| FR-040 | FR-040-AC-2 | TC-119 | 🚧 Planned |
| FR-040 | FR-040-AC-3 | TC-119 | 🚧 Planned |
| FR-040 | FR-040-AC-4 | TC-119 | 🚧 Planned |
| FR-040 | FR-040-AC-5 | TC-119 | 🚧 Planned |
| FR-040 | FR-040-AC-6 | TC-119 | 🚧 Planned |
| FR-040 | FR-040-AC-7 | TC-119 | 🚧 Planned |
| FR-040 | FR-040-AC-8 | TC-119 | 🚧 Planned |
| FR-040 | FR-040-AC-9 | TC-119 | 🚧 Planned |
| FR-040 | FR-040-AC-10 | TC-119 | 🚧 Planned |
| FR-041 | FR-041-AC-1 | TC-120 | 🚧 Planned |
| FR-041 | FR-041-AC-2 | TC-120 | 🚧 Planned |
| FR-041 | FR-041-AC-3 | TC-120 | 🚧 Planned |
| FR-041 | FR-041-AC-4 | TC-120 | 🚧 Planned |
| FR-041 | FR-041-AC-5 | TC-120 | 🚧 Planned |
| FR-041 | FR-041-AC-6 | TC-120 | 🚧 Planned |
| FR-041 | FR-041-AC-7 | TC-120 | 🚧 Planned |

## Six coverage rules

Every existing FR-005/006 AC has a case. Their historical scope has one selected native profile;
current/post operation contexts and matching/mismatching observation-qualified
guards are explicit case pairs. Numeric upper-bound equality and the strict
guard edge distinguish safe addition from possible overflow. Missing,
ambiguous, stale, undefined and ill-typed paths are named rather than collapsed
to generic failure. Binding permutations and prior successful calls test the
atomic-result boundary; no runtime state transition is claimed by static typing.

TC-024 uses a bounded generated input family and is Property. The other cases
use selected real adapter/checker integrations and expected judgments. Further
implementation-specific limits, version-feature combinations and adverse
adapter capabilities must be specified when that API exists; this matrix does
not claim complete coverage of a future interface it has not inspected.

## Preconditions and claim limits

The source of truth for preconditions is
[IT-005](../integration/IT-005-qualify-native-model-consumption.md).
The independently authored rule-model hypotheses require their own qualified
realization; the existing ConfigVersion model is not interchangeable with them.
Linker tests use imported single-line ix-trace-rs attributes and real APIs.
At the PR8 linker baseline, Quire reconciliation reported TM-003 10/15 backed,
FR-005 5/5 and FR-013 6/6, with no status lies or untracked symbols. The PR9
source bridge subsequently added five executed cases. The current matrix has
35 qualified cases, including all 13 checker cases. FR-006 has five executed
judgments and FR-016 has nine qualified criteria. The
known functional-table Status/Coverage Status classifier limitation is retained;
explicit TC statuses and executed logs supply the separate completion evidence.

The historical rows specify evidence for LC02. Native linking and static checking are
implemented; runtime validation/evaluation and qualified projection remain incomplete. The accepted IR ADR-0054
removes the earlier prerequisite for a shared Filament model adapter. The
generic lane uses the public formal declaration API; A owns concrete native
projection work for clauses that need additional semantic correspondence.
The full workflow remains IT-002 and the original Agent A assignment.

FR-013 now defines the concrete formal-environment resolution API and TC-030–034
cover its six criteria. TC-020–024 consume that same real API. Canonical byte
selection, explicit self binding, lexical scopes and resource budgets are
defined before code; FR-006's five static-judgment cases remain separate.

## Formal source bridge qualification

FR-014 adds five executed cases to the same LC02 matrix. TC-035–038 exercise
actual pinned IR constructors and independent adverse inputs; TC-039 generates
156 sources and enumerates all valid and invalid offset pairs with a separate
coordinate oracle. Every FR-014 criterion maps to one case. Boundaries include
empty input, EOF, CRLF interior, split scalars, the existing source-byte ceiling,
foreign labels/digests and misleading IR endpoint coordinates. Success after
failure checks immutable request behavior; there is no runtime state transition
or callback/concurrency option in this API. Loom and concurrency fault injection
do not apply to this immutable, single-request bridge. No fuzz result is claimed.
These tests do not discharge FR-006 or the full IT-005 model/checker integration.

## Native model and checker qualification

FR-015/016 define the actual native model-role and checker interfaces for the
unchanged TC-025–029 judgments. TC-040–045 qualify the real source-derived Rust
model producer, exact artifact/provenance and native link compatibility.
TC-046–052 cover additional observation, type-constraint, source/anchor, lexical,
proof-budget and runtime-input-obligation behavior. Every new criterion has
explicit tests; the five old typing cases keep their reference and operation
semantics. Their execution statuses advanced after the recorded real runs.

The six coverage rules include valid/adverse role dimensions, nominal/unit and
context permutations, zero/equal/one-over budget boundaries, exact and foreign
source/anchor bindings, immutable success-after-failure behavior, unreachable
branches and scope transitions. Runtime invocation/population transitions are
recorded as FR-007 input requirements; static checking cannot claim to execute
them. Bounded generated mutation/permutation and proof-expansion families cover
the property-shaped artifact and resource criteria. Native tests use the actual
IR checker and exact model producer; no mock bypass or abstract fixture setup
failure can count as application refusal. No concurrency or Loom claim is
needed for this serial immutable checker; no fuzz or whole-workflow proof is
inferred from its unit/integration/property suites.

TC-053 independently checks accepted presence proofs over a bounded generated
Boolean formula family and all assignments, including mandatory positive
controls. It addresses the native alternative-join fact calculation rather than
assuming the existing IR proof implementation qualifies that added logic.

## Construction repair qualification

FR-017 / Task-011 is complete at 08a4fe7. TC-054 has three executed tests for
original occurrences and strict/foreign refusals. Existing linker and audit
regressions carry FR-017-AC-3/4 attributes and retain their actual outcomes.
SR-083 supplies the separate FR-017-AC-2 ownership inspection. The default
suite passed 61 tests and the selected private lane passed three tests.
These results qualified the four scoped construction repairs. Subsequent
Task-008/009 evidence appears below; trace presence alone is not qualification.

## Native linkage qualification

TC-044 is qualified at 667bf07 by SR-084. Eight new Rust link tests exercise
the actual source-derived model and shared linker, including original field/
operation/enum/parameter targets, exact profile/digest selection, carrier access
refusals, conflicting inventory identities and hard native-link limits. The
default suite passed 69 tests and all three selected private tests passed.
At that revision, additional TC-042/043/045 link controls executed while broader
model criteria remained planned. SR-085 subsequently completes them below.
Neither successful linking nor a bound
test tag establishes checking or runtime qualification.

## Complete native model qualification

TC-040–045 are qualified at 0cd679c by SR-084/085. Seventeen additional Rust
tests cover every primitive site/wrapper, native-only role/carrier/operation
refusals, ordinary zero-bounded text, unused unsupported IR declarations, all
source-locus classes, seven source-derived semantic mutations and six inventory
permutations. Artifact payload assertions preserve exact signed i64 extrema,
changed bounds, unused values and ordered parameters. False line/column/source/
revision and split-scalar loci are constructor-valid before native refusal.

Exact small ModelLimits include every dimension and every aggregate-entry class.
Valid 10,000-node/10,000-entry and depth-64 models pass; the next required node,
entry or depth fails even with elevated options. The 10,001-role ceiling is
observed before artifact work; 10,000 full roles exceed this fixture's artifact
ceiling, so that is recorded as a coupled refusal rather than an exact success.
The earlier native-link tests retain exact 1 MiB and 8 MiB artifact boundaries.
The default suite passed 86 tests and all three selected private tests passed.
Task-008 is complete. Task-009's subsequent checker qualification follows.

## Native checker qualification

TC-025–029 and TC-046–053 execute in 24 Rust tests. Actual reference unwraps,
operation results, contextual nominal types and guarded arithmetic use the
qualified source-derived model and IR prover. Binding/source permutations,
lexical and observation controls, comparison eligibility and Unicode text maxima
have independent expected judgments. No setup failure is counted as a checker
refusal. Unreachable branches still reject name/type errors.

Each CheckLimits dimension succeeds at measured exact work and refuses one less
and zero across four generated alias families. A compact shared graph hits the
hard per-goal ceiling despite elevated caller options. Expanded depth exactly
64 succeeds and the next depth refuses before IR execution; accumulated goals
also exercise the independent total materialization budget. TC-053 enumerates
202 formulas and 808 independent assignments, with 62 admitted guards all sound
for presence and mandatory positive controls admitted.

TC-052 also retains populations reached through structural records, skipped
context fields and unused invocation parameters/results. A recorded failing
regression exposed the omitted nested population before the bounded traversal
fix. A native reference cycle terminates with the exact observation requirements.

The final default suite passes 110 tests; all three selected private audits pass.
Strict Clippy in both feature configurations, formatting, minimal build, rustdoc
and documented CLI/audit commands pass. Tasks 009/010 are complete with validated
SR-086/087 and the ready private PR #10 handoff. This does not
qualify runtime populations, truth, backend projection or Quire integration.

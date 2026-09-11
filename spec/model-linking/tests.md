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
the native checker. Model/checker qualification covers the earlier adopted definition; executable projection
and the full workflow remain open. TM-001/002 retain their existing native/audit evidence.

Task-034 extends TC-041 with the sequence-declaration ceiling and retains TC-065's
hard exhaustion checks using admitted nested sequences. SR-263 records actual
local execution. Source-profile reconciliation remains compiler #30.

The #28 status-column defect is a property of the installed catalog revision,
not of this document. The installed `spec-artifacts-process` module carries a
single `traceability.status.column: Status` while its TestMatrix archetype
asserts the `functional_coverage` columns as `Coverage Status`, so the two
spellings cannot both be satisfied: renaming the authored column to `Status`
makes `quire validate` fail the column assert, and keeping `Coverage Status`
makes `quire coverage` skip classification with `status-column-matches-nothing`.
The producer fix already exists. With the pinned stack recorded in
[docs/matrix-status.md](../../docs/matrix-status.md) — CLI `ff638b9`, engine
`d3bc2ba`, process module `e6ea515`, ISO module `a60ee12` — the
`functional-coverage` declaration carries its own `status_column: Coverage
Status`, no `status-column-matches-nothing` diagnostic is raised, and
`status_lies` is an empty list because the check ran rather than because it was
skipped. Do not author around the installed revision: keep `Coverage Status`,
and read status results from the pinned stack until those producer changes are
installed. Remaining work: #28 for that installed-stack adoption.

Two limits of a clean status result are load-bearing and are not visible in
`status_lies` or `unbacked_rows`. First, this declaration classifies a row by
the row's `Test Cases` reference, not per acceptance criterion: a row naming a
backed test case is classified `complete` whatever the criterion in its
`Acceptance Criteria` cell is worth. Per-criterion honesty therefore rests on
the minted acceptance-criterion targets, which `quire coverage` reports
separately, and the row status never carries it. Second, and for the same
reason, `FR-042-AC-10` names `TC-121`, `TC-121` binds through nine modules, and
so the row is backed and a completion label on it would not be reported as a
status lie; only the FR-042 group count, 9 of 10, exposes that the criterion
itself carries no tag. Check the per-group backed counts, not only the unbacked
rows, before moving any row off Planned.

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
| FR-042 | FR-042-AC-1 | TC-121 | ✅ Passed |
| FR-042 | FR-042-AC-2 | TC-121 | ✅ Passed |
| FR-042 | FR-042-AC-3 | TC-121 | ✅ Passed |
| FR-042 | FR-042-AC-4 | TC-121 | ✅ Passed |
| FR-042 | FR-042-AC-5 | TC-121 | ✅ Passed |
| FR-042 | FR-042-AC-6 | TC-121 | ✅ Passed |
| FR-042 | FR-042-AC-7 | TC-121 | ✅ Passed |
| FR-042 | FR-042-AC-8 | TC-121 | ✅ Passed |
| FR-042 | FR-042-AC-9 | TC-121 | ✅ Passed |
| FR-042 | FR-042-AC-10 | TC-121 | 🚧 Planned |

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
| TC-114 | Composed dependencies and declaration-owned roles | Integration | P1 | FR-036-AC-1..FR-036-AC-4, FR-036-AC-7 | ✅ Passed locally (tests/composed_models.rs, tests/composed_namespace.rs, tests/composed_definitions.rs, tests/composed_definition_source.rs, tests/composed_linking.rs, tests/composed_scopes.rs, tests/composed_binding.rs, src/linking/composed/arena.rs) |
| TC-115 | Static meaning and requested capabilities | Integration | P1 | FR-036-AC-5, FR-036-AC-6, FR-036-AC-8 | 🚧 Planned |
| TC-119 | Composed value types and guarded definedness | Integration | P1 | FR-040-AC-1..FR-040-AC-10 | ✅ Passed locally (tests/composed_types.rs, tests/composed_type_pipeline.rs, tests/composed_proofs.rs, tests/composed_query_proofs.rs, tests/native_query_emission.rs) |
| TC-120 | Explicit rational model profile and historical isolation | Integration | P1 | FR-041-AC-1..FR-041-AC-7 | ✅ Passed locally (tests/native_model_profiles.rs, src/checking/types.rs, src/linking.rs) |
| TC-117 | Exact numeric wire values and strict refusal | Integration | P1 | FR-038-AC-1..FR-038-AC-5 | ✅ Passed |
| TC-121 | Full compiled protocol artifact and Rust handoff requiring [B's IT-001](ix://agent-ix/quire-protocol/IT-001) | Integration | P1 | FR-042-AC-1..FR-042-AC-10 | 🚧 FR-042-AC-10 planned; FR-042-AC-1..FR-042-AC-9 passed locally (tests/protocol_artifact.rs, tests/native_protocol_emission.rs, tests/native_population_emission.rs, tests/native_choice_emission.rs, tests/native_compensation_emission.rs, tests/native_query_emission.rs, tests/native_domain_event_choices.rs, tests/native_domain_event_boundaries.rs, tests/native_mixed_observation_choices.rs) |

## Composed language admission (L2)

TC-113 exercises the composed parser through thirteen public Rust tests in
`tests/composed_syntax.rs`; the historical corpus checks compatibility. TC-114's
source namespace and native dependency portions have public tests in
`tests/composed_namespace.rs` and `tests/composed_linking.rs`. The combined path in
`tests/composed_binding.rs` adds exact definition/rule and NativeModel export
selection, lexical/capture scope and protocol references, with dedicated adverse
tests in the corresponding modules. `tests/composed_definitions.rs`,
`tests/composed_definition_source.rs`, `tests/composed_models.rs` and
`tests/composed_scopes.rs` complete TC-114's definition closure, model export and
declaration-owned role portions, with further tagged unit tests in
`src/linking/composed/arena.rs`. Each FR-036-AC-1..AC-4 and AC-7 row below cites
the module carrying its `#[trace]` tag, and `quire coverage` reports FR-036 5 of
8 backed — exactly those five. Full typing/runtime-role criteria, TC-115
and [IT-009](../integration/IT-009-composed-package-boundary.md) remain open under
compiler #35, so FR-036-AC-5, AC-6 and AC-8 stay Planned; no test in this tree
carries those tags. Their evidence is authored on the open, unmerged
[PR #67](https://github.com/agent-ix/quire-spec-language/pull/67); those three
rows and the TC-115 summary row move only once that work is on `main` and
`quire coverage` reports FR-036 8 of 8. An unmerged branch is not cited here as
backing. Names resolved at
this stage grant no checked or executable package. Status records local runs,
not review or assurance acceptance.

TC-120's public rational-model controls are implemented and pass locally through
the real frontend, admission and composed exports; all seven FR-041 criteria are
tagged in `tests/native_model_profiles.rs`, with two further tagged unit tests in
`src/checking/types.rs` and `src/linking.rs`, and `quire coverage` reports FR-041
7 of 7 backed. TC-119 has public type-admission controls across predicates,
state, temporal and protocol consumers, including partial upstream reports.
Supported guarded proofs and their authored correspondence are exercised in
`tests/composed_proofs.rs` through the actual IR prover, ordered-query proof
representation in `tests/composed_query_proofs.rs`, and declaration-owned runtime
requirements in `tests/composed_type_pipeline.rs`; FR-040 reports 10 of 10 backed.
The local status of these rows is a test-summary record, not review or assurance
acceptance; neither test set establishes complete compiler #35/#40.

| Functional Req | Acceptance Criteria | Test Cases | Status |
| --- | --- | --- | --- |
| FR-035 | FR-035-AC-1 | TC-113 | ✅ Passed |
| FR-035 | FR-035-AC-2 | TC-113 | ✅ Passed |
| FR-035 | FR-035-AC-3 | TC-113 | ✅ Passed |
| FR-035 | FR-035-AC-4 | TC-113 | ✅ Passed |
| FR-035 | FR-035-AC-5 | TC-113 | ✅ Passed |
| FR-035 | FR-035-AC-6 | TC-113 | ✅ Passed |
| FR-036 | FR-036-AC-1 | TC-114 | ✅ Passed locally (tests/composed_models.rs) |
| FR-036 | FR-036-AC-2 | TC-114 | ✅ Passed locally (tests/composed_namespace.rs) |
| FR-036 | FR-036-AC-3 | TC-114 | ✅ Passed locally (tests/composed_definitions.rs) |
| FR-036 | FR-036-AC-4 | TC-114 | ✅ Passed locally (tests/composed_scopes.rs) |
| FR-036 | FR-036-AC-5 | TC-115 | 🚧 Planned |
| FR-036 | FR-036-AC-6 | TC-115 | 🚧 Planned |
| FR-036 | FR-036-AC-7 | TC-114 | ✅ Passed locally (tests/composed_binding.rs) |
| FR-036 | FR-036-AC-8 | TC-115 | 🚧 Planned |
| FR-040 | FR-040-AC-1 | TC-119 | ✅ Passed locally (tests/composed_types.rs) |
| FR-040 | FR-040-AC-2 | TC-119 | ✅ Passed locally (tests/composed_types.rs) |
| FR-040 | FR-040-AC-3 | TC-119 | ✅ Passed locally (tests/composed_types.rs) |
| FR-040 | FR-040-AC-4 | TC-119 | ✅ Passed locally (tests/composed_proofs.rs) |
| FR-040 | FR-040-AC-5 | TC-119 | ✅ Passed locally (tests/composed_query_proofs.rs) |
| FR-040 | FR-040-AC-6 | TC-119 | ✅ Passed locally (tests/composed_proofs.rs) |
| FR-040 | FR-040-AC-7 | TC-119 | ✅ Passed locally (tests/composed_type_pipeline.rs) |
| FR-040 | FR-040-AC-8 | TC-119 | ✅ Passed locally (tests/composed_proofs.rs) |
| FR-040 | FR-040-AC-9 | TC-119 | ✅ Passed locally (tests/composed_types.rs) |
| FR-040 | FR-040-AC-10 | TC-119 | ✅ Passed locally (tests/composed_proofs.rs) |
| FR-041 | FR-041-AC-1 | TC-120 | ✅ Passed locally (tests/native_model_profiles.rs) |
| FR-041 | FR-041-AC-2 | TC-120 | ✅ Passed locally (tests/native_model_profiles.rs) |
| FR-041 | FR-041-AC-3 | TC-120 | ✅ Passed locally (tests/native_model_profiles.rs) |
| FR-041 | FR-041-AC-4 | TC-120 | ✅ Passed locally (tests/native_model_profiles.rs) |
| FR-041 | FR-041-AC-5 | TC-120 | ✅ Passed locally (tests/native_model_profiles.rs) |
| FR-041 | FR-041-AC-6 | TC-120 | ✅ Passed locally (tests/native_model_profiles.rs) |
| FR-041 | FR-041-AC-7 | TC-120 | ✅ Passed locally (tests/native_model_profiles.rs) |

## Compiled protocol numeric component

The separate numeric component of compiler #40 is tracked below; it does not
establish composed parsing, model admission or source-to-artifact correspondence.

| Functional Req | Acceptance Criteria | Test Cases | Status |
| --- | --- | --- | --- |
| FR-038 | FR-038-AC-1 | TC-117 | ✅ Passed |
| FR-038 | FR-038-AC-2 | TC-117 | ✅ Passed |
| FR-038 | FR-038-AC-3 | TC-117 | ✅ Passed |
| FR-038 | FR-038-AC-4 | TC-117 | ✅ Passed |
| FR-038 | FR-038-AC-5 | TC-117 | ✅ Passed |

TC-117 executes nine public Rust API tests in `tests/protocol_number.rs` with
`#[trace]` tags for these five criteria. This status records the local component
run, not a Quire-engine coverage or complete compiler-to-consumer claim.

## Compiled protocol artifact (L6)

[TC-121](../test-cases/TC-121-publish-compiled-protocol-artifacts.md) covers the
full [FR-042](../functional/FR-042-publish-compiled-protocol-artifacts.md) contract.
Its per-criterion rows are in the Functional Requirement Coverage table above
rather than in a second local table, so one row per criterion carries the
declaration the engine reconciles.

### Owning native Quire requirements

FR-042 owns the compiled-protocol wire contract, its canonical encoding and the
parser-free reader. It consumes, and does not restate,
[FR-035](../functional/FR-035-parse-composed-native-units.md) composed syntax,
[FR-036](../functional/FR-036-link-composed-native-packages.md) composed package
linking, [FR-040](../functional/FR-040-check-composed-values.md) typed values and
guarded definedness, [FR-041](../functional/FR-041-admit-rational-native-model-profile.md)
the rational model profile and [FR-038](../functional/FR-038-encode-exact-protocol-numbers.md)
the exact numeric codec. Native Quire source stays the sole editable formal
source; nothing in this section grants a second frontend or canonicalizer.

### Evidence backing each criterion

`tests/protocol_artifact.rs` exercises the reader/encoder with independently
supplied wire selections and actual admitted model exports.
`tests/native_protocol_emission.rs`, `tests/native_population_emission.rs`,
`tests/native_choice_emission.rs`, `tests/native_compensation_emission.rs`,
`tests/native_query_emission.rs`, `tests/native_domain_event_boundaries.rs`,
`tests/native_domain_event_choices.rs` and
`tests/native_mixed_observation_choices.rs` drive real source through native
family admission, emission and independent reading.
`tests/native_digest_domains.rs` proves the producer-owned source/model/config
digest domains stay separate from the compiled-artifact seal and from a
consumer's RFC 8785/JCS protocol-result identity: an exact producer reference
survives emission and reading, either substituted digest refuses, and a
recanonicalized producer reference refuses at producer admission.

FR-042-AC-1 through FR-042-AC-9 each carry `#[trace("TC-121", "FR-042-AC-n")]`
tags on executed Rust tests, and `quire coverage --scope . --json` reports each of
those minted acceptance-criterion targets as backed, FR-042 9 of 10; FR-042-AC-10
is reported unbacked. One module carrying each criterion's tag:

| Criterion | Module carrying the tag |
| --- | --- |
| FR-042-AC-1 | `tests/native_protocol_emission.rs` |
| FR-042-AC-2 | `tests/protocol_artifact.rs` |
| FR-042-AC-3 | `tests/protocol_artifact.rs` |
| FR-042-AC-4 | `tests/native_population_emission.rs` |
| FR-042-AC-5 | `tests/native_choice_emission.rs` |
| FR-042-AC-6 | `tests/native_compensation_emission.rs` |
| FR-042-AC-7 | `tests/protocol_artifact.rs` |
| FR-042-AC-8 | `tests/native_choice_emission.rs` |
| FR-042-AC-9 | `tests/native_domain_event_boundaries.rs` |
| FR-042-AC-10 | none; see below |

Per-criterion backing comes from those minted criterion targets and not from the
row status: the `functional-coverage` declaration classifies a row by its
`Test Cases` cell, so every FR-042 row above — AC-10 included — reads as backed
through `TC-121` and a completion label on AC-10 would not be reported as a
status lie. The FR-042 group count, 9 of 10, is the only place the gap shows.
This status records engine-verified trace binding plus a local serial test run;
it is not a claim of complete family semantics or of executed consumer
integration.

### Consumer handoff

FR-042-AC-10 stays planned. No test carries its tag, the engine reports it
unbacked, and the actual producer-to-consumer handoff has not been executed. The
downstream intake of the native compiler artifact is
[quire-protocol#11](https://github.com/agent-ix/quire-protocol/issues/11) under
epic [quire-protocol#14](https://github.com/agent-ix/quire-protocol/issues/14);
its integration caller owns
[IT-001](ix://agent-ix/quire-protocol/IT-001) on the consumer side. Remaining
work: FR-042-AC-10 and that handoff.

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

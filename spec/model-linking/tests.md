---
id: TM-003
title: "Native model-linking and static-typing matrix"
type: TestMatrix
---

## Overview

Ten linking cases, TC-020–024 and TC-030–034, execute through the public
formal linker API; the five FR-006 typing cases execute through the native
checker. Source-profile reconciliation remains compiler #30.

## Requirements Traceability

### Functional Requirement Coverage

| Functional Req | Acceptance Criteria | Test Cases | Status |
| --- | --- | --- | --- |
| FR-005 | FR-005-AC-1 | TC-020 |  |
| FR-005 | FR-005-AC-2 | TC-021 |  |
| FR-005 | FR-005-AC-3 | TC-022 |  |
| FR-005 | FR-005-AC-4 | TC-023 |  |
| FR-005 | FR-005-AC-5 | TC-024 |  |
| FR-006 | FR-006-AC-1 | TC-025 |  |
| FR-006 | FR-006-AC-2 | TC-026 |  |
| FR-006 | FR-006-AC-3 | TC-027 |  |
| FR-006 | FR-006-AC-4 | TC-028 |  |
| FR-006 | FR-006-AC-5 | TC-029 |  |
| FR-013 | FR-013-AC-1 | TC-030 |  |
| FR-013 | FR-013-AC-2 | TC-030 |  |
| FR-013 | FR-013-AC-3 | TC-031 |  |
| FR-013 | FR-013-AC-4 | TC-032 |  |
| FR-013 | FR-013-AC-5 | TC-033 |  |
| FR-013 | FR-013-AC-6 | TC-034 |  |
| FR-014 | FR-014-AC-1 | TC-035 |  |
| FR-014 | FR-014-AC-2 | TC-036 |  |
| FR-014 | FR-014-AC-3 | TC-037 |  |
| FR-014 | FR-014-AC-4 | TC-038 |  |
| FR-014 | FR-014-AC-5 | TC-039 |  |
| FR-015 | FR-015-AC-1 | TC-040 |  |
| FR-015 | FR-015-AC-2 | TC-041 |  |
| FR-015 | FR-015-AC-3 | TC-042 |  |
| FR-015 | FR-015-AC-4 | TC-043 |  |
| FR-015 | FR-015-AC-5 | TC-044 |  |
| FR-015 | FR-015-AC-6 | TC-045 |  |
| FR-016 | FR-016-AC-1 | TC-025, TC-053, TC-106 |  |
| FR-016 | FR-016-AC-2 | TC-026, TC-046 |  |
| FR-016 | FR-016-AC-3 | TC-027, TC-047 |  |
| FR-016 | FR-016-AC-4 | TC-028, TC-048 |  |
| FR-016 | FR-016-AC-5 | TC-029, TC-048 |  |
| FR-016 | FR-016-AC-6 | TC-049 |  |
| FR-016 | FR-016-AC-7 | TC-050, TC-053 |  |
| FR-016 | FR-016-AC-8 | TC-051 |  |
| FR-016 | FR-016-AC-9 | TC-052 |  |
| FR-017 | FR-017-AC-1 | TC-054 |  |
| FR-017 | FR-017-AC-3 | TC-030, TC-031, TC-032, TC-033, TC-034 |  |
| FR-042 | FR-042-AC-1 | TC-121 |  |
| FR-042 | FR-042-AC-2 | TC-121 |  |
| FR-042 | FR-042-AC-3 | TC-121 |  |
| FR-042 | FR-042-AC-4 | TC-121 |  |
| FR-042 | FR-042-AC-5 | TC-121 |  |
| FR-042 | FR-042-AC-6 | TC-121 |  |
| FR-042 | FR-042-AC-7 | TC-121 |  |
| FR-042 | FR-042-AC-8 | TC-121 |  |
| FR-042 | FR-042-AC-9 | TC-121 |  |
| FR-042 | FR-042-AC-10 | TC-121, TC-135 |  |
| FR-042 | FR-042-AC-11 | TC-121 |  |
| FR-042 | FR-042-AC-12 | TC-121 |  |
| FR-042 | FR-042-AC-13 | TC-121 |  |
| FR-042 | FR-042-AC-14 | TC-121 |  |
| FR-042 | FR-042-AC-15 | TC-121 |  |
| FR-046 | FR-046-AC-1 | TC-126 |  |
| FR-046 | FR-046-AC-2 | TC-126 |  |
| FR-046 | FR-046-AC-3 | TC-127 |  |
| FR-046 | FR-046-AC-4 | TC-127 |  |
| FR-046 | FR-046-AC-5 | TC-127 |  |
| FR-046 | FR-046-AC-6 | TC-128 |  |
| FR-046 | FR-046-AC-7 | TC-128 |  |
| FR-046 | FR-046-AC-8 | TC-126, TC-127 |  |
| FR-047 | FR-047-AC-1 | TC-129 |  |
| FR-047 | FR-047-AC-2 | TC-129, TC-131 |  |
| FR-047 | FR-047-AC-3 | TC-129 |  |
| FR-047 | FR-047-AC-4 | TC-130 |  |
| FR-047 | FR-047-AC-5 | TC-130 |  |
| FR-047 | FR-047-AC-6 | TC-130 |  |
| FR-047 | FR-047-AC-7 | TC-131 |  |
| FR-047 | FR-047-AC-8 | TC-131 |  |
| FR-048 | FR-048-AC-1 | TC-132 |  |
| FR-048 | FR-048-AC-2 | TC-132 |  |
| FR-048 | FR-048-AC-3 | TC-133 |  |
| FR-048 | FR-048-AC-4 | TC-133 |  |
| FR-048 | FR-048-AC-5 | TC-133 |  |
| FR-048 | FR-048-AC-6 | TC-134 |  |
| FR-048 | FR-048-AC-7 | TC-134 |  |
| FR-048 | FR-048-AC-8 | TC-134 |  |
| FR-048 | FR-048-AC-9 | TC-135 |  |
| FR-048 | FR-048-AC-10 | TC-135 |  |
| FR-049 | FR-049-AC-1 | TC-136 |  |
| FR-049 | FR-049-AC-2 | TC-136 |  |
| FR-049 | FR-049-AC-3 | TC-136 |  |
| FR-049 | FR-049-AC-4 | TC-136 |  |
| FR-049 | FR-049-AC-5 | TC-136 |  |
| FR-049 | FR-049-AC-6 | TC-136, TC-137 |  |
| FR-049 | FR-049-AC-7 | TC-137 |  |
| FR-049 | FR-049-AC-8 | TC-137 |  |
| FR-049 | FR-049-AC-9 | TC-142 |  |
| FR-050 | FR-050-AC-1 | TC-138 |  |
| FR-050 | FR-050-AC-2 | TC-138 |  |
| FR-050 | FR-050-AC-3 | TC-138 |  |
| FR-050 | FR-050-AC-4 | TC-138 |  |
| FR-050 | FR-050-AC-5 | TC-138 |  |
| FR-050 | FR-050-AC-6 | TC-138 |  |
| FR-050 | FR-050-AC-7 | TC-138 |  |
| FR-050 | FR-050-AC-8 | TC-138 |  |
| FR-054 | FR-054-AC-1 | TC-143 |  |
| FR-054 | FR-054-AC-2 | TC-143 |  |
| FR-054 | FR-054-AC-3 | TC-143 |  |
| FR-054 | FR-054-AC-4 | TC-143 |  |
| FR-056 | FR-056-AC-1 | TC-145 |  |
| FR-056 | FR-056-AC-2 | TC-145 |  |
| FR-056 | FR-056-AC-3 | TC-146 |  |
| FR-056 | FR-056-AC-4 | TC-145 |  |
| FR-056 | FR-056-AC-5 | TC-145 |  |
| FR-056 | FR-056-AC-6 | TC-147 |  |
| FR-056 | FR-056-AC-7 | TC-145, TC-147 |  |
| FR-056 | FR-056-AC-8 | TC-148, IT-012 |  |
| FR-056 | FR-056-AC-12 | TC-145 |  |
| FR-056 | FR-056-AC-13 | TC-145 |  |
| FR-056 | FR-056-AC-14 | TC-145 |  |
| FR-056 | FR-056-AC-15 | TC-145 |  |
| FR-056 | FR-056-AC-16 | TC-911 |  |

FR-017-AC-2 is verified by Inspection rather than a Test Case; no test symbol
is invented for that criterion.

## Test Case Summary

| Test ID | Title | Type | Priority | Traces To | Status |
| --- | --- | --- | --- | --- | --- |
| TC-020 | Exact qualified import | Integration | P1 | FR-005-AC-1 | ✅ |
| TC-021 | Missing selected import | Integration | P1 | FR-005-AC-2 | ✅ |
| TC-022 | Ambiguous exported declaration | Integration | P1 | FR-005-AC-3 | ✅ |
| TC-023 | Stale package closure | Integration | P1 | FR-005-AC-4 | ✅ |
| TC-024 | Failed linkage is atomic | Property | P1 | FR-005-AC-5 | ✅ |
| TC-025 | Unguarded optional unwrap | Integration | P1 | FR-006-AC-1, FR-016-AC-1 | ✅ |
| TC-026 | Presence facts stay with their observation | Integration | P1 | FR-006-AC-2, FR-016-AC-2 | ✅ |
| TC-027 | Guarded bounded addition | Integration | P1 | FR-006-AC-3, FR-016-AC-3 | ✅ |
| TC-028 | Ambiguous scalar inference | Integration | P1 | FR-006-AC-4, FR-016-AC-4 | ✅ |
| TC-029 | Clause roots are Boolean | Integration | P1 | FR-006-AC-5, FR-016-AC-5 | ✅ |
| TC-030 | Exact source and formal artifact binding | Integration | P1 | FR-013-AC-1, FR-013-AC-2 | ✅ |
| TC-031 | Lexical and formal declaration occurrences | Integration | P1 | FR-013-AC-3 | ✅ |
| TC-032 | Unmapped reference and operation forms | Integration | P1 | FR-013-AC-4 | ✅ |
| TC-033 | Native linking resource ceilings | Property | P1 | FR-013-AC-5 | ✅ |
| TC-034 | Ambiguity provenance and atomicity | Property | P1 | FR-013-AC-6 | ✅ |
| TC-035 | Explicit source identity assignment | Integration | P1 | FR-014-AC-1 | ✅ |
| TC-036 | Independent formal coordinate examples | Integration | P1 | FR-014-AC-2 | ✅ |
| TC-037 | Foreign native source requests | Integration | P1 | FR-014-AC-3 | ✅ |
| TC-038 | Inconsistent formal coordinates | Integration | P1 | FR-014-AC-4 | ✅ |
| TC-039 | Bounded generated span correspondence | Property | P1 | FR-014-AC-5 | ✅ |
| TC-040 | Qualify the source-derived native rule model | Integration | P1 | FR-015-AC-1 | ✅ |
| TC-041 | Refuse missing or inconsistent native model roles | Integration | P1 | FR-015-AC-2 | ✅ |
| TC-042 | Bind all native model semantics and provenance | Property | P1 | FR-015-AC-3 | ✅ |
| TC-043 | Verify model loci and inventory identity consistency | Integration | P1 | FR-015-AC-4 | ✅ |
| TC-044 | Resolve explicit references and operation declarations | Integration | P1 | FR-015-AC-5 | ✅ |
| TC-045 | Bound model construction and native linkage | Property | P1 | FR-015-AC-6 | ✅ |
| TC-046 | Check observation and operation value availability | Integration | P1 | FR-016-AC-2 | ✅ |
| TC-047 | Prove signed arithmetic through the actual IR API | Integration | P1 | FR-016-AC-3 | ✅ |
| TC-048 | Solve exact native contextual types | Integration | P1 | FR-016-AC-4, FR-016-AC-5 | ✅ |
| TC-049 | Retain exact checked source and authored clause bindings | Integration | P1 | FR-016-AC-6 | ✅ |
| TC-050 | Check lexical scope and guarded evaluation order | Integration | P1 | FR-016-AC-7 | ✅ |
| TC-051 | Bound constraint checking and shared proof expansion | Property | P1 | FR-016-AC-8 | ✅ |
| TC-052 | Retain population and invocation obligations after checking | Integration | P1 | FR-016-AC-9 | ✅ |
| TC-053 | Independent guard-fact truth-table soundness | Property | P1 | FR-016-AC-1, FR-016-AC-7 | ✅ |
| TC-054 | Exact decoded JSON occurrence provenance | Integration | P1 | FR-017-AC-1 | ✅ |
| TC-113 | Composed syntax and historical grammar | Integration | P1 | FR-035-AC-1..FR-035-AC-6 | ✅ |
| TC-114 | Composed dependencies and declaration-owned roles | Integration | P1 | FR-036-AC-1..FR-036-AC-4, FR-036-AC-7 | 🚧 |
| TC-115 | Static meaning and requested capabilities | Integration | P1 | FR-036-AC-5, FR-036-AC-6, FR-036-AC-8, FR-057-AC-11 | 🚧 |
| TC-119 | Composed value types and guarded definedness | Integration | P1 | FR-040-AC-1..FR-040-AC-10 | ✅ |
| TC-120 | Explicit rational model profile and historical isolation | Integration | P1 | FR-041-AC-1..FR-041-AC-7 | ✅ |
| TC-117 | Exact numeric wire values and strict refusal | Integration | P1 | FR-038-AC-1..FR-038-AC-5 | ✅ |
| TC-121 | Full compiled protocol artifact and Rust handoff requiring [B's IT-001](ix://agent-ix/quire-protocol/IT-001) | Integration | P1 | FR-042-AC-1..FR-042-AC-15 | 🚧 |
| TC-126 | Preserve exact predicate meaning at cross-family calls | Integration | P1 | FR-046-AC-1, FR-046-AC-2, FR-046-AC-8 | ✅ |
| TC-127 | Evaluate ordered query values against independent expected results | Integration | P1 | FR-046-AC-3, FR-046-AC-4, FR-046-AC-5, FR-046-AC-8 | ✅ |
| TC-128 | Keep incomplete query inputs and exhausted work distinct from values | Integration | P1 | FR-046-AC-6, FR-046-AC-7 | ✅ |
| TC-129 | Bind exact finite graph identities and inputs | Integration | P1 | FR-047-AC-1..FR-047-AC-3 | ✅ |
| TC-130 | Evaluate positive-length reachability over finite cycles | Integration | P1 | FR-047-AC-4..FR-047-AC-6 | ✅ |
| TC-131 | Bound graph evaluation and preserve historical profiles | Integration | P1 | FR-047-AC-2, FR-047-AC-7, FR-047-AC-8 | ✅ |
| TC-132 | Preserve workflow, role and channel occurrence identities | Integration | P1 | FR-048-AC-1, FR-048-AC-2 | 🚧 |
| TC-133 | Admit bounded choreography control and visible progress | Integration | P1 | FR-048-AC-3..FR-048-AC-5 | ✅ |
| TC-134 | Preserve compensation, retry, commit and recovery prerequisites | Integration | P1 | FR-048-AC-6..FR-048-AC-8 | ✅ |
| TC-135 | Demonstrate the composed compiler-to-assessment ecosystem handoff | E2E | P1 | FR-048-AC-9, FR-048-AC-10, FR-042-AC-10 | 🚧 |
| TC-136 | Admit exact composed state views and typed outcomes | Integration | P1 | FR-049-AC-1..FR-049-AC-6 | 🚧 |
| TC-137 | Bound composed evaluation and retry immutable inputs | Property | P1 | FR-049-AC-6..FR-049-AC-8, NFR-009 | ✅ |
| TC-138 | Publish and read authenticated compiled temporal selections | Integration | P1 | FR-050-AC-1..FR-050-AC-8 | ✅ |
| TC-142 | Evaluate admitted version-2 compensation expressions and initialized captures | Integration | P1 | FR-049-AC-9, NFR-009-AC-4 | ✅ |
| TC-143 | Publish and read strict control temporal activation mappings | Integration | P1 | FR-054-AC-1..FR-054-AC-4 | ✅ |
| TC-145 | Admit an IR 2.0.0 domain package as model declarations | Integration | P0 | FR-056-AC-1, FR-056-AC-2, FR-056-AC-4, FR-056-AC-5, FR-056-AC-7, FR-056-AC-12, FR-056-AC-13, FR-056-AC-14, FR-056-AC-15 | 🚧 |
| TC-146 | Refuse an unknown or mismatched construct meaning id | Integration | P0 | FR-056-AC-3 | 🚧 |
| TC-147 | Account for and reproduce domain-package intake | Integration | P0 | FR-056-AC-6, FR-056-AC-7 | 🚧 |
| TC-148 | Link native source against a domain-package bundle end to end | Integration | P0 | FR-056-AC-8, FR-036-AC-9 | 🚧 |
| IT-012 | Admit a spec artifact bundle through quire-rs and the FCD semantic IR crates | Integration | P0 | FR-056-AC-8, FR-036-AC-9 | 🚧 |
| TC-153 | Admit exactly the ten capability kinds and refuse every other label | Unit | P0 | FR-057-AC-1, FR-057-AC-2, FR-057-AC-4, FR-057-AC-7, FR-057-AC-10 | 🚧 |
| TC-154 | Refuse a capability carrier with an unsupported vocabulary version | Unit | P0 | FR-057-AC-3 | 🚧 |
| TC-155 | Keep admission backend-independent and route only supported items | Integration | P0 | FR-057-AC-5, FR-057-AC-6, FR-057-AC-8 | 🚧 |

## Domain-package model intake (L2)

[FR-056](../functional/FR-056-admit-domain-package-model-declarations.md) admits
one Semantic IR 2.0.0 domain package per ModelSelection under Quire specification
FR-154, reading lifted bytes through `agent-ix-semantic-ir`. TC-145 covers FR-154
admission refusals, reader refusal, artifact-id identity, relationship exports,
all-or-nothing declaration refusal and digest-slot separation; TC-146 covers
construct and meaning-id binding under Quire specification FR-208 and FR-154;
TC-147 covers `normalize.record` accounting and
byte-identical results from both entry points. TC-148 and IT-012 run the
agent-ix/filament-core-data#173 architecture fixture through quire-rs, the lift
and intake, resolve its ports and a connection under FR-152, and link native
model references against it (FR-036-AC-9).

TC-148 runs in `tests/it/composed_domain_models.rs` over a copy of the fixture
bundle taken from the pinned `agent-ix-extraction-frontend` checkout at test
time. The copy retypes `Pump`/`Sys`/`Tank.id` as `Boolean` and drops `Flow2`,
because the unedited bundle does not admit whole at this pin: `UUID` is not a
QSL native value type (PLAT-836), and FCD identifies `Flow2`'s inline
relationship as `.../relationship/Flow2-specializes-Flow` rather than
`<owner>/<name>` (FCD `crates/extraction-frontend/src/identity.rs:223`,
PLAT-1064). `Count`, a record value type (FR-208), admits as authored. Every
part, port, connection and allocation stays as authored. The linker binds
a native reference to a domain declaration by its FR-154 key and kind
(`ModelTarget::Declaration`). A type site binds an object, interface, record
value or value type; a protocol role also binds a Part or Port; a protocol
relationship binds a Connection or navigation relationship; any other kind
refuses at the linker. The composed type checker types a domain object type,
Interface or record value type, and a field read by the field's value type
(`Boolean`, or another such domain type, under multiplicity `1`, `0..1` or
`0..n`); any other field type, and a domain value type, refuses as an
unsupported prerequisite.

| Functional Req | Acceptance Criteria | Test Cases | Status |
| --- | --- | --- | --- |
| FR-035 | FR-035-AC-1 | TC-113 |  |
| FR-035 | FR-035-AC-2 | TC-113 |  |
| FR-035 | FR-035-AC-3 | TC-113 |  |
| FR-035 | FR-035-AC-4 | TC-113 |  |
| FR-035 | FR-035-AC-5 | TC-113 |  |
| FR-035 | FR-035-AC-6 | TC-113 |  |
| FR-036 | FR-036-AC-1 | TC-114 |  |
| FR-036 | FR-036-AC-2 | TC-114 |  |
| FR-036 | FR-036-AC-3 | TC-114 |  |
| FR-036 | FR-036-AC-4 | TC-114 |  |
| FR-036 | FR-036-AC-5 | TC-115 |  |
| FR-036 | FR-036-AC-6 | TC-115 |  |
| FR-036 | FR-036-AC-7 | TC-114 |  |
| FR-036 | FR-036-AC-8 | TC-115 |  |
| FR-036 | FR-036-AC-9 | TC-148 |  |
| FR-057 | FR-057-AC-1 | TC-153 |  |
| FR-057 | FR-057-AC-2 | TC-153 |  |
| FR-057 | FR-057-AC-3 | TC-154 |  |
| FR-057 | FR-057-AC-4 | TC-153 |  |
| FR-057 | FR-057-AC-5 | TC-155 |  |
| FR-057 | FR-057-AC-6 | TC-155 |  |
| FR-057 | FR-057-AC-7 | TC-153 |  |
| FR-057 | FR-057-AC-8 | TC-155 |  |
| FR-057 | FR-057-AC-10 | TC-153 |  |
| FR-057 | FR-057-AC-11 | TC-115 |  |
| FR-040 | FR-040-AC-1 | TC-119 |  |
| FR-040 | FR-040-AC-2 | TC-119 |  |
| FR-040 | FR-040-AC-3 | TC-119 |  |
| FR-040 | FR-040-AC-4 | TC-119 |  |
| FR-040 | FR-040-AC-5 | TC-119 |  |
| FR-040 | FR-040-AC-6 | TC-119 |  |
| FR-040 | FR-040-AC-7 | TC-119 |  |
| FR-040 | FR-040-AC-8 | TC-119 |  |
| FR-040 | FR-040-AC-9 | TC-119 |  |
| FR-040 | FR-040-AC-10 | TC-119 |  |
| FR-041 | FR-041-AC-1 | TC-120 |  |
| FR-041 | FR-041-AC-2 | TC-120 |  |
| FR-041 | FR-041-AC-3 | TC-120 |  |
| FR-041 | FR-041-AC-4 | TC-120 |  |
| FR-041 | FR-041-AC-5 | TC-120 |  |
| FR-041 | FR-041-AC-6 | TC-120 |  |
| FR-041 | FR-041-AC-7 | TC-120 |  |

## Compiled protocol numeric component

The separate numeric component of compiler #40 is below; it does not
establish composed parsing, model admission or source-to-artifact correspondence.

| Functional Req | Acceptance Criteria | Test Cases | Status |
| --- | --- | --- | --- |
| FR-038 | FR-038-AC-1 | TC-117 |  |
| FR-038 | FR-038-AC-2 | TC-117 |  |
| FR-038 | FR-038-AC-3 | TC-117 |  |
| FR-038 | FR-038-AC-4 | TC-117 |  |
| FR-038 | FR-038-AC-5 | TC-117 |  |

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
`tests/native_digest_domains.rs` proves the source/model/config digest domains
stay separate from the compiled-artifact seal and from a consumer's RFC 8785/JCS
protocol-result identity: an exact model reference survives emission and
reading, either substituted digest refuses, and a recanonicalized model
reference refuses at model admission.

AC-10's QSL test covers only the compiler-owned publication and strict-reader
leg; the cross-repository Protocol leg is not this repository's. FR-042-AC-11's positive half runs
over the architecture bundle: a domain-typed declaration checks, emits a
`Model` naming its domain package, and reads back. FR-042-AC-13 runs over the
same bundle with a population declaration added to its lifted document: a
domain object population and a domain operation check, emit and read back.
FR-042-AC-14 adds operations with parameters and a result to `Pump` in the
bundle copy: clauses reading them check, emit and read back.

FR-042-AC-15 is the public in-process producer: behind the
`handoff-writer` feature, `handoff::write_v1` compiles the same authored
recipe, reads its own output back through the strict reader used above, then
writes a complete handoff -- including `dependencies/` bytes -- to a
caller-chosen directory. `tests/it/handoff_writer.rs` calls only that public
function, exactly as a downstream crate would, and checks determinism,
checksum completeness and refusal of an existing directory (leaving a sentinel
file already there untouched). The no-dev-dependency half of the acceptance is
not, and cannot be, this test's claim: a test under this crate's own `tests/`
always builds inside its dev-dependency closure. `make ci`'s `ci-clean-build`
target's `cargo check -p quire-spec-language --lib --no-default-features
--features handoff-writer` line is the actual check -- it resolves
`write_v1`/`write_v2`'s dependency graph on the library target alone, with no
dev-dependency in it (Makefile).

### Consumer handoff

QSL publishes and strictly reads the immutable four-source `/1` handoff through
version-explicit public addresses; that compiler-local leg does not establish
B/F ecosystem acceptance. The
downstream intake of the native compiler artifact is
[quire-protocol#11](https://github.com/agent-ix/quire-protocol/issues/11)
under epic [quire-protocol#14](https://github.com/agent-ix/quire-protocol/issues/14);
its integration caller owns [IT-001](ix://agent-ix/quire-protocol/IT-001) on the
consumer side. TC-135 records the separate D-owned campaign gate.

| Functional Req | Acceptance Criteria | Test Cases | Status |
| --- | --- | --- | --- |
| FR-042 | FR-042-AC-1 | TC-121 |  |
| FR-042 | FR-042-AC-2 | TC-121 |  |
| FR-042 | FR-042-AC-3 | TC-121 |  |
| FR-042 | FR-042-AC-4 | TC-121 |  |
| FR-042 | FR-042-AC-5 | TC-121 |  |
| FR-042 | FR-042-AC-6 | TC-121 |  |
| FR-042 | FR-042-AC-7 | TC-121 |  |
| FR-042 | FR-042-AC-8 | TC-121 |  |
| FR-042 | FR-042-AC-9 | TC-121 |  |
| FR-042 | FR-042-AC-10 | TC-121, TC-135 |  |
| FR-042 | FR-042-AC-11 | TC-121 |  |
| FR-042 | FR-042-AC-12 | TC-121 |  |
| FR-042 | FR-042-AC-13 | TC-121 |  |
| FR-042 | FR-042-AC-14 | TC-121 |  |
| FR-042 | FR-042-AC-15 | TC-121 |  |

## Authenticated temporal artifact selections (L5/L6)

[FR-050](../functional/FR-050-publish-authenticated-temporal-artifacts.md)
defines compiler #40's strict `quire.compiled-protocol/2` delta. It retains the
entire `/1` contract unchanged and adds only the declaration-indexed temporal
definition/clock table needed to authenticate the already merged L5 evaluator's
profile inputs.

TC-138 uses real native compilation, the strict `/2` reader and L5 evaluation.
Its version-crossing, definition-byte, numeric/profile and exact/one-short
controls prevent `/1` compatibility or trace premises from being mistaken for
authenticated `/2` correspondence.
Its handoff-address control imports the producer-owned directory, member and
format constants, verifies their committed inventory, and leaves no
environment-variable or duplicated-vocabulary escape hatch.

FR-050-AC-8 is FR-042-AC-15's `/2` counterpart: behind the same
`handoff-writer` feature, `handoff::write_v2` writes the complete authenticated
handoff -- offer, selection, reference, sources, model source, the three clock
inputs, every dependency's exact bytes, the `mutations/` corpus and manifest,
and a checksum inventory -- to a caller-chosen directory, in-process, replaying
every mutation to its expected refusal before writing. `tests/it/handoff_writer.rs`
calls only `write_v2`, beside `write_v1`, checking determinism, checksum
completeness, strict-reader admission, that every mutation case's referenced
file(s) exist, a replayed mutation case's refusal code against the strict
reader, and refusal of an existing directory (leaving a sentinel file already
there untouched). The same `make ci` `cargo check --lib --no-default-features
--features handoff-writer` line backs its no-dev-dependency half; see
FR-042-AC-15 above.

## Composed evaluation admission and bounds (L3/L4)

[FR-049](../functional/FR-049-admit-composed-evaluation-inputs.md) defines the
shared immutable input, authority and typed-outcome boundary used by composed
predicate/query and finite-graph evaluation. The public evaluator now exercises
the exact admitted-artifact and state-view contract.

[NFR-009](../non-functional/NFR-009-bound-composed-evaluation.md) constrains the
same evaluator with independent inclusive counters and charge-before-work
behavior. No native-runtime NFR-006 result is reused as evidence for this new
accounting identity.

| Quality Req | Verification Method | Test Cases |
| --- | --- | --- |
| NFR-009 | Test: negative-abuse-testing for every declared metric | TC-137, TC-142 |

## Reusable predicates and ordered queries (L3)

[FR-046](../functional/FR-046-execute-predicates-and-ordered-queries.md)
is the retrospective scoped requirement under issue #66. They serve US-002's exact model meaning and
US-003's distinction between a value and inability to evaluate, under StR-001.
Their illustrative user-story examples are not invented AC identifiers.

The executed cases distinguish integer/rational representation, exact/foreign
units, each query form, empty/nonempty domains, duplicate occurrences,
current/pre/captured origins and compatible/incompatible callee profiles.
Boundary controls include admitted wrapper maxima 1 and 10,000, refused
declaration maxima 0 and 10,001, empty runtime sequences, numerical prefix bounds
and zero/exact/one-short work capacities. TC-126 covers call/type/profile/guard
errors; TC-128 covers missing population/completeness and resource exhaustion.
There is no new state machine here; immutable source/anchor retention and fresh
retry accounting are the applicable lifecycle checks.

The pipeline exercises exact reduced-rational prefix proofs and typed unsupported
outcomes, all eight ordered query forms, explicit population completeness, every
declared input form, and zero/exact/one-short limits. Runtime expected results
are constructed independently from the offered compiled artifact; tags alone
are not treated as acceptance.

## Finite typed object-reference graphs (L4)

[FR-047](../functional/FR-047-evaluate-finite-object-reference-graphs.md)
retrospectively scopes the finite-graph profile under issue #66. The public
composed evaluator and native population emitter now exercise the scoped graph
contract directly.

TC-129 covers every identity/type/authority axis and distinguishes refusal from
unavailable closure. TC-130 covers isolated/self-loop/cyclic/duplicate/ordered
and edge-shape cases. TC-131 covers zero/exact/one-short work in each exercised
dimension, fresh retry and the frozen ConfigVersion profile. No shortest-path,
mutable graph or domain-package relationship behavior is inferred.

## Native ecosystem choreography preservation (L6)

[FR-048](../functional/FR-048-preserve-native-choreography-semantics.md)
retrospectively scopes compiler-owned choreography preservation under issue #66.
The compiler rows stop at source-to-artifact authority; B and F retain the
conformance and observation assertions in TC-135.

TC-132/133/134 cover identity, control and recovery independently before the
composed integration. TC-135 executes the real release-compiler and B-intake leg
locally, then defines D's separately owned campaign gate under quire-research
#39/#49. The external run will pin accepted B conformance and F handoff revisions
and cover healthy batch/incremental agreement plus cross-order, missing/ambiguous
authority, late/missing refund, failed compensation, wrong selection and
one-short resource controls.

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

The historical rows specify evidence for LC02. The accepted IR ADR-0054
removes the earlier prerequisite for a shared Filament model adapter. The
generic lane uses the public formal declaration API; A owns concrete native
projection work for clauses that need additional semantic correspondence.
The full workflow remains IT-002 and the original Agent A assignment.

FR-013 now defines the concrete formal-environment resolution API and TC-030–034
cover its six criteria. TC-020–024 consume that same real API. Canonical byte
selection, explicit self binding, lexical scopes and resource budgets are
defined before code; FR-006's five static-judgment cases remain separate.

## Formal source bridge qualification

FR-014 adds five cases to the same LC02 matrix. TC-035–038 exercise
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
semantics.

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

## Complete native model qualification

Rust tests for TC-040–045 cover every primitive site/wrapper, native-only role/carrier/operation
refusals, ordinary zero-bounded text, unused unsupported IR declarations, all
source-locus classes, seven source-derived semantic mutations and six inventory
permutations. Artifact payload assertions preserve exact signed i64 extrema,
changed bounds, unused values and ordered parameters. False line/column/source/
revision and split-scalar loci are constructor-valid before native refusal.

Exact small ModelLimits include every dimension and every aggregate-entry class.
Valid 10,000-node/10,000-entry models pass at the defaults; the next required
node or entry fails at the default and passes with that limit raised. Deeply
nested models pass under node limits sized for them. The 10,001-role ceiling is
observed before artifact work; 10,000 full roles exceed this fixture's artifact
ceiling, so that is recorded as a coupled refusal rather than an exact success.
The earlier native-link tests retain exact 1 MiB and 8 MiB artifact boundaries.

## Native checker qualification

In TC-025–029 and TC-046–053, actual reference unwraps,
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

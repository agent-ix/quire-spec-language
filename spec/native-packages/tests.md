---
id: TM-005
title: "Native package construction and reconstruction matrix"
type: TestMatrix
---

## Overview

This LC02 slice covers FR-019/020/021, NFR-007 and IT-007. All 27 functional
criteria have explicit cases. All fourteen cases are qualified by the producer evidence in SR-111 and the
reader/runtime evidence in the PR review. Native payload qualification is complete. Existing TM-001–004
and qualified native runtime behavior retain their scopes. A planned mapping
is not executed evidence or complete LC02/FS05/backend/Quire acceptance.

## Requirements Traceability

### Functional Requirement Coverage

| Functional Req | Acceptance Criteria | Test Cases | Coverage Status |
| --- | --- | --- | --- |
| FR-019 | FR-019-AC-1 | TC-078, TC-089 | ✅ Qualified |
| FR-019 | FR-019-AC-2 | TC-078 | ✅ Qualified |
| FR-019 | FR-019-AC-3 | TC-078 | ✅ Qualified |
| FR-019 | FR-019-AC-4 | TC-079 | ✅ Qualified |
| FR-019 | FR-019-AC-5 | TC-080 | ✅ Qualified |
| FR-019 | FR-019-AC-6 | TC-080 | ✅ Qualified |
| FR-019 | FR-019-AC-7 | TC-081, TC-089 | ✅ Qualified |
| FR-019 | FR-019-AC-8 | TC-082 | ✅ Qualified |
| FR-019 | FR-019-AC-9 | TC-081, TC-082 | ✅ Qualified |
| FR-019 | FR-019-AC-10 | TC-088 | ✅ Qualified |
| FR-020 | FR-020-AC-1 | TC-089 | ✅ Qualified |
| FR-020 | FR-020-AC-2 | TC-083 | ✅ Qualified |
| FR-020 | FR-020-AC-3 | TC-084 | ✅ Qualified |
| FR-020 | FR-020-AC-4 | TC-084 | ✅ Qualified |
| FR-020 | FR-020-AC-5 | TC-085 | ✅ Qualified |
| FR-020 | FR-020-AC-6 | TC-085 | ✅ Qualified |
| FR-020 | FR-020-AC-7 | TC-086 | ✅ Qualified |
| FR-020 | FR-020-AC-8 | TC-087 | ✅ Qualified |
| FR-020 | FR-020-AC-9 | TC-087, TC-088 | ✅ Qualified |
| FR-020 | FR-020-AC-10 | TC-082 | ✅ Qualified |
| FR-020 | FR-020-AC-11 | TC-081, TC-086 | ✅ Qualified |
| FR-021 | FR-021-AC-1 | TC-090 | ✅ Qualified |
| FR-021 | FR-021-AC-2 | TC-082 | ✅ Qualified |
| FR-021 | FR-021-AC-3 | TC-091 | ✅ Qualified |
| FR-021 | FR-021-AC-4 | TC-091 | ✅ Qualified |
| FR-021 | FR-021-AC-5 | TC-091 | ✅ Qualified |
| FR-021 | FR-021-AC-6 | TC-088, TC-090 | ✅ Qualified |

### Non-Functional Requirement Coverage

| Non-Functional Req | Verification Method | Evidence/Test Cases | Status |
| --- | --- | --- | --- |
| NFR-007 | Test: negative-abuse-testing | TC-088; offered/emitted bytes, string content, entries, depth, every bounded pass and independent frontend limits | ✅ All package passes and frontend limits qualified |
| NFR-005 | Inspection and existing Rust gates | Source/dependency/CI review and actual Rust qualification | ✅ Qualified |

## Test Case Summary

| Test ID | Title | Type | Priority | Traces To | Status |
| --- | --- | --- | --- | --- | --- |
| TC-078 | Retain complete package inventories | Integration | P1 | FR-019-AC-1, FR-019-AC-2, FR-019-AC-3 | ✅ Qualified |
| TC-079 | Retain package resolutions and obligations | Integration | P1 | FR-019-AC-4 | ✅ Qualified |
| TC-080 | Bind package semantics and features | Property | P1 | FR-019-AC-5, FR-019-AC-6 | ✅ Qualified |
| TC-081 | Retain unlowered package projections | Integration | P1 | FR-019-AC-7, FR-019-AC-9, FR-020-AC-11 | ✅ Qualified |
| TC-082 | Preserve static package byte identity | Property | P1 | FR-019-AC-8, FR-019-AC-9, FR-020-AC-10, FR-021-AC-2 | ✅ Qualified |
| TC-083 | Reject nonclosed package JSON | Property | P1 | FR-020-AC-2 | ✅ Qualified |
| TC-084 | Select package versions and features | Property | P1 | FR-020-AC-3, FR-020-AC-4 | ✅ Qualified |
| TC-085 | Verify package dependencies and authorship | Property | P1 | FR-020-AC-5, FR-020-AC-6 | ✅ Qualified |
| TC-086 | Reject forged package derivations | Property | P1 | FR-020-AC-7, FR-020-AC-11 | ✅ Qualified |
| TC-087 | Reconstruct through real compiler stages | Integration | P1 | FR-020-AC-8, FR-020-AC-9 | ✅ Qualified |
| TC-088 | Bound package passes and retries | Property | P1 | FR-019-AC-10, FR-020-AC-9, FR-021-AC-6 | ✅ Qualified |
| TC-089 | Qualify native package runtime reconstruction | Integration | P1 | FR-020-AC-1, FR-019-AC-1, FR-019-AC-7 | ✅ Qualified |
| TC-090 | Qualify native canonical bytes and domain vectors | Property | P1 | FR-021-AC-1, FR-021-AC-6 | ✅ Qualified |
| TC-091 | Reject static identity and projection substitutions | Property | P1 | FR-021-AC-3, FR-021-AC-4, FR-021-AC-5 | ✅ Qualified |

## Test Matrix Rules

EARS/requirement grammar was run before updating this matrix: all 227 current
spec documents were grammar-clean using Quire 0.31.0. The initial 218-document
identity/reader-order check remains historical evidence.
The six rules below govern the completed local
qualification. Tests use imported bare single-line #[trace(...)] attributes
with minted TC/FR criterion IDs. IT procedure labels and NFR metric ordinals
are not invented trace IDs.

## Option Permutation Matrix

| Dimension | Required combinations | Cases |
| --- | --- | --- |
| Source inventory | One import/constant clause minimum, multiple owners/clauses, current/pre/post and multiple aliases; header-only/import-only are adverse source inputs | TC-078, TC-087, TC-089 |
| Model contents | Used/unused declarations; scalar/unit/bounds, enum, structural record, object/reference, option/sequence and operation/frame roles | TC-078–080 |
| Features | Exact set, unique permutations, duplicates/escaped duplicates, unknown, omitted/invented known feature, consumer subset and extra unknown consumer strings | TC-080, TC-084, TC-086 |
| Wire selectors | Exact/unknown format, language, edition, profiles, definition revision/digest; no decoder fallback | TC-083–085 |
| Bindings | Exact/missing/duplicate/extra/foreign source/model/authored clause; conflicting selected or unselected model identities | TC-085, TC-087 |
| Representation | Producer bytes, reordered object members/escaped spellings, reordered unique features, changed ordered arrays | TC-082–084 |
| Canonical identity | Independent full bytes/preimage, large exact revision, Unicode/control escapes, every static dependency, excluded projection forgery, unknown domains and raw/IR/JCS role substitutions | TC-090–091 |
| Limits | Independent package and parser/link/check defaults, zero, lowered, elevated hard options, each pass and retry | TC-087–088 |

## Constraint Boundary Tests

TC-088 independently counts offered bytes before hashing, emitted bytes before
append, decoded string content before retention, members/elements before storage
and container depth. Zero/exact/one-below and hard/elevated/one-over controls
cover each; content or upstream coupling remains explicit. Delimiter text inside
strings is not JSON nesting. Recognition/decode/derive/canonical/encode/compare retain
separate usage. TC-087 lowers each existing frontend limit on otherwise valid
selected source/models so a setup refusal cannot mask the expected stage.

## Error Paths and State Transitions

Construction proceeds from actual CheckedPackage to immutable artifact or
PackageError; reading proceeds from exact byte selection through closed
recognition, format selection, closed typed decode, semantic/feature selection,
external dependency/authored-binding checks,
real parse/link/check, full derivation comparison and accepted package. No
intermediate stage grants a public successful package. TC-083–087 cover every
documented wire/version/identity/derivation/native refusal. TC-088 covers
incompleteness and fresh retries; TC-089 covers subsequent runtime truth,
refusal, frame failure, incomplete population and exhausted evaluation.
TC-084 exercises multi-defect precedence, including unknown-version payloads
with different valid JSON fields and independent earlier intake failures.
TC-091 establishes that static identity cannot authorize forged capabilities.

## Edge Cases

- Smallest admitted import/constant-clause source: TC-078; header-only/import-only syntax refusals: TC-087.
- Different display paths with identical original source identity/bytes: TC-078/082.
- Unused selected declarations and unreachable feature-bearing syntax: TC-078/080.
- Escaped duplicate names, null versus omitted, exact u64 revisions and malformed numbers: TC-083/084.
- IR-shaped nested records with unexpected members: TC-083.
- Attacker recomputes raw digest after changing semantic claims: TC-085/086.
- Original package dropped before reconstruction: TC-089.
- Deleted-object pre capture and post result; source-bound incomplete event prefixes: TC-089.
- Real checker proof supplied as an invented executable disposition: TC-081/086.
- Distinct package/native-source/IR/JCS identity roles: TC-081/082.
- Unicode/control escaping and exact revision 9007199254740993: TC-090.
- Same static identity with a forged excluded projection: TC-091.

## Integration Test Matrix

IT-007 composes the actual public Rust package, native compiler, pinned IR and
runtime APIs. TC-078/079/081/087/089 exercise their real boundaries; generated
mutation cases also start from successful real setup. There is no service,
browser, event bus or database in this scope, so those classifications are not
fabricated. The complete compiled ConfigVersion/backend workflow remains IT-002.

## Evidence Strategy

Independent manifest assertions and one-axis mutations complement actual
read/rebind observations; a serializer/reader round trip alone is insufficient.
Generated families qualify identity/feature/ordered-array transformations and
bounded work, with real frontend failure-stage controls. Existing independent
runtime oracles remain in the full regression. The API has no shared scheduler
or lock state for Loom; no fuzz or mutation-adequacy result is claimed without
its separate actual tooling. The adviser's known limitations do not replace
this explicit assessment of applicable methods.

## Coverage Gaps

All fourteen cases have executed Rust evidence. NFR-007's five metrics are
measured at their actual boundaries, with private controls for coupled maxima
that cannot be reached through public intake. The PR review maps the reader
and runtime observations; SR-111 retains the producer evidence. The existing
NFR metric trace-target discrepancy remains a tooling limitation. Shared-domain
registration and full interchange acceptance remain open under LC02/FS05.
Independent B/C consumption,
LC04 actual lowering/backend parity and LC05 Quire integration remain required.
The existing catalog's Coverage Status versus Status mismatch still requires
manual status reconciliation; no module or shared classifier is changed here.

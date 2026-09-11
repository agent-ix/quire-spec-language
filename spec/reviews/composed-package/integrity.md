---
id: SR-302
title: "Integrity of composed compiler admission requirements"
type: SpecReview
analysis: integrity
scope: "FR-035/036; TC-113–115; IT-009; US-001/002; master index and TM-003 amendment"
review_set: all
evaluated_revision: "fa07b07"
---
## Summary

Reviewed the L2 requirements at `fa07b07` against the compiler at `dd34599` and
the shared standard draft at `d7483f3`. All fourteen criteria have planned cases;
two inherited package rules need explicit compiler controls before the exact
agreement and budget oracles are unambiguous.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-036 inherits the standard package contract's single-edition/header-agreement rule but does not state it directly or test mixed editions and a header disagreeing with the package's selected edition. Make that namespace-admission refusal explicit and add independent TC-114 mutations. | FR-036 Inputs/Behavior; TC-114; standard package-contract.md, Static package subject | missing-requirement |
| FND-002 | medium | FR-036 names finite traversal dimensions but does not require a declared accounting version, effective limits and charge rules before work. TC-114's exact/one-step budget oracle is therefore underspecified. Carry the standard NFR-010 accounting contract into this boundary without inventing universal numeric ceilings. | FR-036 Inputs/AC-7; TC-114 step 6; standard NFR-010/TC-048 | missing-requirement |

## Traceability and verification

| User need | Compiler requirement | Stakeholder path | Verification |
| --- | --- | --- | --- |
| US-001, precise authored-source feedback | FR-035 | US-001 traces to StR-001 | FR-035-AC-1 through FR-035-AC-6 map individually to TC-113 in TM-003 |
| US-002, exact intended model selection | FR-036 | US-002 traces to StR-001 | FR-036-AC-1/2/3/4/7 map to TC-114; AC-5/6/8 to TC-115; IT-009 adds real-producer controls for AC-1/3/4/6 |

The master index contains both FRs and IT-009; both user stories name the added
FR. Traceability to StR-001 is through the stated user-story relationships, not
a claimed new direct FR-to-StR edge. All added TM-003 criteria are Planned.
No existing Passed row or authored test procedure establishes composed execution.

NFR-001's source/token/node/nesting limits apply through FR-035's existing
syntax inputs and FR-002 seam. NFR-003's refusal/incompleteness rule agrees with
FR-036's retained request inventory and prohibition on aggregate success after
unfinished work. Rust tests and local/manual CI remain the existing contributor
and NFR-002/005 constraints. Standard NFR-010 scopes composed expansion accounting;
FND-002 concerns making that new package boundary's selected accounting explicit,
not prescribing a global memory threshold or using Cargo jobs as semantics.

## Implementation grounding and consistency

`src/syntax.rs` currently declares `0-draft`, flat expression IDs local to one
ParsedUnit and concrete bounded syntax limits. `src/parser.rs` uses the existing
Logos token stream and structured/Pratt parser; `header` retains an unknown
edition at Phase::Profile. Extending edition classification and family variants
fits the specified seam without a second body-string parser. Recognizing a
family still does not admit a profile, type or backend capability.

`src/linking.rs` preserves exact declaration owners and local spans in an atomic
single-unit LinkedPackage. `checking::check` separately establishes contextual
types and definedness; `package::NativePackage` is constructed from CheckedPackage
and the historical reader reconstructs claims through the actual compiler.
FR-036 explicitly separates its proposed per-declaration report from these
historical complete artifacts. TC-115 requires frozen identities and refuses
retagged partial reports. No new wire or canonical identity is inferred.

The requirements retain the full source inventory, unit-local aliases,
package-wide native names, exact source/definition/model owners and typed
role/anchor/scope dependencies. Duplicate or conflicting definitions refuse;
there is no first-wins or shape-based fallback. Missing units prevent namespace
admission while dependency-local failures preserve independent declarations.
The single-edition rule requires the additional explicit control in FND-001.

The shared package stage table supplies static definitions, model exports and
producer/native correspondence before dependent linking. Concrete populations,
windows and observations arrive at assessment. IT-009 names accepted contracts,
actual producer exports and an implemented composed entry point as prerequisites;
there is no interim synthetic-model success or invented ConfigVersion bound.
Pending producer implementation is an explicit integration dependency, not a
reason to relabel the historical fixture as composed evidence.

## Atomicity and failure probes

FR-035 owns edition-selected syntax and FR-036 owns package dependency admission;
their individual statements have independently observable outputs or refusals.
Retaining several identity fields is one preservation obligation. The standard
remains semantic authority; compiler requirements allocate implementation seams.

The supplied-inventory path invokes no external CLI, paginated/authenticated
API, concurrent retrieval or interactive scaffold, so those hidden-assumption
probes do not apply. Lookup conflicts have explicit refusal rules. User syntax
is parsed and linked without executing guards or importing runtime side effects.
Chain, diamond, self/mutual cycles and independent declarations have planned
controls. Explicit bounded protocol repetition and producer-owned model graphs
are distinguished from refused semantic dependency cycles. FND-002 ensures
their declared budget oracle can be reproduced rather than guessed.

## Correction recheck

Targeted reread of `d5047a8` resolves both findings while retaining their original
`fa07b07` observations above:

| Finding | Disposition at d5047a8 |
| --- | --- |
| FND-001 | Resolved. FR-036 now refuses namespace admission unless every header agrees with the inventory's single language/edition, retaining selections and original header loci. AC-1 and TC-114 step 3 require separate mixed-available-edition and equal-header/foreign-inventory mutations, plus a matching positive control. |
| FND-002 | Resolved. FR-036 declares accounting version, dimensions, capacities, effective limits and charging rules before work; shared dependencies, cache hits, revisits, zero, prospective charges, overflow and fresh retries are explicit. AC-7 and TC-114 step 6 derive expected charges independently from that contract and preserve inputs and the prior report. Historical per-unit limits do not silently set new package capacities. |

The master index now labels finite-state-only verification and transition text
as historical, includes L2's additional acceptance boundary and reflects the
approved A/B/D/E/F ownership with C's separate assurance work. These corrections
introduce no new semantic engine, numeric ceiling or execution claim.

## Verdict

The integrity verdict was CONDITIONAL at `fa07b07`; the scoped correction
recheck at `d5047a8` is PASS. Shared-standard/producer acceptance and all composed
implementation evidence remain outstanding. This review ran no build or Rust
test and creates no architecture or qualification campaign.

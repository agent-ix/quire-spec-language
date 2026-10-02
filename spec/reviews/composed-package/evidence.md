---
id: SR-304
title: "Evidence strategy for composed compiler admission"
type: SpecReview
analysis: evidence
scope: "FR-035/036; TC-113–115; IT-009; TM-003 composed amendment"
review_set: all
evaluated_revision: "fa07b07"
---
## Summary

Reviewed the fourteen authored acceptance criteria and the real-producer
integration prerequisite against the current compiler and the standard draft at
`d7483f3`. The planned Rust controls have concrete oracles; the advisor could not
produce recommendations because its CLI-version check failed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The deterministic advisor was attempted but failed before reading obligations. The method choices below are reviewer judgment using the installed catalog, not advisor matches or mismatch results. | FR-035-AC-1–6; FR-036-AC-1–8 |
| FND-002 | low | IT-009 requires accepted producer contracts, actual exports/correspondence and an implemented composed entry point. Those prerequisites and all new tests remain planned; historical fixtures cannot discharge this integration. | IT-009; TC-113–115 |

## Advisor and catalog evidence

`quoin advise --repo /home/peter/dev/worktrees/quire-language-composed-spec --json`
exited 2 with `could not determine the quire CLI version (expected >= 0.21.0)`.
The actual version command reports `quire 0.31.0 (cli 4f6ed024, engine
0.46.0@ca7362d4)`; installed Quoin reports `0.23.1`. No obligation recommendation,
matched applicability rule, evidence-store observation or mismatch population was
returned. This limitation is not an inconclusive recommendation for each AC and
does not justify changing authored methods from an imagined advisor result.

`quoin catalog methods --json` succeeded, with no duplicate or unreadable method
entries. Its `integration-testing`, `contract-testing` and
`negative-abuse-testing` methods all use class Test and evidence kind Integration.
The following choices retain the authored Test class by reviewer judgment:

| Obligations | Planned controls | Catalog method and reason |
| --- | --- | --- |
| FR-035-AC-1, FR-035-AC-2, FR-035-AC-3, FR-035-AC-4 | TC-113 actual public parser, typed family trees, independent source slices and refused mutations | integration-testing for the source/lexer/parser seam; negative-abuse-testing for malformed, wrong-position and trailing-input controls |
| FR-035-AC-5 | TC-113 known exact and insufficient syntax limits | negative-abuse-testing against the declared charge boundary, not timing or a benchmark threshold |
| FR-035-AC-6 | TC-113 frozen historical parse/format/refusal/package corpus | integration-testing; independently retained expectations prevent a round trip from hiding historical identity changes |
| FR-036-AC-1, FR-036-AC-3, FR-036-AC-4 | TC-114 exact sources, model owners, dependency selections and declaration-owned roles; IT-009 actual producer | integration-testing and contract-testing; same-shaped fixtures cannot establish producer/native correspondence |
| FR-036-AC-2, FR-036-AC-7 | TC-114 omitted inventory, wrong kind/scope, chain/diamond/cycle and lowered-budget cases | negative-abuse-testing; an explicit bounded protocol loop is a positive control against overbroad cycle refusal |
| FR-036-AC-5 | TC-115 independently changed assessment/backend/resource inputs and selected semantic components | integration-testing with explicit component comparison; no new canonical hash is an oracle |
| FR-036-AC-6 | TC-115 and IT-009 requested-inventory preservation and unsupported downstream dispositions | integration-testing; deleting either request must fail the inventory assertion |
| FR-036-AC-8 | TC-115 historical reader/rebinding corpus and partial-report retagging refusal | negative-abuse-testing and integration-testing against the original package contract |

## Trace and integration limits

TM-003 explicitly maps all six FR-035 and all eight FR-036 criteria to TC-113,
TC-114 or TC-115; every added row is Planned. A scoped Rust-source search found
no FR-035/036, TC-113–115 or IT-009 trace attributes. These are planned controls,
not missing execution disguised as a passed matrix. The existing harness uses
`ix_trace_rs::trace`; no new evidence framework or suite artifact is introduced.

The inspected `tests/parser.rs`, `tests/model_source.rs`,
`tests/package_construction_cases/fixed.rs` and `tests/package_reading.rs` supply
real historical boundaries and frozen expectations to preserve. Current
`src/syntax.rs` still selects edition `0-draft`; `link_native` returns one atomic
historical package and `NativePackage` requires actual checked input. None is
evidence that composed declarations already parse, link or execute.

IT-009 correctly distinguishes producer/static correspondence from later
assessment binding. Actual O1/O2 relationship and finite population roles must
come from the accepted producer interface, not invented bounds or a model-shaped
JSON mock. Current standard acceptance and producer implementation remain
prerequisites. Parsing temporal/protocol syntax does not require a monitor,
model checker or family evaluator to qualify this compiler admission slice.

## Correction recheck

Targeted reread at `d5047a8` confirms that TC-114 now independently exercises
mixed available editions and a package/header selection conflict, with a matching
positive control. Its resource cases read the declared accounting version,
dimensions, capacities and effective limits, then derive expected charges
independently rather than accepting the tested run's own consumption as its
oracle. Shared dependencies, cache hits, revisits, zero, overflow and preserved
retry reports are included. These corrections resolve the testability concerns
recorded in SR-302 without changing the authored Test methods or creating
execution evidence. The advisor limitation and IT-009 prerequisites remain.

## Verdict

PASS for the scoped authored evidence strategy and the `d5047a8` correction
recheck, with the advisor limitation recorded. This is a specification review,
not executed qualification or permission
to bypass the stated standard/producer acceptance gates. No build, Rust test,
hosted workflow or external producer execution was run for this review.

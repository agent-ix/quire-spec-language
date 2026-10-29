---
id: SR-301
title: "Failure-domain review of composed compiler parsing and linking"
type: SpecReview
analysis: failure-domain
scope: "Compiler FR-035/036, TC-113–115, IT-009 and changed US-001/002, master index and TM-003; current parser/syntax/linking/package interfaces"
review_set: all
evaluated_revision: "fa07b079861286c884e2f44380a3ed8f9508ef86"
review_date: "2026-09-10"
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-035, type: reviews }
  - { target: ix://agent-ix/quire-spec-language/FR-036, type: reviews }
---
## Summary

No blocking failure-domain gap was found in the proposed L2 parsing/linking
contract. Its adverse cases distinguish malformed syntax, unavailable editions,
dependency refusal, exhausted work and unsupported downstream requests without
turning a partial composed report into a checked or historical executable package.
This is requirements review grounded in existing interfaces, not implementation
or qualification of the new composed path.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No issues found in this lens. Explicit identity, purity, graph and exhaustion rules cover the changed boundary; TC-113–115 and IT-009 remain planned controls. | FR-035-AC-1–6; FR-036-AC-1–8; controls below |

## Failure discriminators

| Failure domain | Required outcome and concrete control |
| --- | --- |
| Source integrity and edition selection | FR-035 preserves exact bytes and half-open spans, rejects unavailable editions and malformed/trailing input, and preserves historical grammar. TC-113 uses CRLF, escaped/multibyte text, reserved words in member versus binder positions and the frozen historical corpus. Parser output cannot imply profile or model admission. |
| Shared expression purity and family separation | FR-035 uses the existing grammar/parser for `holds`, guards, captures and queries while retaining value, temporal and control variants. TC-113 distinguishes `true` from `holds(true)` and rejects a temporal formula in a value argument. No callback, source-text evaluator or user-code extension point is introduced; predicate evaluation stays downstream. |
| Cross-unit identity | FR-036 requires the closed explicit inventory, unit-local aliases, package-wide native names, exact declaration kind/owner and declaration-owned runtime roles. TC-114 contrasts equal alias/clock spellings across units/declarations, wrong-kind targets, foreign nominal owners and trigger names outside their scope. A unit-local expression handle cannot become a cross-unit identity. |
| Dependency failure containment | An unavailable/malformed source unit prevents namespace admission. A missing/conflicting definition or producer correspondence instead refuses all dependents while preserving unrelated bound declarations. TC-114 mutates each dependency beside an unrelated declaration and excludes files outside the inventory. No first/last candidate or ambient retrieval repairs the input. |
| Graph topology and termination | TC-114 exercises self/mutual predicate recursion, definition cycles, chains and diamonds. FR-036 refuses semantic cycles with their source occurrences, while bounded protocol repetition and identity-bearing model graphs retain their own policies. Finite supplied inventories and explicit traversal limits bound the work. |
| Resource stops and retries | TC-113 tests exact/one-over syntax limits; TC-114 tests zero, exact and insufficient traversal budgets. Exhaustion leaves unfinished work explicit and yields no complete/executable package; a sufficient-budget retry preserves original source/model inputs. |
| Static/runtime confusion | TC-114 and IT-009 link a template without live instances or future observations. TC-115 varies population/window/trace/backend inputs without changing static components, while a changed semantic selection changes the selected subject. Required runtime roles remain precise for later assessment binding. |
| Partial-result promotion | TC-115 retains supported and unsupported request entries, rejects a missing requested-inventory entry, and attempts to retag a partial composed report for the historical reader/runner. FR-036 forbids presenting bound syntax as checked, erasing unsupported requests or claiming complete aggregate success. |

## Grounding and limits

The current [parser](../../../src/parser.rs) calls the bounded lexer and
constructs a complete historical unit; [syntax](../../../src/syntax.rs) retains
source-owned arenas and explicitly documents `ExprId` as unit-local.
[Linking](../../../src/linking.rs) separates name correspondence from typing
and exposes atomic historical `link`/`link_native` results.
[Native model intake](../../../src/linking/native.rs) already distinguishes
conflicting source and model-owner inventories. These are reuse boundaries;
they are not evidence that multi-unit composed admission already exists.

[NativePackage](../../../src/package.rs) requires a `CheckedPackage`, and its
[strict reader](../../../src/package/reading.rs) checks historical selections and
reconstructs through the real compiler. The proposed partial report remains a
separate stage under FR-036, without requiring a second parser, model authority
or incidental wire format.

The standard input was inspected at
`d7483f3d0abe7e71614f73ee18eb51a677ebe3d8` in
`quire-specification`: `proposals/quire-v1/shared-grammar.md`,
`package-contract.md` and `definitions/edition.md`. Its source/static/runtime
boundaries support the controls above. Standard/producer acceptance remains an
explicit prerequisite, and later assurance remains separate. No build or runtime
test was run for this review.

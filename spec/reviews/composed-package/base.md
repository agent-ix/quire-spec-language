---
id: SR-300
title: "Composed compiler admission PR readiness review"
type: SpecReview
analysis: base
scope: "Compiler #35: FR-035/036, TC-113–115, IT-009, changed US-001/002, master requirements and TM-003"
review_set: all
evaluated_revision: "fa07b079861286c884e2f44380a3ed8f9508ef86"
correction_revision: "d5047a8"
review_date: "2026-09-10"
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-035, type: reviews }
  - { target: ix://agent-ix/quire-spec-language/FR-036, type: reviews }
---
## Summary

This is the owner-selected QUOIN base-plus-seven PR-readiness review for L2
under [compiler #35](https://github.com/agent-ix/quire-spec-language/issues/35).
The specification extends the existing compiler with typed composed syntax and
multi-unit linking while preserving historical behavior and explicit admission
stages. The bounded findings below are corrected; the packet is ready for
independent acceptance, not a claim that composed admission is implemented.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The master still called the finite-state workflow the system's end-state acceptance after adding L2. Corrected at d5047a8: that scope is historical, with composed acceptance and current ownership stated explicitly. | spec/spec.md sections 8, 12, 15; TM-003 historical-scope text | wrong-requirement |
| FND-002 | medium | TC-113 did not explicitly exercise the unavailable-edition refusal stated by FR-035. Corrected in this readiness packet: step 3 requires the typed header-located error and no successful unit. | FR-035 Behavior; TC-113 step 3 | correct-requirement-no-evidence |

## Review set and dispositions

The existing issue explicitly selects `all`; no AssuranceProfile override was
found in this repository. The review used the installed QUOIN SpecReview catalog
and inspected actual parser, syntax, linking and package boundaries. Shared
standard input was `d7483f3` on standard PR #15, with acceptance still pending.

| Analysis | Artifact | Outcome for this specification |
| --- | --- | --- |
| Base | This document, SR-300 | Two bounded findings corrected as recorded above. |
| Failure domain | [SR-301](failure-domain.md) | No blocking finding. |
| Integrity | [SR-302](integrity.md) | Original package/header edition and accounting findings corrected in FR-036 and TC-114 at d5047a8. |
| Dependency | [SR-303](dependency.md) | No blocking finding; affected standard/producer acceptance remains explicit. |
| Evidence | [SR-304](evidence.md) | Planned strategy reviewed manually; advisor could not parse the installed Quire version banner. No advisor recommendations are claimed. |
| Risk and complexity | [SR-305](risk-complexity.md) | New admission and external-contract risks have named mitigations. |
| Scope boundary | [SR-306](scope-boundary.md) | No blocking finding; downstream family engines and producer implementation retain their owners. |
| EARS conformance | [SR-307](ears-conformance.md) | No blocking finding for the reviewed requirement statements. |

The edition correction requires every source header to agree with the inventory's
single selection and adds independent mixed-edition/header-conflict controls.
The accounting correction declares the version, charging rules and effective
limits before work, distinguishes zero from unlimited, refuses before an
unaffordable charge or overflow, and preserves prior inputs/reports on retry.
It introduces no invented whole-package numeric ceiling.

## Base checklist and claim limits

- IDs are unique in the inspected repository history and changed artifact set.
  Both FRs implement existing user stories; the master and TM-003 index them.
  The stories retain author/model-user intent and two Given/When/Then examples.
- All 14 new ACs have explicit planned matrix rows backed by TC-113–115.
  IT-009 exercises the actual producer boundary. Tests use inspectable types and
  unconditional assertions, with contrary cases for grammar, identity, scope,
  dependencies, limits, unsupported requests and historical compatibility.
- Inputs, outputs and failures separate source recognition, namespace closure,
  dependency binding, later checking and execution. No parser success or linked
  syntax becomes a checked body; a missing requested entry cannot disappear from
  an aggregate success claim. No new command/wire error strings are standardized
  accidentally by this specification.
- The approved choice is one native composed language using the existing parser
  and producer contracts. This scope introduces no alternate language, model
  authority, network discovery, mutable evaluation or concurrency mechanism.
  Performance is bounded work with explicit exhaustion, not an unmeasured latency
  promise. Later family engines and composed wire formats keep their own tickets.

The cases remain planned. Quire grammar validation is structural evidence only;
historical tested rows are not composed conformance. No Rust implementation,
Cargo test or hosted workflow was run for this documentation change. Independent
review acceptance and the affected shared contracts remain required before
dependent implementation; later assurance is not claimed complete here.

Final scoped validation covers the ten changed typed specification artifacts
and all eight reviews: **18/18 grammar-clean, zero grammar findings**.
The change also passes `git diff --check`.

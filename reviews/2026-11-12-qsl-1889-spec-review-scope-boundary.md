---
id: SR-202611121441563
title: "scope-boundary review of PR2688 ADR-036 engine placement"
type: SpecReview
analysis: scope-boundary
scope: "agent-ix/quire-spec-language; spec/decisions/ADR-036-protocol-local-projection-and-realizability-engine.md PA-1 to PA-4, PA-6, PA-22; PR2688 234fdec9bc4e24b543b6e63ec461c6b53a209f42"
review_set: subset
evaluated_revision: "234fdec9bc4e24b543b6e63ec461c6b53a209f42"
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-036
    type: reviews
---

## Summary

Ticket: QSL-1889. Placement is consistent with the repository's layering: qsl-analyze already has normal dependencies on qsl-replay, qsl-eval and qsl-package; qsl-replay already depends on qsl-eval and qsl-package; arch-lint `qualified_core` puts qsl-replay in the core and qsl-analyze above it, so analyze to replay is allowed and no core crate gains an edge upward. Two boundary gaps remain on the read surface and on the unsupported mapping.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | PA-6 says channel premises (ordering with FIFO key expression, overflow policy) are populated through the PA-3 accessors, but `protocol_view` has no ordering, FIFO-key or overflow accessor today (only `channel_endpoints` with message type, `delivery`, `queue_capacity`). These are FR-054 key members, so slice 2 carries an FR-519 amendment that quire-protocol also imports; name it in the slice instead of "any missing accessor" | ADR-036:111 |
| FND-002 | low | PA-22's unsupported list does not map qsl-eval's existing refusals (ADR-010 G7: protocol captures, `activation on each` with compensation, unordered channel with head/tail loss selector, unenumerable step binder) that PA-13/PA-18 will hit; say they surface as `unsupported` with the qsl-eval cause | ADR-036:147 |

## Verdict

Placement (PA-1, PA-2, PA-4) is sound and arch-lint-consistent. FND-001 should be fixed in the ADR text; FND-002 is a nit.

## Scope and evidence

Checked qsl-analyze, qsl-replay, qsl-package, qsl-eval Cargo.toml [dependencies]; tools/arch-lint/qualified_core.rs CORE_CRATES and QSL_ABOVE_CORE; qsl-package/src/protocol_view.rs public accessors.

- PA-1 (ADR-036): The engine is a new module protocol_analysis of the layer-A crate qsl-analyze, beside model_check. No new crate and no new layer edge.
- PA-2 (ADR-036): The key, bounds, premises, outcome, witness and counterexample types, the settlement map and the independent witness checker live in qsl-replay module protocol_analysis, as ADR-028 SP-2 places EN-5's types.
- PA-3 (ADR-036): The engine reads the checked protocol only through qsl_package::protocol_view (FR-519).

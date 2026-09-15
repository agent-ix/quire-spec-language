---
id: SR-441
title: "Code and Rust review of distinct compiled-protocol v1 role authorities"
type: SpecReview
analysis: code-review
scope: "quire-spec-language#114; Plan-009/Task-044; FR-042; TC-121; examples/protocol-handoff; artifacts/compiled-protocol-v1"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: reviews
---

## Summary

`/rust-review` examined the complete QSL #114 delta at PR readiness. The
version-1 producer now assigns `Service` and `Provider` distinct admitted object
authorities while retaining `Service` as the sole role authorized for
`Workflow::apply`. The existing Rust producer regenerated the complete
version-1 handoff and checksum inventory. No reader, wire, path or handoff type
changed.

## Verdict

**CONDITIONAL PASS** — all findings are resolved. Merge remains conditioned on
the repository's serial local gate over one exact committed head.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Resolved: the first consumer-focused control stopped after strict QSL admission, which could not detect the exact failure Protocol found. It now inspects the admitted `Flow` and requires exactly two named roles with unequal owner-published model coordinates. | `tests/published_protocol_v1.rs`; Protocol #11; QSL #114 |
| FND-002 | medium | Resolved: changing the shared workflow/model inputs would leave the committed `/2` handoff unchanged while silently changing its regeneration recipe. The historical `/2` inputs are now explicit frozen files, and the default `/2` inventory test proves they match the committed selection. | `examples/protocol-handoff/*-v2-frozen.*`; `tests/compiled_protocol_v2.rs` |

## Rust review

- New executable behavior remains Rust. No `unsafe`, parser, subprocess,
  network path, callback, shared mutable state, unchecked numeric conversion or
  approximate authority rule was introduced.
- The producer still performs the established bounded parse, link, check,
  family admission, canonical emission and strict read before publishing. The
  regenerated package changes only because its authored model/source and actual
  producer executable changed.
- `Service` retains the `Workflow` object authority required by attempt and
  compensation operation ownership. `Provider` receives a separate admitted
  `Provider` object and reference universe; no Protocol-owned rule is copied
  into QSL.
- The new assertion consumes QSL's own admitted `wire::Role` records. It does
  not import Protocol, mirror the linker or infer authority from names.
- The committed `/2` directory, synthetic compatibility fixture,
  `resources/native-v1/`, Cargo dependencies and tags have no diff. The frozen
  v2 inputs preserve their original AGPL-3.0-only notices and exact bytes.
- Panics added by this change are test-only assertions over committed fixture
  invariants. Producer I/O and semantic failures retain the existing typed
  `Error` boundary.

## Focused evidence

| Control | Result |
| --- | --- |
| Protocol static link before correction | exact located `DuplicateRoleAuthority` refusal for Provider at source 3 span 916..945 |
| regenerated `/1` producer and public strict-reader replay | pass |
| committed `/1` role-authority distinction | pass: Service `(0,13)`, Provider `(0,12)` |
| committed `/1` checksum/path inventory | pass |
| preserved `/2` historical model and workflow inputs | pass |
| scoped Quire validation | pass; inherited duplicate-module warnings only |

The exact committed-head Cargo gate remains the final merge condition.

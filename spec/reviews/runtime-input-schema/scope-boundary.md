---
id: SR-282
title: "Scope review of the runtime input schema"
type: SpecReview
analysis: scope-boundary
scope: "FR-024-AC-5 at f4679ef"
review_set: all
---
## Summary

Agent A owns the schema as a compiler artifact. Rust runtime records and readers
continue to own wire decoding and admission; caller schema validation is optional.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings | - |

## Allocation and dependencies

| Requirement | Owner | Class |
| --- | --- | --- |
| FR-024-AC-5 | Native compiler runtime artifact boundary | Core |

Caller → local structural schema → caller feedback; original bytes → Rust reader
→ constructor → model-aware runtime. Schema feedback cannot replace any latter
stage. Existing Serde, IR identifier constructors and jsonschema behavior are
exercised through TC-099/100; arbitrary consumer parsers and validators are
external assumptions, with precision limitations disclosed. No B/C/TL/Filament
producer change, alternate source language, emitter qualification or foreign
executable dependency is introduced. Hosted CI remains dispatch-only.

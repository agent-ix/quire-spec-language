---
id: SR-280
title: "Evidence review of the runtime input schema"
type: SpecReview
analysis: evidence
scope: "New FR-024-AC-5 obligation at f4679ef"
review_set: all
---
## Summary

`quoin advise --json` failed CLI-version detection despite installed Quire
0.31.0. The current method catalog was read; the choice below is author judgment,
not an advisor recommendation. Existing AC-1–4 methods remain unchanged.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Retain Test for AC-5 using catalog integration-testing and negative-abuse-testing, both evidence kind Integration; advisor unavailable. | FR-024-AC-5; TC-099; TC-100 |

## Suite and limits

The existing runtime_reading Rust integration target now compiles the local
schema, validates real producer output and exercises actual reader refusals.
Four new tests cover required field mutations, scalar controls, all variants,
null and duplicates. Both feature lanes pass. No evidence-kind suite file exists;
the executable integration target supplies the selected catalog kind directly.
Broader generated-input, fuzz and mutation qualification belongs to later
assurance; these finite controls do not claim exhaustive JSON conformance.
Loom would not address this synchronous immutable schema/reader boundary.

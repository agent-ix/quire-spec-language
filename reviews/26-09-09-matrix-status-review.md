---
id: SR-275
title: "Compiler matrix status consumer review"
type: SpecReview
analysis: base
scope: "compiler #28; TM-001–007 at 7619105; docs/matrix-status.md"
review_set: subset
---
## Summary

Applied the QUOIN spec-matrix checks to the existing, unchanged requirements
and matrices at PR readiness. The shared contract requires both authored header
names; the consumer correction is explicit selection of the already-owned
engine/module stack, not a local schema or table migration.

## Verdict

**CONDITIONAL** — consumer compatibility and false-completion detection are
demonstrated with the exact candidate stack; its shared module PRs and installed
adoption remain open. No new runtime or requirement semantics were authored.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The installed CLI rejects the new declaration and ambient coverage still cannot classify the seven functional columns. Candidate-stack results do not resolve installed adoption or authorize producer merges. | compiler #28; process PR85; ISO PR37 |

## Evidence

The clean CLI ff638b9 embeds clean engine d3bc2ba; its executable SHA-256 is
bb0bd39281508b4148dda05fee920e57f9a0af26ffd6b96536c5758ebafff089.
Exact clean module checkouts are process e6ea515 and ISO a60ee12. The full
revision identities and repeatable commands are in
[matrix-status.md](../docs/matrix-status.md).

Real coverage: 325/329, zero status-column diagnostics, zero status lies.
The catalog's no_symbol_rows exempts TC-010 (Manual) and FR-017-AC-2 (Inspection);
unbacked-row counts must be interpreted alongside those exemptions. StR-001's
two demonstration targets also remain unbacked in the source rollup; their
demonstration evidence is not discharged by this check. The seven
matrices retain all authored headers, row references and statuses. TC-094's
generated activation remains deferred despite its existing trace binding.

Seven injected functional completion claims were detected, with report exit 0
and strict exit 1 producing identical JSON. An independent unbacked-criterion
control also produced the expected test-summary Status lie. These are manual
observations through the actual Rust CLI, not new automated helper programs or
a qualification of the entire engine. No Java/Node/Python checks were added.

No requirement, source, test, dependency or workflow changed. Existing code
verification remains the 3c6a0e6 runs recorded in SR-273; Cargo was not rerun for
this documentation correction. Old review records retain their original tool
limitations. No new all-set requirement review or semantic gap review is claimed.
Only the affected interpretation and this PR-readiness review are added.

---
id: SR-329
title: "Base requirement review of the compiled protocol artifact contract"
type: SpecReview
analysis: base
scope: "FR-042; TC-121; US-004; spec/spec.md; TM-003 (spec/model-linking/tests.md); docs/compiled-protocol-v1.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: references
  - target: ix://agent-ix/quire-spec-language/US-004
    type: references
---

## Summary

Base checklist review of the new FR-042/TC-121/US-004 additions and their
normative wire contract at 51506ed. IDs, cross-references and the six coverage
rules hold: every FR-042 AC has TC-121, TC-121's ten procedure groups map one to
one onto them, US-004 and spec.md gain matching `exercises`/`contains` edges, and
TM-003 keeps all ten FR-042 rows at 🚧 Planned. The contract is unusually precise
about identity domains, ordering and limits. Three interoperability gaps remain:
no typed refusal vocabulary, an underdetermined type-table indexing order and an
ambiguous `U` bound.

## Verdict

**CONDITIONAL** — no high finding; three medium items block a second
implementation reproducing the encoding from the documents alone.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-042 and the wire contract require "typed discriminating causes" but publish no refusal vocabulary; `Invalid`/`Unsupported`/`Dimension` exist only in Rust | spec/functional/FR-042-publish-compiled-protocol-artifacts.md:125; src/protocol_artifact/mod.rs:135 | missing-requirement |
| FND-002 | medium | Type first-occurrence indexing is specified only as "during this declaration/value order"; the reader also indexes binder and binding-requirement types before value types | docs/compiled-protocol-v1.md:135; src/protocol_artifact/validate.rs:231 | wrong-requirement |
| FND-003 | medium | `U` is specified as "a bare JSON integer in 0..1,048,576"; reader, writer and span checks all admit 1,048,576 inclusive | docs/compiled-protocol-v1.md:55; src/protocol_artifact/decode.rs:59 | wrong-requirement |
| FND-004 | low | TC-117/TC-121 rows are appended after TC-119/TC-120 in TM-003's L2 table, breaking the checklist's sequential-ID expectation | spec/model-linking/tests.md:110-111 | missing-requirement |
| FND-005 | low | FR-042 carries no `FR-042-OPT-*`/`FR-042-CON-*` sections and TC-121 states no Type/Priority in the document; both follow existing FR-040/FR-041 and TC-120 precedent and live in TM-003 instead | spec/functional/FR-042-publish-compiled-protocol-artifacts.md:34 | missing-requirement |

## Checklist results

ID format and uniqueness: `US-004`, `FR-038`, `FR-042`, `TC-117`, `TC-121` and
`FR-042-AC-1..10` all conform; no duplicates. Cross-referencing: FR-042
`implements` US-004, depends on FR-036/038/040/041 plus the accepted standard
FRs, and references `quire-protocol` FR-001/IT-001; US-004 gained reciprocal
`exercises` edges; TC-121 `verifies` FR-042; all relative links resolve.

FR quality: Description, Inputs, Outputs, Behavior, Acceptance Criteria and
Dependencies are present and specific. Inputs separate source inventory,
producer, baseline, contract, dependency bytes and admitted producer views, and
state explicitly that source/profile acceptance never comes from a payload flag.
Performance targets are concrete — thirteen named dimensions with defaults and
hard maxima, charge-before-work, clamping and fresh-retry semantics. Security is
addressed directly: the seal is external, digest integrity is not authenticity,
and a resealed mutant is refused. Error conditions are described in prose but
carry no stable codes (FND-001).

Coverage (six rules): every AC has TC-121; constraint boundaries are named per
limit at zero/exact/one-short in step 9; error paths carry typed causes in steps
3–8; state transitions appear as the repeat/await/choice edge expansion; edge
cases include equal-time observations, N=0 repeats and supplementary Unicode.
Option permutation does not apply — this version declares `optional: []`.

## Contract fidelity spot checks

The single `ix.artifact-ref/3-draft` `linked-package` seal over complete
canonical bytes, the absent self-digest, `canonicalIdentity:null`, the closed
`Number`/`Integer` tagged objects, CompactFormatter-only spelling with no
normalization, the member-order table and the structural-edge expansion table are
all internally consistent and match the delivered reader. FND-002 and FND-003 are
the only two places where a second producer could diverge while still satisfying
the prose.

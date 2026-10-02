---
id: SR-781
title: "QSL-320 gap analysis of PR 518 (ADR-013 replay-type ownership vs qsl-replay code)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@4a4f1c286339abf7e86969800989f2da295d3467; spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md (O-24, O-25, C-09, C-11, OQ-H, TK-04); qsl-replay/src/witness.rs; qsl-replay/src/witness/frame.rs; qsl-replay/src/identity.rs; qsl-replay/src/lib.rs; qsl-replay/src/proof_result.rs; agent-ix/quire-contract-ir spec FR-031 (context only)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
---
## Summary

Ticket: QSL-320. The ticket asks for ADR-013's O-24 to O-27 and TK-04 to name
QSL as owner of the replay types, as a coherent rewrite rather than a word
swap, without touching QSpec. Each ask is delivered: O-24 (terminal record,
category map, envelope), O-25 Owner and Carrier, O-26, O-27, and TK-04
(narrowed to the IR work that remains). QSpec is untouched.

Gate claim checked: no test, build script or Makefile target reads
`spec/decisions/` or ADR-013. A search for `include_str!`, `include_bytes!`,
`read_to_string` and `fs::read` over every `.rs` file found embeds of FR-092,
Plan-013 tasks and `docs/compiled-protocol-v*.md` only. Makefile, `.toml`,
`.sh`, `.py` and YAML files mention ADR-013 only in comments. `make ci` is not
required for this diff. Comments that cite `ADR-013:113` now point one line
off, since the Context bullet grew by a line; no code reads them.

Code against the ADR text: the four mismatches the coder reported were each
checked against the code at the reviewed sha. Findings follow. They are code
or cross-repository drift, not defects in this PR's text, and should not be
fixed in this PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `Witness::decode` does not match ADR-013 O-25 (or AD-016). The code is `decode(&self, order: &[String]) -> Result<Vec<i64>, DecodeRefusal>`: a name-ordered projection to `i64`. The ADR says `decode(&[WitnessBinding])` yields typed `WitnessValue`s joined to parameters by `WireNodeId`, with lossless widening in CG (C-11). This PR now names that decode as QSL's, so the gap is QSL's to close. Needs a follow-up ticket. | qsl-replay/src/witness.rs:157 |
| FND-002 | low | `ReplaySource::Input` holds `Vec<CanonicalAssignment>` with `value: i64` only. The ADR's `Input(values)` holds a counterexample's canonical assignments with no integer-only limit. Fold this into the FND-001 follow-up ticket. | qsl-replay/src/witness.rs:269-293 |
| FND-003 | low | Doc comments contradict OQ-H. `witness.rs:6-14` says `Witness` "mirrors IR's own `Witness`" and is "QSL's own copy". `identity.rs:5-19` and `lib.rs:18-24` call `ObligationIdentity` "provisional, not canonical" and say it waits for consolidation into a canonical home. Under OQ-H, QSL's type is canonical. The `QualifiedName` part of those comments may still be accurate. `witness/frame.rs:14-16` builds over "IR's `WitnessBinding`" (see SR-780 FND-004). Fold this into the same follow-up ticket. | qsl-replay/src/witness.rs:6-14 |
| FND-004 | low | C-09 says "one exhaustive map", but IR FR-031 (as amended by IR PR #202) leaves `Unavailable` and a non-vacuous `Inconclusive` unmapped, returning a typed absence (FR-031 lines 77-79, FR-031-AC-5), until IR AD-001 OQ-3 is answered. QC-9 already tracks the missing FR-331 result value. ADR-013 states the target design, so this is IR-side drift covered by QC-9 and IR's OQ-3. It needs no QSL ticket. | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:961 |

## Verdict

Every item the ticket asks for is delivered, and the gate claim holds. The
four reported code/ADR mismatches are real and not overstated. Three of them
(FND-001 to FND-003) belong in one QSL follow-up ticket to bring `qsl-replay`'s
`decode`, `Input` assignments and doc comments into line with O-25 and OQ-H.
FND-004 is tracked on the IR side and by QC-9. None of them blocks this PR.

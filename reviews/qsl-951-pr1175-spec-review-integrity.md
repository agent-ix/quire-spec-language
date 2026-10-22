---
id: SR-13741
title: 'Spec review of PR #1175: ix-trace-rs owns the string_edge marker, integrity'
type: SpecReview
analysis: integrity
scope: 'agent-ix/quire-spec-language@76a302455; git diff origin/main...76a302455 (PR #1175, branch qsl-951-trace-marker): ADR-011 X-10, ADR-012 section 9 and work table, FR-064, TC-162; read against agent-ix/ix-trace-rs main (Cargo.toml, README.md, src/lib.rs, tags v0.1.0/v0.1.1), the QSL workspace manifests and qsl-string-edge/src/detector.rs at origin/main, IR FR-028 on quire-contract-ir main, and the proposal in IR-212 comment 855ebf10 and IR-9 comment a884990a; ticket QSL-951'
review_set: subset
date: '2026-10-22'
relationships:
- target: ix://agent-ix/quire-spec-language/ADR-011
  type: reviews
- target: ix://agent-ix/quire-spec-language/ADR-012
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-064
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-162
  type: reviews
- target: ix://agent-ix/quire-spec-language/ADR-032
  type: reviews
---

## Summary

Ticket: QSL-951. PR quire-spec-language#1175, head 76a302455. Integrity lens: whether FR-064, TC-162, ADR-012 section 9 and its work table, and ADR-011 X-10 agree with each other, with IR FR-028, with the IR proposal, and with the real ix-trace-rs crate.

These agree: the marker contract (no argument, identity expansion, item kept on error, no file or crate exemption) in FR-064 and the IR proposal; ADR-012 section 9 and its work table (#214 builds the scan, the attribute is ix-trace-rs's) with FR-064's 'QSL SHALL provide the scan'; ADR-011 X-10 swapping `qsl-attrs` for `ix-trace-rs` in qsl-replay's dependency list; TC-162 steps 13 to 16 and their expected results with AC-12 to AC-15; and the consumer git-dependency route with IR FR-028-AC-3, which forbids only crates from the QSL repository. ix-trace-rs's manifest has an empty `[dependencies]`, so the 'no normal dependency' property holds today. The stale Dependencies sentence and the false ADR claim in AC-15 are recorded in the base review (SR-13740 FND-002, FND-003).

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-064 owner statement | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md | examined |
| FR-064 marker no argument | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md | examined |
| FR-064 marker identity | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md | examined |
| FR-064 marker crate deps | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md | examined |
| FR-064 import rule | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md | examined |
| FR-064 consumer marker | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md | examined |
| FR-064 Dependencies | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md | examined |
| FR-064-AC-13 | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md | examined |
| FR-064-AC-15 | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md | examined |
| ADR-012 section 9 marker | spec/decisions/ADR-012-semantic-family-extension-contracts.md | examined |
| ADR-012 work table #214 | spec/decisions/ADR-012-semantic-family-extension-contracts.md | examined |
| ADR-011 X-10 | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md | examined |
| ADR-011 crate map | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md | examined |
| ADR-032 R-3 | spec/decisions/ADR-032-checked-input-and-duplicate-canonical-type-gates.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-064 sets SHALL obligations on a crate QSL does not own. Examples: '`ix-trace-rs` SHALL have no normal dependency, no dependency on QSL ...', and the marker 'SHALL take no argument; a call with any argument SHALL be a compile error that retains the annotated item'. AC-13 also fixes ix-trace-rs's diagnostic text. No QSL change can meet or break these. A QSL coder cannot implement them, and ix-trace-rs's own spec/, which is where they belong, does not hold them. Recast them as the contract QSL depends on, for example 'QSL relies on ix-trace-rs providing ...' under Dependencies, and have the ix-trace-rs ticket from SR-13740 FND-001 own them as ix-trace-rs requirements. AC-13 and AC-15 then remain QSL-side conformance checks of that contract. | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md:72-74; spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md:64-66; spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md:67-69; spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md:240; spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md:242 |
| FND-002 | low | ADR-032 R-3 and its rejected alternative 'A `#[canonical]` attribute from `qsl-attrs`' still treat `qsl-attrs` as QSL's marker-attribute crate. Once this PR's AC-14 holds, no `qsl-attrs` crate exists. The reasoning still holds for ix-trace-rs, since a marker crate would still add a dependency to K and SV. A reader of the decision record, though, will look for a crate that is gone. Name the marker crate generically ('a marker attribute crate, now ix-trace-rs'). Or leave the text as a dated rejected alternative if decision records are meant to keep their wording. | spec/decisions/ADR-032-checked-input-and-duplicate-canonical-type-gates.md:141 |

## Verdict

Mergeable on this lens once FND-001 is resolved: QSL obligations should be stated as dependencies on ix-trace-rs, not as SHALLs on it. FND-002 is low.

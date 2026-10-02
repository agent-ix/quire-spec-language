---
id: SR-760
title: "Spec review of FR-064 Status and ADR-012 §9 clock row (PR 501)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@8b5606b9d2f5b92c14837142f6c0ac99e82b8dd2; spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md; spec/decisions/ADR-012-semantic-family-extension-contracts.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-064
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: reviews
---
## Summary

Tickets: QSL-287, QSL-288. PR: quire-spec-language#501 at 8b5606b9. A
spec-review (integrity: does the Status text state what is) of the two spec
edits: FR-064's Status moves from Partial to Implemented, and ADR-012 §9's
`"clock:"` row moves from Partly done to Done. The AC backing list in FR-064
matches the tests in `xtask/src/string_edge.rs`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-064 Status overstates coverage. It says the string-value clause "is built", but Behavior asks for a comparison between a string and any other `&str`/`String` value, and any `match` whose scrutinee is a string. The detector reports literals and same-crate `const NAME: &str` only. A comparison of two bindings (for example `adapter.observation_contract_revision != offered.observation_contract_revision`) and a cross-crate const are not seen. The xtask doc says so, but the Status does not. Either state the residual in Status or narrow Behavior. Status also lists `operation_role` as a typed conversion; it is a mark (SR-758 FND-005). | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md:57-59; spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md:124-151; src/state/evaluation.rs:952 |
| FND-002 | low | ADR-012 §9's `"clock:"` row keeps the target "the prefix is parsed once, at lexing" beside "Done", while the prefix is now parsed at v2 emission, at the v2 consistency check and at v1 admission. The row text should say what was built: a typed v2 field whose value is still derived from the prefix. See SR-758 FND-003. | spec/decisions/ADR-012-semantic-family-extension-contracts.md:858 |

## Verdict

Minor spec corrections needed. Both Status edits should state what is built
and what residual remains, not the full target.

---
id: SR-759
title: "QSL-287/QSL-288/QSL-294 gap analysis of PR 501"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md; spec/functional/FR-050-publish-authenticated-temporal-artifacts.md; spec/decisions/ADR-012-semantic-family-extension-contracts.md; spec/functional/FR-031-run-extracted-native-source.md; spec/functional/FR-100-run-a-named-function-through-the-spine.md; spec/functional/FR-108-run-the-configversion-spine-corpus.md; spec/functional/FR-109-run-a-state-clause-through-the-spine.md; spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md; src/command.rs; src/command/extraction.rs; qsl-replay/src/spine/clause.rs; xtask/src/string_edge.rs; tests/it/handoff_writer.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-064
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-050
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-109
    type: reviews
---
## Summary

Tickets: QSL-287, QSL-288 (closed by the PR), QSL-294 (left open, claimed
blocked). PR: quire-spec-language#501. A manual check of the
tickets' done-when criteria against spec text and tests. The QSL-294 claim
was checked against the spec text and the code, not against the PR body.

QSL-287 done-when: the detector resolves same-crate named consts (backed by
`named_string_consts_are_resolved_like_literals`). `cargo xtask string-edge`
is clean in `make ci`. TC-162 has a real named-const site (`operation_role`).
"No mark launders dispatch" does not hold for `operation_role` (SR-758
FND-005). QSL-288: the typed field and the refusal are built and traced
(TC-138/FR-050-AC-1), but "spec first if it changes the wire schema" was not
done (FND-002 here, SR-758 FND-001).

QSL-294, checked independently:

- FR-031 does not say "native-only". It specifies native-run/1 extraction
  and refuses extraction combined with selected-package execution or
  compile/lower export. The "run-only, native-only" wording the PR quotes is
  a code comment in `src/command.rs`, not spec text.
- FR-100:242 refuses `extraction` on a `1-draft` native-run/1 CLI request.
  That is the CLI edge only.
- FR-108 Behavior ("each case's unit SHALL also be embedded in a Markdown
  fence and run through the I3 adapter") and FR-108-AC-5 require the
  `1-draft` spine (`run_clause`) to run Markdown-extracted units. So do FR-109
  Inputs ("or an I3 extracted source (ADR-011 I3, `qsl-source`...)") and
  Outputs.
- QSL-295 (PR #502, open) is building exactly that path: `run_clause`
  accepting an I3 extracted source. #502's diff feeds the extracted body to
  compile but carries no `SourceMap`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | QSL-294's "genuinely blocked, spec-level scope decision" claim does not hold. The spec already puts I3 extraction into the `1-draft` spine: FR-108 Behavior and FR-108-AC-5, and FR-109 Inputs/Outputs. FR-031 does not forbid it; the quoted "native-only" text is a code comment, not FR-031. QSL-295 (PR #502) is building the `run_clause` extracted-source input now. QSL-294 is therefore a real code task sequenced after QSL-295: thread `ExtractedSource`'s document map into the `PackageDeclarations` embedding on the `run_clause`/compile path. Its correct state is blocked-by QSL-295, not spec-blocked. | spec/functional/FR-108-run-the-configversion-spine-corpus.md:116-120; spec/functional/FR-108-run-the-configversion-spine-corpus.md:130; spec/functional/FR-109-run-a-state-clause-through-the-spine.md:48-50; spec/functional/FR-109-run-a-state-clause-through-the-spine.md:81-82; spec/functional/FR-031-run-extracted-native-source.md:55-62; src/command.rs:803-806 |
| FND-002 | medium | QSL-288 changes the closed v2 wire schema without the spec-side update the ticket required. `docs/compiled-protocol-v2.md` (FR-050's normative wire text) is unchanged, and the new refusal code is not listed. See SR-758 FND-001. | docs/compiled-protocol-v2.md:44; docs/compiled-protocol-v2.md:76-80 |
| FND-003 | medium | FR-064 Status says "Implemented" and "the Behavior clause on string-value comparisons is built". But Behavior covers a comparison with any `&str`/`String` value and any `match` on a string scrutinee, and the detector covers only literals and same-crate consts. The xtask module doc admits that residual. The Status should state it; see SR-760 FND-001. | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md:57-59; spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md:124-135; xtask/src/string_edge.rs:19-24 |

## Verdict

QSL-287 and QSL-288 are built but not complete against their done-when
criteria (FND-002, FND-003; SR-758 FND-005). QSL-294's blocked claim is
rejected: it is sequenced behind QSL-295 (#502), not blocked on a spec
decision, and real work would have been skipped had the ticket been parked
as spec-blocked (FND-001).

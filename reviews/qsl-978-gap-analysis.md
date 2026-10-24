---
id: SR-12349
title: "Gap analysis of QSL-978 published extension definitions in the checked-package v2 fixture reader (PR #1267)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@6f03809f4b44576a162090839eb63af925fe3db3; git diff origin/main...HEAD over qsl-package/src/checked_v2/tests.rs and qsl-semantics/src/library/package_identity.rs"
review_set: subset
---
# SR-12349: Gap analysis of QSL-978 published extension definitions in the checked-package v2 fixture reader (PR #1267)

## Summary

Ticket: QSL-978. PR quire-spec-language#1267 at head 6f03809f4. Plan completion: not assessed. This is a manual AC-to-test check: FR-469 is a QSpec requirement, so QSL's `quire matrix` does not carry it. The baseline is QSpec PR #365 head 02cf63fd, read from a worktree at that head.

Units examined: FR-469 "Published definitions", FR-469-AC-3, FR-469-AC-9, FR-469-AC-10, TC-951 X05, X18 and X19, and the ticket's "fixture and production read paths" scope.

Measured:

- With `QSPEC_DIR` at QSpec #365, XD-01 to XD-04, c14, i2 and fr340 pass.
- With `QSPEC_DIR` at a copy that omits only `extension-definitions.json`, the reads that need the definitions fail. So the loader does real work and a missing file is loud (measured: c14, i2 and XD-01 each fail with `unknown_required_capability` at the extension node. IR itself reports the expected locus `7aa76a0f...` and the `document_pointer`, so IR behaves correctly. The QSL test just never asserts those two fields).
- The withheld control refuses `unknown_required_capability` with cause `null` at `/semantic_graph/nodes/{5,34}/body/definition`.

At 02cf63fd, QSpec split the old FR-469-AC-9 (b0ad3e9d, the head the ticket cites) into AC-9 and AC-10:

- AC-9 is file conformance: the file meets its schema, entries come in reference order, and each production pairs with a node entry.
- AC-10 is the vectors.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Wrong trace. The test is tagged `FR-469-AC-9`, but at QSpec #365 head 02cf63fd AC-9 (TC-951 X18) is file conformance: the file meets `extension-definitions.schema.json`, entries come in reference order, and each production is paired with a node entry. The test checks none of that. What it does test is FR-469-AC-10 (X19), which it does not tag. Either retag it to AC-10 and add an X18 test (schema validation with the root crate's `jsonschema` dev-dependency, the ordering check and the production/node pairing check), or drop the AC-9 claim if QSpec owns X18. | qsl-package/src/checked_v2/tests.rs:2794 |
| FND-002 | medium | The `FR-469-AC-3` tag over-claims. AC-3 also requires that the lock, source map and node are unchanged, and that a definition missing from `definition_selections` refuses `missing_declaration`/`missing-selection` before the held check (X05, X06). No test in this PR covers X06, and the locus half of X05 is unasserted (SR-12348 FND-002). | qsl-package/src/checked_v2/tests.rs:2794 |
| FND-003 | low | No unit test in `qsl-semantics` for an `extension`-tagged projection node. The `NODE_TAGS` addition is exercised only through XD-03/04 and c14, which are skipped when `QSPEC_DIR` is unset. A default `cargo test` therefore never runs the changed line. | qsl-semantics/src/library/package_identity.rs:303 |
| FND-004 | low | The ticket scopes the fix to the "fixture and production read paths". The PR changes only the fixture path. Its stated reason is that QSL emits no extension nodes: `ir_tag` has no `Extension` arm, so that much is true. But the production `checked_v2` reader of an imported package that carries extension nodes still holds no definitions and refuses. That behaviour is spec-correct for a reader without the extension. The production half needs either a follow-up ticket or an explicit rescope on QSL-978. | qsl-package/src/checked_v2.rs |

## Verdict

The behaviour is correct and the loader is not lenient in a way that hides failure. It is mergeable once FND-001 and FND-002 are fixed: retag to AC-10, add or disclaim X18, and assert the locus/document_pointer from SR-12348 FND-002. FND-003 and FND-004 are low.

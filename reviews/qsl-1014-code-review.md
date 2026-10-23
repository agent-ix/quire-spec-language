---
id: SR-16276
title: "code review of QSL-1014 checked_v2 reader admits the hybrid node tag (QSL PR #1381)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@89ebc48e365ad269bcb223ccde3b00ff474effd0; Cargo.toml, Cargo.lock, qsl-package/Cargo.toml, qsl-replay/Cargo.toml, qsl-semantics/src/library/package_identity.rs, qsl-package/src/checked_v2/tests.rs, qsl-package/src/emit/tests/admission_corpus.rs"
review_set: subset
---
# SR-16276: code review of QSL-1014 checked_v2 reader admits the hybrid node tag (QSL PR #1381)

## Summary

Ticket: QSL-1014. One `code-review` pass with its `rust-review` lane over
`git diff origin/main...89ebc48e3` (7 files, +74/-18). Spec oracles: QSL FR-447-AC-13..16 at
QSL #1365 head, QSpec FR-481 and FR-322-AC-13 at the QSpec #415 checkout (da44f70b).

Measured, with `QSPEC_DIR` set to the QSpec #415 checkout:

- `cargo test -p qsl-semantics --lib node_tag`: 2 passed.
- `cargo test -p qsl-package --features test-support --lib -- conformance node_tags every_emitted_node_family`: 10 passed,
  including `conformance_hybrid_nodes_are_admitted_and_a_swapped_model_body_is_refused`,
  `node_tags_equal_irs_checked_node_tag_wire_spellings` and
  `every_emitted_node_family_is_admitted_at_its_package_id`.
- Vacuity check: with `"hybrid"` removed from `NODE_TAGS` in a throwaway copy, all three new or
  affected tests fail. The conformance test fails with
  `Refused(Structural(InvalidPreimage { .. Node { index: 39, defect: NodeTag } }))`, the unit test
  fails its `is_ok` assertion, and the IR parity test fails. None of the tests is vacuous.

Examined, clean:

- `NODE_TAGS` gains `hybrid` and its length goes to 15. That equals IR's `CheckedNodeTag::ALL` at
  the pinned rev, by the parity test, and QSpec FR-322-AC-13's "all 15 closed node tags".
- The conformance test asserts exactly 3 `hybrid` nodes in `positive-all-families.json`, a
  `Verified` read, and `malformed_wire` for the swapped `variables`/`modes` model body. The
  refusal assertion pins the code, so an identity-mismatch refusal would not pass it.
- The `admission_corpus` classification of `CheckedNodeKind::Hybrid(_)` as not emitted follows the
  existing `Extension(_)` pattern and keeps the corpus total. QSL emits no hybrid node.
- The dependency pin changes only the `quire-contract-model` source lines in three manifests and
  the lockfile. It copies or vendors no file. One lockfile entry remains, so the workspace still
  resolves one `quire-contract-model`. It is temporary until IR #528 merges, as the PR body says.
- No production `unwrap`, panic or integer-conversion surface is added. The only production change
  is the constant.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `a_hybrid_tagged_node_passes_the_node_tag_check` is traced `QSpec-FR-481-AC-1`. That AC is about the node QSL FR-447's `Braking` produces: its `declaration`, `variables` classes, `modes` order, `Moving`'s rates and transition. The test sets one projection node's tag to `hybrid` and asserts the tag check passes, so it asserts none of that. The tag puts an emission AC that QSL cannot meet yet (no hybrid lowering, QSL-900) into the matrix as covered. The test's own doc comment names QSpec FR-322-AC-13 (the 15 closed node tags), which is the AC it does back. | qsl-semantics/src/library/package_identity.rs:648 |
| FND-002 | low | `Cargo.lock` also moves `quire-verification-contracts` from `e1ede119` to `0aef3350`. In that bump its dependency on `serde_json_canonicalizer` is replaced by one on `quire-canonical`. The change is unrelated to the hybrid tag or the IR pin, and the PR body says only "lock bumped". Either keep the lock change to `quire-contract-model` (`cargo update -p quire-contract-model`) or state the extra bump in the body. | Cargo.lock:1718 |

## Verdict

Two findings: one medium trace defect and one low lockfile note. No high findings. The code change
is correct and minimal, and the tests are real: each one fails when the tag list is left unchanged.
Merge once FND-001 is fixed. FND-002 can be fixed or stated in the body.

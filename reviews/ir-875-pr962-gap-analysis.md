---
id: SR-12881
title: IR-875 QSL PR 962 gap-analysis
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; qsl-package/src/checked.rs; qsl-package/src/emit.rs; qsl-package/src/emit/tests.rs; spec/functional/FR-087-typestate-and-cross-package-node-key.md; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md; spec/test-cases/TC-416-emission-writes-the-nodes-check-lowered.md"
review_set: subset
---

## Summary

Ticket: IR-875. Planless, diff-scoped gap analysis of agent-ix/quire-spec-language#962 at head 3a67aaa842c305f0caacb34dbbd64dc418963ff1. The PR adds no spec documents and changes no acceptance criteria. Its requirement owner for the memory property is Contract IR (NFR-006 and FR-051, delivered in quire-contract-ir PRs 461, 464 and 467). The QSL side is a producer adoption.

## Examined units

- **FR-087-AC-3** (`examined`): "`library` constructs a `VerifiedPackage` only when the three ADR-011 §4 verified-binding conditions hold, and refuses with a named cause and no partial output on: an unsupported schema version; a recomputed `package_id` that does not lexically equal the declared one; ..." The computed matrix at head binds the new `emitted_package_preserves_the_owned_projection_golden` here, alongside the existing TC-253 binders. The test asserts the positive admission with a recomputed `package_id`, so the binding is not inflation.
- **FR-093-AC-17** (`context_only`): the commit 3a67aaa removed this tag from the new test because the test does not assert `diagnostics.catalog`. Correct.
- **TC-416** (`context_only`): its scope is FR-093 criteria only. The TC-label mismatch is recorded in SR-12880 FND-002.
- **qsl-package/src/emit.rs `emit_package_inner`** (`examined`): reverse-gap check. The changed code is the existing FR-093 emitter, so no unowned symbol was added. No stub, `todo!` or placeholder.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The workspace does not compile under `--all-features` at head (root cause: SR-12880 FND-001). So every feature-gated binder in the computed matrix cannot run: `tests/it/handoff_writer.rs` and `examples/native_protocol_v2_handoff.rs` (FR-050-AC-1, FR-050-AC-4), and the `quire-extraction` modules in `tests/it/spine_run.rs` and `tests/it/config_version_spine.rs` (FR-100-AC-3). Those criteria keep default-lane binders, so none becomes untagged, but part of their evidence is lost until the build is fixed. | src/population_fixtures.rs:460; tests/it/main.rs:69 |

## Coverage

`quire matrix --scope . --format json` at head reports 2488 criteria and 1072 without a binder. No criterion is added or removed by this diff, and none became untagged. The new test is bound to FR-087-AC-3. The optional semantic review (intent to test to code) was skipped. Plan completion: not assessed.

## Verdict

CONDITIONAL: one medium finding, which is resolved by the fix for SR-12880 FND-001. There is no untagged criterion and no unowned or stubbed code in the diff.

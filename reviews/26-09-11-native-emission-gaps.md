---
id: SR-334
title: "Gap analysis of native protocol emission against FR-042 and TC-121"
type: SpecReview
analysis: gap-analysis
scope: "src/protocol_artifact/native/; tests/native_protocol_emission.rs; spec/functional/FR-042-publish-compiled-protocol-artifacts.md; spec/model-linking/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: references
---

## Summary

Gap analysis of the native emission increment at `ae910c0` against FR-042 and the
`spec/model-linking` matrix, using QUOIN `gap-analysis` with `quire coverage`. The
delivered native scope is honestly scoped: every FR-042 matrix row is still
`🚧 Planned`, no status lie exists anywhere in the corpus, and the reconciliation's
six unbacked rows are all inherited from FR-036/FR-017/NFR-005 and untouched by this
change. The optional semantic review was declined by the owner and was not run.

## Verdict

**FAIL** — six unbacked matrix rows remain in the corpus (all inherited, none in the
reviewed scope), and one `high` finding covers newly added emitter behavior with no
executing test. Nothing here indicates a false completion claim for FR-042.

## Delivered scope versus FR-042/TC-121 acceptance

Delivered and exercised end to end from real source: predicate, state, temporal and
protocol families; sequence, parallel/join, check, event, attempt, effect, send,
receive and commit controls; record and scalar channels with FIFO keys and per-send
delivery cardinality; cross-unit callee owners and lexical `let` provenance;
registered semantic revisions independent of source artifact revisions; independent
limits with no partial package.

Not delivered, and correctly declared open in FR-042, `README.md` and
`docs/compiled-protocol-v1.md`: general symbolic choice and visibility proofs,
ordered-query proofs, recovery and compensation admission, unavailable producer
exports, and the actual producer-to-`quire-protocol` handoff. A successful native
emission followed by an independent `read` is compiler-to-reader evidence only; it
establishes nothing about B's consumer implementation or runtime conformance.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | Newly added emitter behavior with no backing test: the Choice and Await lowering, the emitted `Repeat` control and `RepeatProgress` edge, the `Progress::NeedsAuthority` refusal, and every non-literal arm of the closed Boolean interpreter. `tests/native_protocol_emission.rs` is the sole caller of `native::admit`, and it contains no `choice` and no `await`. Not a stub — the code is complete, it is the evidence that is absent. Duplicates SR-333 FND-001 | src/protocol_artifact/native/families.rs:184; src/protocol_artifact/native/families.rs:235; src/protocol_artifact/native/controls.rs:94; src/protocol_artifact/native/controls.rs:140 | correct-requirement-no-evidence |
| FND-002 | medium | Reverse gap. `native::Selections` takes three caller-supplied revision namespaces (`SourceSelection::revision_namespace`, `definition_revision_namespace`, `requirement_revision_namespace`) as public API inputs. FR-042's Inputs section names the source/model/definition/rule/dependency selections but not these namespaces; they are described only in the wire contract. One sentence in FR-042 Inputs would own them | src/protocol_artifact/native/mod.rs:27; src/protocol_artifact/native/mod.rs:45; src/protocol_artifact/native/mod.rs:47; spec/functional/FR-042-publish-compiled-protocol-artifacts.md:42 | missing-requirement |
| FND-003 | medium | Inherited, outside this change. Six unbacked reference rows: TC-115 in `spec/model-linking/tests.md` with FR-036-AC-5/AC-6/AC-8, and TC-010 in `spec/tests.md`. TC-010 and FR-017-AC-2 are exempt by declared method (`Manual`, `Inspection`) and appear in `no_symbol_rows`; the four FR-036 rows are not exempt | spec/model-linking/tests.md:107; spec/functional/FR-036-link-composed-native-packages.md:129; spec/functional/FR-036-link-composed-native-packages.md:130; spec/functional/FR-036-link-composed-native-packages.md:132 | correct-requirement-no-evidence |
| FND-004 | medium | Inherited, outside this change. Twenty untracked symbols: tests in `src/package/encoding/tests.rs` and `tests/package_construction_cases/limits.rs` carry `NFR-007-M-2..M-5` tags that resolve to no declared row | src/package/encoding/tests.rs:67; src/package/encoding/tests.rs:105; src/package/encoding/tests.rs:170; tests/package_construction_cases/limits.rs:55 | wrong-requirement |
| FND-005 | low | FR-042-AC-10 is the one acceptance criterion with no tagged test, which is correct: it requires the real `quire-protocol` public Rust handoff, and FR-042's Dependencies section and `README.md` both state that it remains open. Recorded so the 9/10 figure is not read as drift | spec/functional/FR-042-publish-compiled-protocol-artifacts.md:151 | correct-requirement-no-evidence |

## Coverage

- Reconciliation: `quire coverage --scope /home/peter/dev/worktrees/quire-language-native-emission --json`, quire 0.31.0 (engine 0.46.0), module `spec-artifacts-process`. Engine path, not a grep fallback. Version is ≥ 0.16.0, so the split document/source roots apply.
- Rows backed by a tagged test: 367 / 376 across the bundle; FR-042 acceptance criteria 9 / 10.
- Tasks done: not applicable. No plan bundle targets FR-042, per the owner directive against creating plan bundles for this work.
- Status lies: 0. Every FR-042 and TC-121 row is `🚧 Planned`.
- Unbacked rows: 6, of which 2 are method-exempt (`no_symbol_rows`). All inherited.
- Untracked symbols: 20, all inherited `NFR-007-M-*` tags.
- Diagnostics: 25 declarations selected nothing; none in the reviewed scope.
- New untraced behaviors: 1 (FND-002). Source stubs: 0. Test stubs: 0. Weak or tautological assertions in the new suite: none found; the eleven tests assert exact values, and the reader expectations in `tests/support/native_protocol/mod.rs` are rebuilt from original parser spans and authored mappings rather than read back from the emitted payload.
- Semantic review: skipped, declined by the owner.

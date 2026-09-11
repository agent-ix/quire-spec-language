---
id: SR-337
title: "Gap analysis of native population and reference exports against FR-042 and TC-121"
type: SpecReview
analysis: gap-analysis
scope: "src/protocol_artifact/native/populations.rs; src/protocol_artifact/models/populations.rs; src/protocol_artifact/native/; tests/native_population_emission.rs; spec/functional/FR-042-publish-compiled-protocol-artifacts.md; spec/test-cases/TC-121-publish-compiled-protocol-artifacts.md; spec/model-linking/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: references
---

## Summary

QUOIN `gap-analysis` over the population/reference increment at `8590407`
against `origin/main` `09dafe4`, with a fresh
`quire coverage --scope <worktree> --json`. The new behaviour is owned: FR-042
gained a normative paragraph for population derivation and reader refusal, TC-121
step 4 gained the corresponding controls, and all five new tests carry resolving
`TC-121`/`FR-042-AC-*` tags. The corpus reconciliation is unchanged from SR-334 —
the same six unbacked rows and twenty untracked symbols, all inherited from
FR-036/FR-017/NFR-005/NFR-007 and all outside this change. Two new gaps are
recorded: an unspecified necessity rule behind the reader's requirement check,
and a stale TC-121 evidence narrative. The optional semantic review was declined.

## Verdict

**FAIL** — retained solely by the skill's own rule that any matrix Test Case with
no backing tagged test fails the gate. TC-115 and three FR-036 acceptance rows are
still unbacked, inherited, pre-existing and untouched here. Read as a delivery
signal for this increment: nothing in the reviewed scope is unbacked, there is no
status lie anywhere in the corpus, and the highest new finding is medium. The
increment is mergeable on this axis.

## Delivered scope versus FR-042/TC-121 acceptance

Delivered and exercised from real authored source and a real admitted model:
`population` exports derived from the admitted `ObjectRole` with path
`[record, universe]` and the role's own locus; `reference` exports and the
`Type.reference` triple bound to one role by pointer identity; `Reaches` lowering
with operand origin/type, actual edge field type, exact universe and Boolean
result; population and closure runtime requirements retaining the declaration
owner and the original pre/post/captured observation anchor, with the closure
depending on exactly its population and selecting `ObservationBinding`/`Progress`
against their verified contract dependencies; nested record/reference traversal
through `Populations::require`'s established shape; and an independent read of
the emitted bytes reproducing the package.

Not delivered, and correctly left open: every FR-042 and TC-121 matrix row
remains `🚧 Planned`. The new tests use explicitly synthetic producer and
baseline metadata, so they establish compiler and reader behaviour, not
FR-042-AC-10 or the `quire-protocol` IT-001 handoff. Composed source/result
correspondence has no adopted contract and stays `Unsupported::Feature`;
`ValueOperation::Parent`, relationship, component and endpoint exports still
refuse. Population membership, closure truth and runtime identities are declared
consumer inputs and are neither computed nor required for static linking.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | Inherited, outside this change, and the sole cause of the FAIL. Six unbacked reference rows persist: TC-115 in `spec/model-linking/tests.md` with FR-036-AC-5/AC-6/AC-8, TC-010 in `spec/tests.md` (NFR-005-M-1) and FR-017-AC-2. TC-010 and FR-017-AC-2 are exempt by declared method (`Manual`, `Inspection`) and appear in `no_symbol_rows`; the four FR-036/TC-115 rows are not exempt and belong to FR-036's own delivery. Re-confirmed at this commit, not assumed | spec/model-linking/tests.md:107; spec/functional/FR-036-link-composed-native-packages.md:129; spec/functional/FR-036-link-composed-native-packages.md:130; spec/functional/FR-036-link-composed-native-packages.md:132 | correct-requirement-no-evidence |
| FND-002 | medium | Reverse gap, new with this increment. FR-042's added paragraph states what a population requirement must be derived *from* and that a crossed triple must be refused, but states nothing about the requirement set being exactly the derived one. The reader implements the sufficiency half only — `required` refuses a missing pair and accepts an unjustified extra `Population`+`Closure` pair for any bound object role at any anchor of the declaration. The unowned behaviour is "a compiled artifact may not demand consumer observation inputs its source does not justify"; see SR-336 FND-001 for the code path and the missing tamper axis | spec/functional/FR-042-publish-compiled-protocol-artifacts.md:118; src/protocol_artifact/models/populations.rs:257; src/protocol_artifact/models/populations.rs:304 | missing-requirement |
| FND-003 | medium | The TC-121 evidence narrative in the matrix is stale and now understates and misattributes its own backing. It names only "Twenty public reader/encoder controls in `tests/protocol_artifact.rs`" — that file holds 24 — while TC-121 is now backed by 44 tagged tests across three files, the other two being `tests/native_protocol_emission.rs` (15, added by the parent) and `tests/native_population_emission.rs` (5, added here). The following sentence, that the fixtures "do not establish native family admission or actual source-to-consumer emission", is true of the reader suite but no longer describes the evidence base as a whole: native family admission is now exercised from authored source. The rows correctly stay `🚧 Planned`; the prose bounding them should name the actual suites | spec/model-linking/tests.md:192; spec/model-linking/tests.md:194; tests/native_population_emission.rs:1 | wrong-requirement |
| FND-004 | medium | Inherited, outside this change. Twenty untracked symbols persist: tests in `src/package/encoding/tests.rs` and `tests/package_construction_cases/limits.rs` carry `NFR-007-M-2..M-5` tags resolving to no declared row. Separately, three `IT-004` tags in `tests/fixture_audit.rs` are unmatched here because IT-004 is owned elsewhere; those three are the `#[ignore]`d private-packet lane with a named reason | src/package/encoding/tests.rs:67; tests/package_construction_cases/limits.rs:55; tests/fixture_audit.rs:231 | wrong-requirement |
| FND-005 | low | Carried unchanged from SR-334 FND-003. FR-042-AC-10 remains the one acceptance criterion with no tagged test, which is correct: it requires the real `quire-protocol` public Rust handoff, and FR-042's Dependencies section and `README.md` both hold it open. The five tests added here deliberately do not claim it — their producer and baseline metadata are explicitly synthetic. Recorded so the 9/10 figure is not read as drift | spec/functional/FR-042-publish-compiled-protocol-artifacts.md:162; tests/native_population_emission.rs:2 | correct-requirement-no-evidence |

## Coverage

- Reconciliation: `quire coverage --scope /home/peter/dev/worktrees/quire-language-native-populations --json`, quire 0.31.0 (engine 0.46.0), module `spec-artifacts-process`. Engine path, not a grep fallback; version ≥ 0.16.0, so the split document/source roots apply.
- Rows backed by a tagged test: 367 / 376 across the bundle; FR-042 acceptance criteria 9 / 10. Both unchanged from `09dafe4` — the five new tests tag criteria that were already backed, so the rollup proves nothing on its own and the assertions were read individually (SR-336).
- Tasks done: not applicable. No plan bundle targets FR-042, per the owner directive against creating plan bundles for this work.
- Status lies: 0. Every FR-042 and TC-121 row is `🚧 Planned`, and the five new tests do not claim otherwise.
- Unbacked rows: 6, of which 2 are method-exempt (`no_symbol_rows`). All inherited. Untracked symbols: 20, all inherited `NFR-007-M-*`; plus 3 unmatched `IT-004` tags owned outside this repository.
- New untraced behaviours: 1, recorded as FND-002. Source stubs: 0. Test stubs: 0. No `todo!`/`unimplemented!`/placeholder return in either new module.
- Test quality: the five new tests take authored sources and a real constructed `NativeModel` through parsing, exact binding, type and proof admission, `native::admit`, `native::emit` and an independent `read`. Locus expectations are rebuilt from `model.source().to_native(&role.source)` rather than read back from the payload; the tamper suite varies one axis at a time and reseals independently; the reader fixture's ten-definition vector is an authored enumeration (7 prior plus `Range`, `ObservationBinding`, `Progress`, the latter two pulling `Range` in as a declared dependency), not a figure observed from implementation usage.
- Semantic review: skipped, declined by the owner.

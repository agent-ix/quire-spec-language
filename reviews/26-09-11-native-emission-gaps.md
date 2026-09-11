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

Recheck of this analysis at `89bc4e3` using QUOIN `gap-analysis` with a fresh
`quire coverage --scope <worktree> --json`. The in-scope evidence gap is closed:
the four added tests execute the Choice, Await, positive `Repeat`,
`NeedsAuthority` and non-literal Boolean paths that had no caller at `ae910c0`,
and FR-042's Inputs section now owns the three caller-supplied revision
namespaces. The corpus reconciliation is unchanged — the same six unbacked rows
and twenty untracked symbols, all inherited from FR-036/FR-017/NFR-005/NFR-007
and all outside this change. The optional semantic review remains declined.

## Verdict

**FAIL** — retained solely by the skill's own rule that any matrix Test Case with
no backing tagged test fails the gate. TC-115 and three FR-036 acceptance rows are
still unbacked, and they are inherited, pre-existing and untouched here. No `high`
finding remains, nothing in the reviewed increment is unbacked, and no status lie
exists anywhere in the corpus. Read as a delivery signal for this increment, the
verdict is a standing corpus debt, not a defect in the native emission path.

## Disposition of the initial findings

- FND-001 (high, emitter behavior with no backing test): **resolved**. See the
  per-behavior verification in SR-333; each newly covered path has at least one
  assertion that a plausible wrong implementation fails.
- FND-002 (medium, reverse gap — unowned revision namespaces): **resolved**.
  FR-042 Inputs now states that the caller supplies revision namespaces for
  authored formal sources, requirements and registered semantic definitions, and
  that source-artifact revision labels stay separate from the semantic revisions
  derived from them. That covers all three public fields.
- FND-003 (medium, six inherited unbacked rows): **unchanged**, carried below.
- FND-004 (medium, twenty inherited untracked `NFR-007-M-*` symbols):
  **unchanged**, carried below.
- FND-005 (low, FR-042-AC-10 untagged by design): **unchanged**, carried below.

## Delivered scope versus FR-042/TC-121 acceptance

Delivered and now exercised end to end from real source: predicate, state,
temporal and protocol families; sequence, owned labeled choice with constant
coverage/non-overlap, parallel/join, bounded repetition including the max-zero and
constant-false bypasses, await with clock/progress/closure authority and its
success/timeout edges, check, event, attempt, effect, send, receive and commit
controls; record and scalar channels with FIFO keys and per-send delivery
cardinality; cross-unit callee owners and lexical `let` provenance; registered
semantic revisions independent of source artifact revisions; independent limits
with no partial package.

Not delivered, and correctly declared open in FR-042, `README.md` and
`docs/compiled-protocol-v1.md`: general symbolic choice and visibility proofs,
ordered-query proofs, recovery and compensation admission, unavailable producer
exports, and the actual producer-to-`quire-protocol` handoff. A successful native
emission followed by an independent `read` remains compiler-to-reader evidence
only; it establishes nothing about B's consumer implementation or runtime
conformance. These are full-requirement gaps against FR-042, not defects in the
increment delivered here.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | Inherited, outside this change, and the sole cause of the FAIL. Six unbacked reference rows persist: TC-115 in `spec/model-linking/tests.md` with FR-036-AC-5/AC-6/AC-8, TC-010 in `spec/tests.md` (NFR-005-M-1) and FR-017-AC-2. TC-010 and FR-017-AC-2 are exempt by declared method (`Manual`, `Inspection`) and appear in `no_symbol_rows`; the four FR-036/TC-115 rows are not exempt and belong to FR-036's own delivery, not FR-042's | spec/model-linking/tests.md:107; spec/functional/FR-036-link-composed-native-packages.md:129; spec/functional/FR-036-link-composed-native-packages.md:130; spec/functional/FR-036-link-composed-native-packages.md:132 | correct-requirement-no-evidence |
| FND-002 | medium | Inherited, outside this change. Twenty untracked symbols persist: tests in `src/package/encoding/tests.rs` and `tests/package_construction_cases/limits.rs` carry `NFR-007-M-2..M-5` tags resolving to no declared row. Separately, three `IT-004` tags in `tests/fixture_audit.rs` are unmatched in this repository because IT-004 is owned elsewhere; those three tests are the `#[ignore]`d private-packet lane and carry a named reason | src/package/encoding/tests.rs:67; src/package/encoding/tests.rs:105; src/package/encoding/tests.rs:170; tests/package_construction_cases/limits.rs:55 | wrong-requirement |
| FND-003 | low | FR-042-AC-10 remains the one acceptance criterion with no tagged test, which is correct: it requires the real `quire-protocol` public Rust handoff, and FR-042's Dependencies section and `README.md` both hold it open. Recorded so the 9/10 figure is not read as drift | spec/functional/FR-042-publish-compiled-protocol-artifacts.md:151 | correct-requirement-no-evidence |
| FND-004 | low | Full-requirement gap, honestly declared, no false completion claim. General symbolic choice/visibility proofs, ordered-query proofs, recovery and compensation admission, unavailable producer exports and the producer-to-B handoff are unimplemented and refuse explicitly through `Unsupported::{FamilyProof,Export,Feature}`. Every FR-042 and TC-121 matrix row remains `🚧 Planned`. These bound the requirement, not this increment | docs/compiled-protocol-v1.md:504; README.md:46; spec/functional/FR-042-publish-compiled-protocol-artifacts.md:151 | correct-requirement-no-evidence |
| FND-005 | low | Evidence granularity inside the newly added coverage, cross-referenced from SR-333 FND-001/FND-002. The choice test does not pin case-to-guard correspondence and the paired `G`/`not (G)` guards leave the `Equal`/`NotEqual` interpreter results proved constant but not proved correct. Both paths execute and the code is right; the assertions do not exclude a specific wrong implementation | tests/native_protocol_emission.rs:1142; src/protocol_artifact/native/families.rs:353; src/protocol_artifact/native/families.rs:356 | correct-requirement-no-evidence |

## Coverage

- Reconciliation: `quire coverage --scope /home/peter/dev/worktrees/quire-language-native-emission --json`, quire 0.31.0, module `spec-artifacts-process`. Engine path, not a grep fallback. Version ≥ 0.16.0, so the split document/source roots apply.
- Rows backed by a tagged test: 367 / 376 across the bundle; FR-042 acceptance criteria 9 / 10. Both unchanged from `ae910c0` — the new tests add tags to criteria that were already backed, which is why counts alone prove nothing here and the assertions were read individually.
- Tasks done: not applicable. No plan bundle targets FR-042, per the owner directive against creating plan bundles for this work.
- Status lies: 0. Every FR-042 and TC-121 row is `🚧 Planned`.
- Unbacked rows: 6, of which 2 are method-exempt (`no_symbol_rows`). All inherited.
- Untracked symbols: 20, all inherited `NFR-007-M-*`; plus 3 unmatched `IT-004` tags owned outside this repository.
- Diagnostics: 25 declarations selected nothing; none in the reviewed scope.
- New untraced behaviors: 0. The one recorded at `ae910c0` is now owned by FR-042 Inputs. Source stubs: 0. Test stubs: 0.
- Test quality in the new suite: the four added tests assert exact values, not counts; reader expectations in `tests/support/native_protocol/mod.rs` are rebuilt from original parser spans and authored mappings rather than read back from the emitted payload; `discharged()` asserts real `Typed`/`Discharged` dispositions. Two residual assertion-strength gaps are recorded as FND-005.
- Semantic review: skipped, declined by the owner.

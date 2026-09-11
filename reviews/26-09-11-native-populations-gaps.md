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

QUOIN `gap-analysis` recheck at source commit `84aec59` with a fresh
`quire coverage --scope <worktree> --json`. The two gaps this review raised
against the increment are closed: FR-042 now owns the surplus/missing pair rule
with its typed cause, and the TC-121 evidence narrative in the matrix has been
rewritten to name the actual suites and reader scope. The corpus reconciliation
is byte-identical to the previous run — the same six unbacked rows and twenty
untracked symbols, all inherited from FR-036/FR-017/NFR-005/NFR-007 and all
outside this change. The optional semantic review remains declined.

## Verdict

**FAIL** — retained solely by the skill's own rule that any matrix Test Case with
no backing tagged test fails the gate. TC-115 and three FR-036 acceptance rows are
still unbacked, inherited, pre-existing and untouched here. Read as a delivery
signal for this increment: nothing in the reviewed scope is unbacked, there is no
status lie anywhere in the corpus, and the highest finding attributable to the
increment is now low. The increment is mergeable on this axis.

## Disposition of the initial findings

- **FND-002 (medium, reverse gap — necessity unowned) — resolved.** FR-042's
  Behavior section now states that the population/closure set SHALL equal the set
  required by the declaration's original input binders and anchored values, and
  that missing or surplus pairs refuse as `Invalid::Binding`
  (`spec/functional/FR-042-publish-compiled-protocol-artifacts.md:122-127`). The
  wire contract carries the matching interchange text and, importantly, keeps the
  two actors distinct: native admission derives the set from original source
  owners, and the parser-free reader checks the set justified by the *decoded*
  input binders and anchored values against admitted models
  (`docs/compiled-protocol-v1.md:112-119`). Neither document claims the reader
  establishes source equivalence. The behaviour is assigned to the existing
  FR-042-AC-4 and FR-042-AC-7, with no new id, renumbering or artifact campaign,
  and the code path plus its new tamper axis are recorded in SR-336.
- **FND-003 (medium, stale TC-121 evidence narrative) — resolved.**
  `spec/model-linking/tests.md:192-198` no longer quotes a control count that
  drifts. It names `tests/protocol_artifact.rs` for reader/encoder behaviour and
  `tests/native_protocol_emission.rs` plus `tests/native_population_emission.rs`
  for real source through native family admission, emission and independent
  reading, including exact population requirements and surplus inputs. Verified
  against the tree: 24 + 15 + 5 = 44 `TC-121`-tagged tests across exactly those
  three files. The paragraph still bounds the claim correctly — the actual
  producer-to-B-consumer handoff and the remaining full-family obligations stay
  open, and every matrix row below it remains `🚧 Planned`.
- **FND-001, FND-004, FND-005 — unchanged and retained below.** All three are
  inherited or deliberately open, and none is a defect in the delivered
  increment.

## Delivered scope versus FR-042/TC-121 acceptance

Unchanged from the initial review, plus the correction: the reader now validates
the offered decoded binder/value graph in both directions against admitted
models, so an artifact cannot demand consumer `ObservationBinding`/`Progress`
inputs that no decoded input binder or anchored value justifies.

Not delivered, and correctly left open: every FR-042 and TC-121 matrix row
remains `🚧 Planned`. The new tests use explicitly synthetic producer and
baseline metadata, so they establish compiler and reader behaviour, not
FR-042-AC-10 or the `quire-protocol` IT-001 handoff. Composed source/result
correspondence has no adopted contract and stays `Unsupported::Feature`;
`ValueOperation::Parent`, relationship, component and endpoint exports still
refuse. Population membership, closure truth and runtime identities are declared
consumer inputs and are neither computed nor required for static linking. These
are corpus and integration limitations, separate from the delivered increment.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | Inherited, outside this change, and the sole cause of the FAIL. Six unbacked reference rows persist, re-confirmed at `84aec59` rather than assumed: TC-115 in `spec/model-linking/tests.md` with FR-036-AC-5/AC-6/AC-8, TC-010 in `spec/tests.md` (NFR-005-M-1) and FR-017-AC-2. TC-010 and FR-017-AC-2 are exempt by declared method (`Manual`, `Inspection`) and appear in `no_symbol_rows`; the four FR-036/TC-115 rows are not exempt and belong to FR-036's own delivery | spec/model-linking/tests.md:107; spec/functional/FR-036-link-composed-native-packages.md:129; spec/functional/FR-036-link-composed-native-packages.md:130; spec/functional/FR-036-link-composed-native-packages.md:132 | correct-requirement-no-evidence |
| FND-004 | medium | Inherited, outside this change. Twenty untracked symbols persist: tests in `src/package/encoding/tests.rs` and `tests/package_construction_cases/limits.rs` carry `NFR-007-M-2..M-5` tags resolving to no declared row. Separately, three `IT-004` tags in `tests/fixture_audit.rs` are unmatched here because IT-004 is owned elsewhere; those three are the `#[ignore]`d private-packet lane with a named reason | src/package/encoding/tests.rs:67; tests/package_construction_cases/limits.rs:55; tests/fixture_audit.rs:231 | wrong-requirement |
| FND-005 | low | Carried unchanged. FR-042-AC-10 remains the one acceptance criterion with no tagged test, which is correct: it requires the real `quire-protocol` public Rust handoff, and FR-042's Dependencies section and `README.md` both hold it open. The five population tests deliberately do not claim it — their producer and baseline metadata are explicitly synthetic. Recorded so the 9/10 figure is not read as drift | spec/functional/FR-042-publish-compiled-protocol-artifacts.md:168; tests/native_population_emission.rs:2 | correct-requirement-no-evidence |
| FND-006 | low | New, recorded so it is not read as drift later. Two normative statements the correction added — that the population requirement key is (selected model, object record, original observation anchor) with binding names as opaque labels, and that nested record/reference traversal visits each key once including cycles under the declared work limits — are owned by the interchange document only, not by an FR/NFR row. That is the correct outcome under the owner's directive against new artifact campaigns and against unauthorized StR/NFR prototype gates, and FR-042 cross-references the wire contract, so neither statement is unowned; it does mean a coverage rollup will never bind them | docs/compiled-protocol-v1.md:112; docs/compiled-protocol-v1.md:119; spec/functional/FR-042-publish-compiled-protocol-artifacts.md:122 | missing-requirement |

## Coverage

- Reconciliation: `quire coverage --scope /home/peter/dev/worktrees/quire-language-native-populations --json`, quire 0.31.0 (engine `ca7362d4`), module `spec-artifacts-process`. Engine path, not a grep fallback; version ≥ 0.16.0, so the split document/source roots apply.
- Rows backed by a tagged test: 367 / 376 across the bundle; FR-042 acceptance criteria 9 / 10 (AC-1..AC-9 backed, AC-10 not). Identical to the previous run — the correction added no test function, only a ninth axis inside an existing tagged test and a second declaration inside another, so the rollup moves for neither and the assertions were read individually (SR-336).
- Tasks done: not applicable. No plan bundle targets FR-042, per the owner directive against creating plan bundles for this work.
- Status lies: 0. Every FR-042 and TC-121 row is `🚧 Planned`, and the rewritten matrix narrative does not claim otherwise.
- Unbacked rows: 6, of which 2 are method-exempt (`no_symbol_rows`). All inherited. Untracked symbols: 20, all inherited `NFR-007-M-*`; plus 3 unmatched `IT-004` tags owned outside this repository.
- New untraced behaviours: 0 in the reviewed scope; the one recorded previously is now owned by FR-042. Source stubs: 0. Test stubs: 0. No `todo!`/`unimplemented!`/placeholder return in either module.
- Test quality: the new `surplus_pair` axis is a real gate, not a tautology — it was run against the pre-correction reader and failed with `expected Invalid(Binding), admission succeeded`, and it asserts beforehand that the target declaration carries no such population so it cannot pass vacuously. The `ScalarEdge` addition pins `CauseKind::InvalidGraphEdge` independently of the crossed-universe `TypeMismatch`.
- Semantic review: skipped, declined by the owner.

---
id: SR-341
title: "Gap analysis of the native protocol producer example against FR-042 and TC-121"
type: SpecReview
analysis: gap-analysis
scope: "examples/native_protocol_handoff.rs; examples/protocol-handoff/; spec/functional/FR-042-publish-compiled-protocol-artifacts.md; spec/model-linking/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: references
---

## Summary

QUOIN `gap-analysis` rerun over correction source `56c1621` with a fresh
`quire coverage --scope /home/peter/dev/worktrees/quire-language-native-handoff
--json` (quire 0.31.0). The correction adds one matrix row and no test, so the
corpus reconciliation moves from 367/376 to 368/377 backed with the same six
unbacked rows, two method-exempt rows, twenty untracked `NFR-007-M-*` symbols,
three unmatched `IT-004` tags and zero status lies. Steps 1-3 ran; the optional
semantic review (step 4) remains declined by the owner.

## Verdict

**FAIL** — unchanged, and unchanged in cause: the skill's own rule fails the gate
on any matrix Test Case with no backing tagged test, and TC-115 plus three FR-036
acceptance rows are still unbacked. All four are inherited, pre-existing and
outside this increment. Read as a delivery signal for the reviewed increment this
is standing corpus debt: the example adds nothing unbacked, claims nothing
unevidenced, and carries no `high` finding.

## Disposition of the prior findings

| Prior | Disposition |
| --- | --- |
| FND-001 four inherited unbacked rows (TC-115, FR-036-AC-5/AC-6/AC-8) | open, inherited; identical rows at `spec/model-linking/tests.md:107` and `spec/functional/FR-036-link-composed-native-packages.md:129,130,132` |
| FND-002 twenty inherited untracked `NFR-007-M-*` symbols | open, inherited; count unchanged at 20 |
| FND-003 one aggregated TC-121 row hid AC-10's distinct state | resolved at the matrix, with a correction to the earlier reading — see below |
| FND-004 three `IT-004` tags matching no matrix target | open, inherited; count unchanged at 3 |
| FND-005 no plan bundle owns FR-042 | open; `grep -rln FR-042 plan` still returns nothing |

**FND-003 disposition.** `spec/model-linking/tests.md:111-112` now carries two
rows: `TC-121 … FR-042-AC-1..FR-042-AC-9` and a separate
`TC-121 | Actual compiler-to-consumer Rust handoff; requires B's IT-001 |
FR-042-AC-10`. The open prerequisite is now named in the matrix with its actual
external target. Two corrections to the earlier finding's reading, both from this
run's coverage output: `FR-042-AC-10` reads `backed: false` in the FR-042
criterion group (9/10 backed), so the open AC is machine-visible through the
requirement's own verification row, which cites TC-121 *and* quire-protocol
IT-001; and the per-AC table at `spec/model-linking/tests.md:212` still maps
AC-10 to TC-121 alone, so the matrix carries two representations of that state.
The split is an improvement in the human-readable matrix, not the source of the
machine-visible flag.

## Step 1 — plan completion

No plan bundle targets FR-042, so this skill's nominal step-1 artifact does not
exist for this scope and the review targets the FR-042/TC-121 requirement set
plus the matrix. Across all bundles 31 of 32 tasks are `done`; the one
`in_progress` task remains
`plan/Plan-008-native-lowering/tasks/Task-020-qualify-boolean-backend.md`, which
owns generated Boolean execution and is unrelated to this example.

## Step 2 — matrix verification

| Reconciliation | Count | Attribution |
| --- | --- | --- |
| Backed targets | 368 / 377 | +1/+1 from the AC-10 matrix row split |
| Unbacked rows | 6 | 4 inherited FR-036/TC-115, 2 method-exempt |
| Method-exempt (`no_symbol_rows`) | 2 | TC-010 `Manual`, FR-017-AC-2 `Inspection` |
| Untracked symbols | 20 | `NFR-007-M-2..M-5` in `src/package/encoding/tests.rs` and siblings |
| Unmatched tags | 3 | `IT-004` in `tests/fixture_audit.rs` |
| Status lies | 0 | — |
| FR-042 acceptance criteria | 9 / 10 backed | `FR-042-AC-10` unbacked, pending quire-protocol IT-001 |

Raw report retained at `/tmp/quire-native-handoff-recheck-coverage.json`.

## Step 3 — underspecified code (reverse gap)

Unchanged and still fully owned: both example files carry `FR-042/TC-121` module
headers, the README states the requirement contract, and the increment adds no
`pub` library API, test symbol or trace tag. The restructuring in `915f479`
introduced new private types (`Inputs`, `DefinitionInputs`, `SelectedInputs`,
`StageIssue`, `ProofCause`) inside the example module only; none is a stub,
placeholder return or re-export-only module, and the built binary still executes
the whole chain into a verified package (SR-340 records the byte-level
re-verification).

## Delivered scope versus the open AC-10 prerequisite

Unchanged: this increment delivers the producer half of AC-10 and nothing else.
B's public Rust admission/linking interface and IT-001 do not exist in any
repository reachable from here, and neither the README nor the matrix now claims
otherwise. The corrected FR-042 Inputs and TC-121 step 10 additionally name who
owns the consumer-side selection, which narrows the specification ambiguity
recorded in SR-342 — it does not create B's missing public Rust consumer, and it
defines no production sidecar schema.

## Findings

| ID      | Severity | Summary                                                                  | Refs                                               |
| ------- | -------- | ------------------------------------------------------------------------ | -------------------------------------------------- |
| FND-001 | medium   | Four inherited unbacked matrix rows (TC-115, FR-036-AC-5/AC-6/AC-8) hold the gate at FAIL | spec/model-linking/tests.md:107                     |
| FND-002 | medium   | Twenty inherited untracked `NFR-007-M-*` symbols carry no matrix row      | src/package/encoding/tests.rs                       |
| FND-003 | low      | The per-AC table still maps FR-042-AC-10 to TC-121 alone, beside the new prerequisite row | spec/model-linking/tests.md:212                     |
| FND-004 | low      | Three `IT-004` tags in `tests/fixture_audit.rs` match no matrix target    | tests/fixture_audit.rs:231                          |
| FND-005 | low      | No plan bundle owns FR-042, so this skill's step-1 target is absent       | plan/                                               |

FND-001, FND-002 and FND-004 are the same inherited items carried in SR-334 and
SR-328, reported here for reconciliation completeness only; no action belongs to
this PR. FND-003 is the residue of the earlier FND-003 and costs one cell.

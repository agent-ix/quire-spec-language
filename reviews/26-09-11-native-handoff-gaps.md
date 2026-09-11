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

QUOIN `gap-analysis` over the example-only increment at `6243ef9`, with a fresh
`quire coverage --scope /home/peter/dev/worktrees/quire-language-native-handoff
--json` (quire 0.31.0, engine 0.46.0). The increment adds no test, no spec text
and no matrix row, so the corpus reconciliation is unchanged from the parent:
367/376 backed, six unbacked rows, twenty untracked `NFR-007-M-*` symbols, three
unmatched `IT-004` tags, zero status lies. Every one of those is inherited from
FR-036/FR-017/NFR-005/NFR-007 and untouched here. The optional semantic review
(step 4) was declined. Steps 1-3 ran.

## Verdict

**FAIL** — held by the skill's own rule that any matrix Test Case with no backing
tagged test fails the gate. TC-115 and three FR-036 acceptance rows remain
unbacked, all pre-existing and all outside this change. Read as a delivery signal
for the reviewed increment the verdict is standing corpus debt: nothing the
example adds is unbacked, nothing it claims is unevidenced, and no `high` finding
exists against it.

## Step 1 — plan completion

No plan bundle targets FR-042; `grep -rln FR-042 plan` returns nothing, so this
skill's nominal target artifact does not exist for this scope, and the review
targets the FR-042/TC-121 requirement set plus the matrix instead. Across all
bundles 31 of 32 tasks are `done`; the single `in_progress` task is
`Plan-008-native-lowering/tasks/Task-020-qualify-boolean-backend.md`, which owns
generated Boolean execution and is unrelated to this example.

## Step 2 — matrix verification

| Reconciliation | Count | Attribution |
| --- | --- | --- |
| Backed targets | 367 / 376 | — |
| Unbacked rows | 6 | 4 inherited FR-036/TC-115, 2 method-exempt |
| Method-exempt (`no_symbol_rows`) | 2 | TC-010 `Manual`, FR-017-AC-2 `Inspection` |
| Untracked symbols | 20 | `NFR-007-M-2..M-5` in `src/package/encoding/tests.rs` and siblings |
| Unmatched tags | 3 | `IT-004` in `tests/fixture_audit.rs:231,275,345` |
| Status lies | 0 | — |

The four non-exempt unbacked rows are TC-115 (`spec/model-linking/tests.md:107`)
and FR-036-AC-5/AC-6/AC-8
(`spec/functional/FR-036-link-composed-native-packages.md:129,130,132`). They
belong to FR-036's delivery, not to FR-042.

## Step 3 — underspecified code (reverse gap)

The example is fully owned: both new Rust files carry `FR-042/TC-121` module
headers, and the README states the requirement contract it implements. It adds no
`pub` API to the library, no test symbol and no trace tag, so it neither raises
nor lowers matrix backing — which is the correct outcome for a recipe that claims
no acceptance criterion. No stub, placeholder return or re-export-only module was
introduced; the delivered binary executes the whole chain and produces a verified
package (see SR-340 for the byte-level verification).

## Delivered scope versus the open AC-10 prerequisite

FR-042-AC-10 requires accepted native source to emit a fixture "consumed
unchanged by quire-protocol's public Rust admission/linking interface". This
increment delivers the producer half of that sentence and nothing else: real
compilation, real emission, real independent read, real selectors on disk. The
consumer half — B's public Rust admission/linking interface and IT-001 — does not
exist in any repository reachable from here, and the example README says so
without hedging. That is an unmet integration prerequisite, exactly as TC-121
step 10 anticipates ("a missing family, producer adapter or consumer
implementation records the unmet positive integration prerequisite"), not a defect
in the delivered recipe.

## Findings

| ID      | Severity | Summary                                                                  | Refs                                               |
| ------- | -------- | ------------------------------------------------------------------------ | -------------------------------------------------- |
| FND-001 | medium   | Four inherited unbacked matrix rows (TC-115, FR-036-AC-5/AC-6/AC-8) hold the gate at FAIL | spec/model-linking/tests.md:107                     |
| FND-002 | medium   | Twenty inherited untracked `NFR-007-M-*` symbols carry no matrix row      | src/package/encoding/tests.rs                       |
| FND-003 | medium   | One matrix row aggregates FR-042-AC-1..AC-10, so the open AC-10 handoff is indistinguishable from covered ACs | spec/model-linking/tests.md:111                     |
| FND-004 | low      | Three `IT-004` tags in `tests/fixture_audit.rs` match no matrix target    | tests/fixture_audit.rs:231                          |
| FND-005 | low      | No plan bundle owns FR-042, so this skill's step-1 target is absent       | plan/                                               |

## Finding detail

**FND-003** is the one worth acting on for this workstream. `spec/model-linking/tests.md:111`
records a single `TC-121 | … | FR-042-AC-1..FR-042-AC-10 | 🚧 Planned` row, and
`spec/model-linking/tests.md:199-208` repeats one row per AC against the same test
case. Tests tagged `TC-121` exist (`tests/native_protocol_emission.rs`,
`tests/native_population_emission.rs`, `tests/protocol_artifact.rs`), so coverage
counts all ten rows backed while AC-10's positive producer-to-consumer handoff is
genuinely open. No status lie is reported because the rows are still `🚧 Planned`.
The honest reading is that AC-10 needs its own row or an explicit blocked marker
once B's interface lands; until then only prose records the distinction. This
example neither caused nor worsened it, and correctly refrains from claiming
AC-10.

FND-001, FND-002 and FND-004 are the same inherited items carried in SR-334 and
SR-328. They are reported here for reconciliation completeness only; no action
belongs to this PR.

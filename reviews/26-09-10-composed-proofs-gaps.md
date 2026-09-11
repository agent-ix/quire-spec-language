---
id: SR-328
title: "Composed guarded-definedness delivery gaps against FR-040 and TC-119"
type: SpecReview
analysis: gap-analysis
scope: "FR-040; TC-119; TM-003 (spec/model-linking/tests.md); src/checking/composed/proofs/; src/checking/proof/; tests/composed_proofs.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-040
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TM-003
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-119
    type: references
---

## Summary

Gap analysis of the composed guarded-definedness slice at 0dab1ce. The engine
reports **351/359 rows backed with zero status lies**, and every FR-040 matrix
row correctly stays 🚧 Planned. The slice genuinely closes SR-326 FND-001 —
FR-040-AC-10 now carries a real historical-boundary control — and puts actual IR
discharge, authored correspondence and work accounting behind fourteen tagged
public tests. It does not discharge TC-119: the ordered-query, graph and runtime
groups are unimplemented by design, and the stage's own
`Unsupported::ValueRepresentation` refusal — the thing AC-4 requires when a
representation is missing — is asserted by no test.

## Verdict

**FAIL** — the full-ticket gate. TC-119 groups 2, 5 and 7 have no
implementation, FR-040-AC-5's positive obligations are tag-backed only, and
FND-001 is high against AC-4's stated refusal. This is the expected result for
an intentionally partial slice of compiler #40, not a regression.

**Delivered stage: CONDITIONAL.** The definedness scope this PR claims —
`discharge` over a borrowed `TypeReport` and authored `CheckBindings`, exact
source/clause/execution-point correspondence with all seven typed refusals,
evaluation-ordered walking with stable aliases, guards, integer/rational domains
and nonzero checks, caller-site argument proof separated from callee totality,
explicit `Unsupported` for ordered queries, dependency propagation over the
existing namespace graph, and fourteen independently charged work dimensions
with fresh-retry and hard-clamp controls — is implemented and locally verified.
Family/runtime admission, #40 source-to-B emission and full FR-040 stay open.

## Findings

| ID      | Severity | Summary                                                                                                | Refs                                                       |
| ------- | -------- | -------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------ |
| FND-001 | high     | `Unsupported::ValueRepresentation`, the refusal FR-040-AC-4 names, is raised at eight sites and asserted by no test | src/checking/composed/proofs/engine/walk.rs:106, tests/composed_proofs.rs:694 |
| FND-002 | medium   | FR-040-AC-5 is tagged by a refusal-only control, so the engine counts the ordered-query criterion as backed | tests/composed_proofs.rs:683                                |
| FND-003 | medium   | `Unsupported::FamilyControl` is a public variant that no code constructs and no test observes             | src/checking/composed/proofs.rs:34                          |
| FND-004 | low      | No control pins `DeclarationProof::complete()` on a refused declaration, masking SR-327 FND-001           | tests/composed_proofs.rs:117                                |
| FND-005 | low      | TC-119 groups 2, 5 and 7 remain unimplemented in this stage; AC-2/AC-5/AC-7 rest on type-admission tags   | spec/test-cases/TC-119-check-composed-values.md:60          |

## Detail

**FND-001.** `Unsupported::ValueRepresentation` is the typed prerequisite
FR-040-AC-4 requires ("Missing producer/proof representation yields an explicit
unsupported prerequisite"). It is constructed at eight sites — unsupported
observation origin (`walk.rs:62`), an integer literal outside `i64`
(`walk.rs:106`), a field base that is not a record or object (`walk.rs:128`),
integer division and remainder (`walk.rs:277`), `mod` (`walk.rs:463`), symbol
and representation failure (`engine.rs:366,414`) and the materializer adapter
(`engine/graph.rs:72`). No test asserts any of them; the only `Unsupported`
assertion in the suite is `OrderedQuery` (`tests/composed_proofs.rs:694`). Every
tagged AC-4 control exercises the positive integer/rational paths instead.
Failure scenario: `a / b` on integers silently reclassifying from
`Unsupported(ValueRepresentation)` to `Unproved` — or to a discharge, once
`Kind::Numeric(Divide)` exists for integers — would pass the whole suite. One
control per meaning, with the cause and the original locus, closes it.

**FND-002.** `unsupported_ordered_query_proofs_remain_distinct_from_type_admission`
carries `FR-040-AC-5` (`tests/composed_proofs.rs:683`), but it asserts only that
a query refuses with `Unsupported::OrderedQuery` and that its dependents refuse.
AC-5 requires all eight query forms with their exact binder and result shape,
N=0/10,000/10,001 boundaries, the Amount/Total/Count vector and per-prefix sum
obligations. The matrix row is honestly 🚧 Planned so there is no status lie, but
the tag makes `quire coverage` count the criterion as backed, which is how a
refusal control gets mistaken later for the positive case it blocks — exactly
what TC-119's own preamble warns against. Tag it AC-4 (unsupported prerequisite)
and leave AC-5 untagged until the query representation exists.

**FND-003.** `Unsupported::FamilyControl` (`proofs.rs:34`) is constructed
nowhere and observed by no test; only `OrderedQuery` and `ValueRepresentation`
are reachable. It is a public `#[non_exhaustive]` variant promising a
classification the stage cannot produce — `rust-review` §3, a public item with
no caller and no test. Either raise it where a temporal/protocol control is
reached, or drop it until that stage lands.

**FND-004.** `unproved()` (`tests/composed_proofs.rs:106-120`) asserts the
declaration's `disposition()` and its cause, and `discharged()` asserts
`complete()` only on the positive path. Nothing asserts `complete() == false`
for a refused declaration, which is why SR-327 FND-001 — `complete()` true for a
leaf whose only goal was unproved — survives a green suite.

**FND-005.** TC-119 group 2 (StateCore/StateQueries/StateGraph permissions),
group 5 (ordered queries) and group 7 (graph/context/operation/capture runtime
requirements) have no proof-stage implementation. AC-2 and AC-7 keep their
type-admission tags from `tests/composed_types.rs` and are unaffected by this
slice; AC-5 is discussed above. Recorded as scope, not as a defect — the spec,
README and matrix all state it plainly, and no row was upgraded.

## Coverage

- **Plan completion:** not applicable — no plan bundle covers FR-040 and the
  owner's bookkeeping directive forbids inventing one. Recorded, not scored.
- **Matrix verification:** `quire coverage --scope <worktree> --json` at 0dab1ce
  (quire 0.31.0, engine 0.46.0) — **351/359 backed**, **0 status lies**, 243
  criteria. Six unbacked rows, all inherited and outside this scope: TC-115
  (FR-036-AC-5/6/8), TC-010 (NFR-005-M-1, Manual) and FR-017-AC-2 (Inspection).
  Three unmatched tags (IT-004, `tests/fixture_audit.rs`) and 20 untracked
  symbols (NFR-007 package encoding/limits) are also inherited. SR-326 FND-001
  is **resolved**: FR-040-AC-10 is now backed by
  `composed_discharge_does_not_upgrade_the_historical_model_boundary`.
- **Underspecified code:** no stub, `todo!`, `unimplemented!`, `TODO`, `dbg!`,
  stray `eprintln!` or new `#[allow]` in the new tree; no re-export-only module;
  every new public item carries a doc comment and an owning FR-040 sentence
  (`discharge`/`ProofReport` → Outputs; `ProofLimits`/`Usage`/`Exhaustion` →
  Work accounting; `Cause`/`CauseKind` → the unproved and unsupported clauses).
  The one reverse gap is FND-003. No test is assertion-free or tautological: the
  zero-budget control compares against an independently authored charge model
  and a fresh-retry usage equality, and the diamond control derives 4/8 from the
  published contract rather than from the run's own usage.
- **Semantic review (step 4):** **skipped** — the optional semantic-review
  extension is declined by standing owner direction. Intent↔test↔code agreement
  was not independently judged beyond the mechanical checks above.
- **Gates:** all local gates green at this HEAD; counts in SR-327.

## Acceptance classification

Fully verified for the delivered definedness scope: AC-8 (correspondence,
all seven typed refusals), AC-9 (all fourteen dimensions at zero, the exact and
one-short diamond, hard clamping, fresh retries) and AC-10. Tag-backed only (a
tagged control exists; the criterion is not discharged): AC-1, AC-3, AC-4
(pending FND-001), AC-6. Carried by type admission only, untouched here: AC-2,
AC-7. Tag-backed but not exercised by this stage: AC-5 (FND-002). Full FR-040
and TC-119 acceptance and #35/#36/#39/#40 remain open; the missing producer/IR
query and family capability stays an implementation gap, not a narrower
definition of the requirement.

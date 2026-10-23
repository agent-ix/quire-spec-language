---
id: SR-16246
title: "spec-review of the execute signature, backend choice and supplied backends (quire-spec-language PR 1323)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@8cc7b5b06a1e13f601404213529476ee380fb32f; spec/functional/FR-279-execute-a-checked-entry-as-a-library-operation.md; spec/functional/FR-285-map-every-outcome-category-to-one-exit-code.md; spec/functional/FR-296-select-the-execution-backend-in-the-request.md; spec/test-cases/TC-760-execute-returns-what-checkedpackage-call-and-evaluate-return.md; spec/test-cases/TC-769-the-exit-function-maps-every-category-and-multi-item-outcome-with.md; spec/test-cases/TC-781-the-request-selects-the-execution-backend-and-no-other-backend-runs.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-279
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-285
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-294
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-296
    type: references
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: references
---

# Spec review of the execute signature, backend choice and supplied backends (quire-spec-language PR 1323)

## Summary

Ticket: STD-369 (QSL-591). This is one spec-review pass over
`git diff origin/main...8cc7b5b06`: FR-279's new Signature section and
prepare-failure mapping, FR-296's `ExecutionBackendChoice`,
`SuppliedBackends` and `SuppliedBackend`, FR-285's categories for
`CallFailure::BackendNotSupplied` and `CallFailure::Limit`, and the TC-760,
TC-769 and TC-781 steps. The oracles were STD-369's gap list and its exact
missing statements, open PR #1316 (STD-350, head 8c656406) which carries the
FR-294 half of QSL-591, FR-294 and ADR-029 EB-1 on main, ADR-013 O-16, and
FR-401 as #1316 edits it.

Gates: `QSPEC_DIR=~/dev/quire-specification make check-spec-validation` at
8cc7b5b06 exits 0 with no warning on a touched file. origin/main (c21bc57a)
is an ancestor of the head. A trial merge of this head into #1316's head
completes with no textual conflict (see FND-002 for what it produces).

The head moved to 7875f6ca while this review ran. This review is of
8cc7b5b06, the head assigned for review. 7875f6ca adopts #1316's
`CallFailure::Unsupported` design and restores the FR-294 text, so it needs
its own review pass. At 7875f6ca, FR-279-AC-4 has different text from
#1316's FR-279-AC-4.

Examined, clean:

- **`SuppliedBackend` as the object-safe form of `ExecutionBackend`.**
  `ExecutionBackend` has an associated `Prepared` type, so it is not
  dyn-compatible. A blanket implementation that boxes `B::Prepared` as
  `Box<dyn PreparedEntries + '_>` is sound Rust and keeps the closed
  `{aot, jit}` set.
- **The interpreter has no `SuppliedBackends` member.** `qsl-eval` owns
  it, and `SuppliedBackends::default()` supplies it alone.
- **FR-296-AC-4** is testable. It counts `prepare` calls on a test JIT and
  checks that choosing `aot` with the same supply calls neither backend.
- **The exit codes** for an absent backend (21) and a prepare limit (22)
  agree with O-16 and with FR-285's order 30, 20, 21, 22, 10, 0.
- **The lead defaults are flagged** in the PR body as "lead default, not an
  owner answer".

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | This PR and #1316, which QSL-591 needs merged together, give the same condition two contradictory designs and reuse the same AC ids with different text. Here a chosen backend that is not supplied is `CallFailure::BackendNotSupplied(ExecutionBackendChoice)`. In #1316 it is `CallFailure::Unsupported(UnsupportedEntry { backend, cause: NotSupplied })`, and #1316's FR-401-AC-11 places `UnsupportedCause::NotSupplied` in `quire_eval`. STD-369's own exact statement also asks for #1316's form. FR-285-AC-6, FR-296-AC-2, FR-296-AC-3 and FR-296-AC-4 appear in both PRs with conflicting text. This PR's FR-279-AC-5 repeats #1316's FR-279-AC-4 (Limit and Fault) and adds Cancelled. The PR body also describes #1316 wrongly: it names `prepare.work_units`, `UnsupportedEntry { entry, construct }` and `PrepareFailure::Unsupported`, but #1316 has `compile_work_units`, `{ backend, cause }`, and no per-entry `PrepareFailure` arm. Fix: use one design for an absent backend, and give each AC id one text across both PRs. | spec/functional/FR-279-execute-a-checked-entry-as-a-library-operation.md:68-76, :116; spec/functional/FR-296-select-the-execution-backend-in-the-request.md:60, :78-81, :91-93; spec/functional/FR-285-map-every-outcome-category-to-one-exit-code.md:92-94, :110 | wrong-requirement |
| FND-002 | high | Merging this head with #1316, in either order, silently deletes #1316's FR-294 seam types. #1316 contains commit bbeedf4b, which was this PR's earlier head. 8cc7b5b06 reverts bbeedf4b's FR-294, ADR-029 and TC-778 edits, so a three-way merge applies the revert to #1316's text with no conflict. A trial merge of this head into #1316 removes FR-294's table (`EntrySelection`, `PrepareLimits`, `PrepareFailure`, `UnsupportedEntry`, `UnsupportedCause`), `PreparedEntries::unsupported`, FR-294-AC-5 and TC-778 step 5. It also brings back "`PrepareFailure` holds per-entry `Unsupported` causes" in FR-294 and ADR-029 EB-1, which is the contradiction STD-369 gap 1 names. The PR body says it "merges cleanly with #1316 in either order". That is true only textually. Fix: base the head on #1316's FR-294 text, or remove the revert, so that the merged tree keeps the seam types. | spec/functional/FR-294-run-every-execution-backend-behind-one-seam.md:30-60 (as merged); spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md:757-795 (as merged); spec/test-cases/TC-778-the-interpreter-implements-the-execution-seam-and-the-seam-admits.md:15-31 (as merged) | wrong-requirement |
| FND-003 | high | Scope-deferral. STD-369's scope includes the FR-294 seam types: `PrepareFailure` with no per-entry arm, `PrepareLimits` with its members and defaults, `UnsupportedEntry` and the unsupported `CallFailure` arm, ADR-029 EB-1 and TC-778. The PR hands that scope to #1316 and STD-350 ("FR-294 seam types are STD-350's"). The standing rule is that a ticket's own scope is never split. On its own, the head is left contradictory. FR-296 keeps "If the chosen backend settles an entry `Unsupported`, then `execute` shall return that outcome", and FR-279 speaks of "a `PrepareFailure` that is not an unsupported entry". Yet `Result<Evaluation<Value>, CallFailure>` has no arm for a per-entry unsupported outcome on this head. FR-279-AC-5 and FR-285-AC-6 name `prepare.code_bytes`, which nothing on main defines. Fix: specify STD-369's whole scope in this PR, or close the overlap with #1316 by consolidating. Do not split the ticket. | spec/functional/FR-296-select-the-execution-backend-in-the-request.md:82-83; spec/functional/FR-279-execute-a-checked-entry-as-a-library-operation.md:68, :116; spec/functional/FR-285-map-every-outcome-category-to-one-exit-code.md:110 | missing-requirement |
| FND-004 | medium | The new `CallFailure` arms are stated in only one place. FR-279's Outputs still lists only `CallFailure::Input`, `Fault` and `Cancelled`, but its Description adds `Limit` and `BackendNotSupplied`. FR-285's Requirements say each `CallFailure` variant "shall have the O-16 category the Description states", but FR-285's Description does not list the two new arms. Only a separate new Requirements bullet gives their categories. Fix: add both arms to FR-279 Outputs and to FR-285's Description category sentence. | spec/functional/FR-279-execute-a-checked-entry-as-a-library-operation.md:89-93; spec/functional/FR-285-map-every-outcome-category-to-one-exit-code.md:69-70, :90-94 | missing-requirement |
| FND-005 | low | "with the chosen backend's `PrepareLimits`" reads as per-backend limits, but `ExecuteLimits` holds one `prepare: PrepareLimits` that applies to whichever backend is chosen. Fix: say "with `limits.prepare`". | spec/functional/FR-279-execute-a-checked-entry-as-a-library-operation.md:63-66 | wrong-requirement |

## Verdict

Not mergeable at 8cc7b5b06. FND-001 and FND-002 together mean QSL-591
cannot reach a consistent spec by merging this PR and #1316. The designs
for an absent backend disagree, four AC ids collide, and the merge silently
reverts #1316's FR-294 seam types. FND-003 is a scope-deferral under the
standing rule. The signature, `SuppliedBackend` and the exit codes are
sound. The head has since moved to 7875f6ca, which needs a fresh review
against these findings and against #1316's FR-279-AC-4.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | efcddb17: one design. An unsupplied backend is `CallFailure::Unsupported(UnsupportedEntry { backend, cause: NotSupplied })` (FR-279, FR-296). `BackendNotSupplied` is gone. #1316 (fc63c46b) contains this PR's 7875f6ca, so FR-279-AC-4, FR-285-AC-6, FR-294-AC-5 and FR-296-AC-2..4 have one text in both PRs, and FR-279-AC-5 is gone. |
| FND-002 | fixed | efcddb17: the head again carries FR-294's seam text. `git merge-tree` of efcddb17 with #1316 fc63c46b is clean. The merged FR-294 keeps `UnsupportedCause`, `PreparedEntries::unsupported` and `PrepareLimitReached`, and has no "holds per-entry" sentence. |
| FND-003 | fixed | efcddb17: STD-369's whole scope is in this PR: FR-294's `PrepareLimits`, a `PrepareFailure` with no per-entry arm, `UnsupportedEntry`/`UnsupportedCause`, `CallFailure::{Unsupported, Limit}`, ADR-029 EB-1 and TC-778. |
| FND-004 | fixed | efcddb17: FR-279 Outputs lists `Limit(PrepareLimitReached)` and `Unsupported`. FR-285's Description gives `CallFailure::Unsupported` unsupported and `CallFailure::Limit` incomplete. |
| FND-005 | fixed | efcddb17: "prepare exactly the selected entry with the chosen execution backend under `limits.prepare`, the one `PrepareLimits` that applies to whichever backend is chosen". |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-006 | medium | `PrepareLimits` is defined only as "the translation work and the generated code size one `prepare` may use". The spec does not say what one compile work unit counts, what `code_bytes` counts, when each charge is made, or whether `&Cancel` is polled at each charge. FR-294-AC-5's test backend charges "one compile work unit per entry", so it does not constrain a real AOT or JIT. Two backends can count differently and both pass. State the counters, the charge points and the cancel poll. The same text in #1316 is SR-16220 FND-008. | spec/functional/FR-294-run-every-execution-backend-behind-one-seam.md:49,120 | missing-requirement |
| FND-007 | medium | On its own, this head has `CallFailure::Unsupported(UnsupportedEntry)` in the `quire-eval` leaf, whose `backend` is FR-296's `ExecutionBackendChoice`. But only #1316's FR-401 rows place `UnsupportedEntry`, `UnsupportedCause` and `ExecutionBackendChoice` in `quire_eval`. Merged alone, FR-401's "the leaf depends on no QSL crate" contradicts an arm that holds QSL types. Merge #1323 and #1316 together (#1316 contains this head), or carry #1316's FR-401 rows here. | spec/functional/FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md; spec/functional/FR-294-run-every-execution-backend-behind-one-seam.md:51-56 | missing-requirement |

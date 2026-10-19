---
id: SR-12203
title: "Code review of quire-spec-language PR #1053: S3 memory defaults, access orderings, access classification, fences and scheduling (FR-212, FR-219, FR-220)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@0dd3dd31d15290d493e356497bb06ce6b290f2f2; PR #1053 (git diff origin/main...HEAD, 19 files): qsl-forms/src/{lib,protocol_clause,protocol_control,syntax}.rs, qsl-forms/tests/it/protocol_control_forms.rs, qsl-replay/src/spine.rs, qsl-semantics/src/check/{footprint,mod,protocol_controls,protocol_memory,protocol_slots,refusal}.rs, qsl-semantics/src/check/protocol_memory/tests.rs, qsl-semantics/src/check/protocol_clause/{coverage,mod,types}.rs and tests/coverage.rs, qsl-semantics/src/model/{observation,state}.rs; FR-219, FR-220, FR-212, ADR-025 MA-1 to MA-4 and section 10; PR body"
review_set: subset
---

## Summary

Ticket: QSL-558 (E15a). One code-review pass with the rust-review lane folded
in, over `git diff origin/main...HEAD` at 0dd3dd31d (the branch is behind
main; only the PR diff is reviewed). Read-only: nothing was built or run.

What the diff builds:

- qsl-forms: `ParallelForm.memory`, `AttemptForm.ordering`, `FenceForm` on
  `ProtocolDeclarationForm.fences`; the `MemoryClause` and `AccessOrdering`
  construct-kind placeholders are deleted, and `Fence` becomes a covered node
  kind.
- `check/protocol_memory.rs`: `check_models` (FR-219 source rules, fence
  orderings) and `classify` (FR-220 classification, MA-3 orderings, MA-4
  shape).
- `CheckedSlots.scheduling` (FR-212).
- `footprint::derive_from` over a type environment and clauses, and
  `StateModel::from_views`, so classification runs inside the check.
- Four exhaustive-match message arms in `qsl-replay/src/spine.rs`.

Silent acceptance: no consumer reads `CheckedProtocol.memory` or
`CheckedSlots.scheduling` (grep across the workspace). Nothing claims that
package identity reflects memory, orderings or fences, and S4 emits no
protocol bodies (declared remainder, FR-383). The deleted placeholders leave
one silent drop, FND-002.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The sharing rule in `classify` requires a reader as well as a writer ("attempts in two or more branches access it, at least one writes it and at least one reads it"). FR-220 "Sharing and classification" and ADR-025 MA-1 require only that two or more branches contain the location and at least one writes it. Places are also keyed per field with objects collapsed. Together these give wrong S3 refusals in both directions. (a) Write-write: two branches each `set` the same cell `X` with `ordering relaxed` and nothing reads `v`. `Cell.v` has no reader, so both attempts classify `Local` and `ordering relaxed` refuses `ill_typed`/`operator-ineligible`. The spec makes them shared stores, for which `relaxed` is admitted. (b) A register read in its own branch (the MP/LB litmus shape): left `Ly` loads into `C1.r` and a later left attempt reads `self.r` to store a cell, while right `Lx` writes `C2.r`. Collapsed `Core.r` then has readers {left} and writers {left, right}, so it counts as shared. `Ly` becomes `Rmw` and refuses with no ordering, and under `tso`/`ra` it refuses `AccessShape` over two locations. ADR-025 section 10 says `(c1, r)` and `(c2, r)` are branch-local. The PR body declares the read rule and says EN-1 and the MA-4 pre-check "remain the exact rule". That backstop covers only what S3 admits. An S3 refusal stops the package before EN-1 runs, so a valid source stays refused. No test exercises (a) or (b). | qsl-semantics/src/check/protocol_memory.rs:282-292, 372-378, 230-266 |
| FND-002 | medium | An attempt outside every `parallel` gets no check of its `ordering`: `classify` skips it (`holders.is_empty()` → `continue`). Nothing records the ordering either. It lives on `AttemptForm` only, never in `CheckedAccess`, `CheckedAttempt` or `CheckedMemory`. Before this PR it refused `unsupported_construct`/`not-yet-implemented`, and it is now accepted and dropped. The PR body says such an ordering is "recorded unvalidated", which does not match the code. FR-220's Description says "S3 SHALL check an attempt's optional `ordering o`". MA-1 shares nothing outside a `parallel`, so by MA-3 such an attempt is local and takes no ordering. Either refuse it or record it, and correct the PR body. | qsl-semantics/src/check/protocol_memory.rs:317-331 |
| FND-003 | low | User-facing messages in the new replay arms are wrong for some causes. For an RMW with no ordering, `AccessOrdering` prints "`Fa` is a Rmw access and takes no `absent` ordering", which states the opposite of the fault. `AccessShape` prints "accesses more than one location" when the cause is a single any-object location or a membership. `{class:?}` puts Debug output into the message. | qsl-replay/src/spine.rs:801-818 |
| FND-004 | low | The classification closure passes `declaring` as both `receiver_type` and `declaring` to `footprint::derive_from`. The qsl-eval caller passes the receiver's own object type. With a subtype receiver, `populations_holding(receiver_type)` reads the supertype's populations, so a `deletes_receiver` membership write into a subtype-only population is missing from the footprint. | qsl-semantics/src/check/mod.rs:2690-2703; qsl-semantics/src/check/footprint.rs:330-338 |
| FND-005 | low | `check_models` and `classify` each rebuild the same child→parent map from `tree.controls` and walk ancestors by hand. One shared helper (ancestors of a node) would remove the duplicate. | qsl-semantics/src/check/protocol_memory.rs:103-108, 300-305 |

## Verdict

NOT MERGEABLE: FND-001 is a major. The S3 classification rule contradicts
FR-220/MA-1 and refuses valid sources at S3, where no EN-1 backstop reaches.
The remaining findings do not block on their own. FND-002 should be fixed in
the same round, because it is a silent drop of a construct that used to
refuse.

Checked and sound: the FR-219 source rules in `check_models` (outermost lookup
over the control tree, nested-differs refusal at the nested clause naming both
controls, weak join over `any`/`quorum`/`predicate`/`all … outstanding` at the
join); `Sc` recorded for a nested `parallel` with no clause, as FR-219 says; the
MA-3 admitted-ordering table; the MA-4 shape check applied only under a weak
holder and only over shared places; the refusal codes and tags in `refusal.rs`;
a footprint with no derivation refusing `not-yet-implemented` at the operation
span (confirmed: `Err(_)` → `Unimplemented { construct: "attempt footprint" }`);
`model_inputs` is always `Some` for an assembled package, so the access-views
gate does not skip classification in practice; the `derive_footprint` wrapper
keeps the qsl-eval caller unchanged; `ModelView::new` is a pure extraction;
the FR-212 `Fair`/`Adversarial` derivation from `ProtocolMemberForm::Scheduling`
(single variant); the deleted fence-refusal test replaced by the fence tests;
the TC-1065 test narrowed to the predicate join. Rust idioms: no `unwrap`/
indexing in the new production paths (0dd3dd31d), no new `unsafe`, exhaustive
matches, `Option::transpose` for the optional forms.

## New findings (disposition pass 1)

Reviewed at 121ae663286793479ad54b1136fd8e268c0ce060 (fix commits 505008530, 5bc8357cb, 47d3ace91; the merges of main are not part of this review).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | medium | Under a weak default, `classify` refuses `AccessShape` for any written any-object field or population membership, whether or not that location is shared. MA-4 is about the attempt's *shared* footprint, and MA-2 makes branch-local writes part of the same step. So a branch that creates objects in a population, or writes an any-object field, that no other branch touches is refused at S3, and no EN-1 backstop can admit it. The PR body declares this ("whatever else is shared"), and STD-218 is to restate the S3/EN-1 split. STD-218 should rule this case explicitly, or S3 should defer it to the MA-4 pre-check like the other sharing-dependent shapes. This edge does not block: it fires only for a declared weak default and only for create/delete/any-object writes. | qsl-semantics/src/check/protocol_memory.rs:215-245, 298-300, 380-390 |

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 5bc8357cb: S3 no longer infers sharing. An ordering refuses only when no access kind the footprint leaves possible takes it, and the write-write (a), MP/LB register (b) and two-cell SB (c) cases are admitted and tested. The split is declared in the PR body and goes to the spec as STD-218. |
| FND-002 | fixed | 505008530: an `ordering` on an attempt outside every `parallel` refuses `ill_typed`/`operator-ineligible` as `Local` (tested), and the PR body is corrected. |
| FND-003 | fixed | 505008530: kind names replace Debug output, an RMW with none reads "needs an `ordering`", and AccessShape reads "is not one memory access". |
| FND-004 | fixed | 505008530: `BoundAttempt.receiver` (the type `on M::T::op` names) is passed as `receiver_type`. |
| FND-005 | fixed | 505008530: shared `parent_map` and `ancestors` helpers are used by `check_models` and `classify`. |

Disposition verdict: MERGEABLE. No major remains. FND-006 is medium and does not block.

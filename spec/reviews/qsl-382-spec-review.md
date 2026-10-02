---
id: SR-1030
title: "Spec review of PR #582 (StateModel family hook, S3 dispatch, model correspondence, abstraction relation)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@18fed69d16de8665787314052900aab5cbe1c240; spec/functional/FR-300 to FR-307, spec/test-cases/TC-790 to TC-809, spec/usecase/US-012, spec/usecase/US-031, spec/spec.md, spec/tests.md"
review_set: subset
---

## Summary

Ticket: QSL-382 (pointer on QSL-388). Review of `git diff origin/main...HEAD`
at 18fed69d against ADR-016 §2, §7 and §9 (G-2a to G-2c, G-7, G-8), ADR-017
§3 (AR-1 to AR-7), ADR-020 §7 (on open PR #564), the wave-B and wave-C owner
rulings, and the QSpec counterparts FR-353, FR-450 and FR-451 on QSpec branch
`spec/wave-b-q7-adr017` (QSpec PR #171).

Lenses applied: integrity (cross-FR and FR-to-ADR consistency), QSpec
consistency (no QSpec semantics re-specified), EARS phrasing, AC-to-TC
coverage, caps and limits, pins and version tracking, compat paths,
undefined-is-refuted.

What is right: ids FR-300 to FR-307, TC-790 to TC-809 and US-031 collide
with nothing on main or any open spec branch. Every AC has a TC that tests
behaviour. Ticket ids appear only in References. No depth cap: the only bound
is FR-083's caller-configurable `family_steps`, with a published default, and
the refusal names the limit, its value and the field that raises it. There are
no pins, ledgers or compat paths. The abstraction relation stays a separate
declaration from model refinement, with its own checked type and keys, as
ADR-020 §7 MC-2 and MC-3 state. FR-301's `FamilyResult::Undefined` for
`precondition-false` is the existing S6a category (ADR-016 SC-7), not a
proof-engine verdict, so the QSL-366 ruling does not touch it.
`tools/check-index-completeness.sh` exits 0. `quire validate` on the changed
files exits 0, with one `ears:non-singular` warning on FR-303.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-304 restates QSpec FR-450's refusals and Rust spellings and the copy is incomplete. It leaves out FR-450 rule 2 (a `type` or `population` name that resolves to the wrong kind of declaration), a field named twice, a parameter named twice, a parameter mapped to the receiver's spelling, the raw-identifier exclusions (`r#crate`, `r#self` and so on), the NFC requirement, the `::` path separator and source-order reporting. An implementer following FR-304 admits bindings that QSpec refuses. Cite FR-450 for the refusal set and the spellings, and keep only the QSL placement (S3, `Relation` family, `CheckedAbstractionRelation`). | spec/functional/FR-304-check-an-authored-abstraction-relation.md:66-98 |
| FND-002 | medium | FR-304 checks "an abstraction relation" into one `CheckedAbstractionRelation`. QSpec FR-450 allows several `abstraction-decl`s in one package and makes key uniqueness span all of them (FR-450-AC-7). FR-304-AC-4 and TC-799 test conflicts inside one declaration only. | spec/functional/FR-304-check-an-authored-abstraction-relation.md:29-31, :114 |
| FND-003 | medium | QSpec FR-450 settles the surface spelling (ADR-017 Q-1), but no QSL AC parses the source form at S2. FR-450-AC-3's `unsupported_construct` without a `quire.model.complete/v1` selection and its `invalid_syntax` refusals have no QSL AC. TC-797 to TC-803 build the relation in the test, and only TC-804 compiles source. | spec/functional/FR-304-check-an-authored-abstraction-relation.md:35-38; spec/tests.md:709-712 |
| FND-004 | medium | ADR-017 is not amended to match. AR-2 still types `receiver: RustField`, while FR-304, QSpec FR-450 and FR-451 use `RustReceiver` (`self` or a parameter identifier). AR-1, AR-4, §5 and §8 still say Q-1, Q-2 and Q-7 are open, although QSpec FR-450, FR-451 and FR-353-AC-1 answer them. | spec/decisions/ADR-017-protocol-refinement-abstraction-boundaries.md:568, :559-560, :583, :631, :714-715 |
| FND-005 | medium | FR-305 says the checker "relates" a frame binding to the operation's `state`/`frame` and `state`/`operation_anchor` nodes, but no output says where that relation is held (a member of the checked binding, a lookup, or something else). FR-305-AC-2 and TC-802 cannot be asserted as written, and two implementers would build different things. | spec/functional/FR-305-relate-a-frame-binding-to-its-operation-s-frame.md:32, :46, :77 |
| FND-006 | medium | TC-791 step 2 tests the opposite direction from FR-300-AC-3. The AC says the probe fails when a `StateModel` arm is absent from its report (the arm is missing in code). The TC removes the entry from the expected list instead, so the arm is reported but not expected. Add a step that drops the S3 arm and checks that the probe names it as missing (the FR-063-AC-2 direction). | spec/functional/FR-300-check-model-forms-through-the-state-model-family.md:92; spec/test-cases/TC-791-seam-probe-reports-the-state-model-arms.md:19-20 |
| FND-007 | low | FR-302 calls the `family_steps` stop "a resource refusal" but does not name its code. FR-083 fixes it as the resource-exhaustion cause `family-steps`, so FR-302-AC-5 and TC-795 have no code to assert. | spec/functional/FR-302-link-dispatch-during-unit-compile.md:70-77, :84 |
| FND-008 | low | FR-303's Description holds three SHALL responses (`quire validate` warns `ears:non-singular`), and "SHALL produce no checked package" has no AC: the ACs test only `record`, not the compile refusal. | spec/functional/FR-303-keep-the-model-correspondence-one-to-one.md:25-29 |
| FND-009 | low | FR-302's ACs are listed in the order AC-1, AC-6, AC-2 to AC-5, and the TC-809 row in tests.md sits between TC-793 and TC-794. | spec/functional/FR-302-link-dispatch-during-unit-compile.md:79-84; spec/tests.md:280 |
| FND-010 | low | FR-300 and FR-301 name the model forms differently. FR-300 lists five forms with "a field read (attribute)", FR-301 lists `deref` and a field read as separate forms, and ADR-016 FP-1 names `deref`. | spec/functional/FR-300-check-model-forms-through-the-state-model-family.md:39; spec/functional/FR-301-render-state-model-causes-under-the-enclosing-family.md:37 |
| FND-011 | low | FR-301 Behavior ends "No `StateModel` arm is added to the S6a family kind", a statement of what is not. It should say what is: the S6a family kind is unchanged, and the cause travels in the enclosing family's result. | spec/functional/FR-301-render-state-model-causes-under-the-enclosing-family.md:57 |

## Verdict

**Not mergeable as it stands.** FND-001 (high) must be fixed: FR-304 copies
QSpec-owned refusal semantics and the copy disagrees with QSpec FR-450.
FND-002 to FND-006 (medium) should be fixed in the same round. FND-007 to
FND-011 are low.

The StateModel half (FR-300 to FR-303) matches ADR-016 G-2a to G-2c, G-7 and
G-8. FR-300 replaces G-2a's inspection with behaviour tests, which is
stronger. FR-306 and FR-307 match ADR-017 AR-4 to AR-6 and QSpec FR-451 and
FR-353-AC-3/AC-5. Merge order: FR-304 cites ADR-020 §7, which is on open PR
#564, in prose only (no `ix://` edge), so either order validates.

## New findings (disposition pass 2)

Delta reviewed: c608c0a2..63b7a058 (0a9eee4b ADR-017 AR-3 names `RustReceiver`; 102ac65c TC-808 on QSL fixtures; 63b7a058 FR-307 drops the driver SHALLs).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-012 | low | FR-307-AC-6 now restates FR-307-AC-1: the same one-bound, one-refused export over the same kind of fixture. Its two added sentences test nothing in QSL: "the bound results are the items a driver's QSpec FR-331 request holds" describes the downstream driver (agent-ix/quire-driver, ADR-011 T-13), and "verified end to end in quire-integration" is a pointer to another repo, not a criterion. Delete AC-6 and TC-808, or give AC-6 a case AC-1 does not cover. | spec/functional/FR-307-export-the-bindings-each-item-references.md:89 |
| FND-013 | low | TC-808 step 3 filters the bound results in the test itself, and Expected Result 3 asserts that filter's output, so it cannot fail when Result 2 passes. Steps 1 and 2 repeat TC-806 step 1. The file name still says `driver-sends-only-bound-items-to-cg` after the title changed to the export partition. | spec/test-cases/TC-808-driver-sends-only-bound-items-to-cg.md:22-23, :33 |

## Dispositions

Round 1, reviewed at c608c0a26baab69d95c7461046544f3e1c9172d8 (fix commit c608c0a2).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | c608c0a2 |
| FND-002 | fixed | c608c0a2 |
| FND-003 | fixed | c608c0a2 |
| FND-004 | fixed | c608c0a2. A few stale ADR-017 lines remain and are merge follow-ups: :687 (AR-6 "Q-2 relation node"), :697 (§4 "Q-1, Q-2"), :789-790 (§8 "Q-1 and Q-2 block #198's feature slice") and :801 (Consequences). #583 and #586 edit the same lines. Also :626, where the AR-3 malformed-segment bullet does not name `RustReceiver`. FR-304 is normative and cites QSpec FR-450, so none of these changes behaviour. |
| FND-005 | fixed | c608c0a2 |
| FND-006 | fixed | c608c0a2 |
| FND-007 | fixed | c608c0a2 |
| FND-008 | fixed | c608c0a2 |
| FND-009 | fixed | c608c0a2 |
| FND-010 | fixed | c608c0a2 |
| FND-011 | fixed | c608c0a2 |

Round 2, reviewed at 63b7a058b8c150c4bbb411eb1d366f01ef3bc53d. No finding had a still-open or missing outcome before this round, so no row is added. FND-012 and FND-013 are new and have no outcome yet.

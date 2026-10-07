---
id: SR-1382
title: "Spec review of quire-spec-language PR #661 (QSL-654): ADR-029 PV-4 routing check order"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@f25bef893c71c15e3d58bdd8aae854e3ef498702; PR #661 diff against origin/main; spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md PV-4 items 1 and 4; checked against quire-specification origin/main a73731e7 (FR-290, FR-331, proposals/backend-provider-v1/schema.json) and quire-specification PR #196 at 3e350dfd (ProofBound.kind)"
review_set: subset
---
# Spec review of quire-spec-language PR #661

## Summary

Ticket: QSL-654. The PR rewrites ADR-029 PV-4 items 1 and 4 to match the plan-lead ruling.

- **Item 1:** the Process descriptor is QSpec's backend-provider/v1 `BackendDescriptor`. Its `bounds` are published limit defaults, not admission maxima. The interim causes and the "pending in QSpec under QSL-637" text are deleted.
- **Item 4:** an ordered three-step check. (i) The mode is advertised for the item's kind. (ii) The item's domain kinds are in the manifest's `domains`. This always applies to a bounded item, and applies to an unbounded item only when the provider does not advertise `unbounded`. (iii) FR-290's advertised-mode rows.

Examined:
- ADR-029 PV-4 item 1, lines 478-489 (examined)
- ADR-029 PV-4 item 4, lines 505-523 (examined)
- QSpec FR-290 "Advertised mode" table and candidate rules (context_only)
- QSpec FR-331 manifest descriptor `domains`/`bounds` text and schema.json `BackendDescriptor`/`Extent` at a73731e7 (context_only)
- QSpec PR #196 `ProofBound.kind`/`DomainKind` (context_only)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Step 2 does not say whose `bounds[].kind` and `domains[].kind` it reads. Item 1 has just named the descriptor's `bounds` (a limit-name to count map with no `kind`) and `domains`, so "over `bounds[].kind`" reads as the manifest's member. The ruling means the item extent's `bounds[].kind` (`ProofBound.kind`, which only QSpec #196 adds) and the extent's `domains[].kind`. Write "the item extent's `bounds[].kind` (QSpec #196 `ProofBound.kind`)" and "the item extent's `domains[].kind`". | spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md:508-511 |
| FND-002 | medium | Step 1 names no settlement, yet "the first check that fails settles it". QSpec FR-290 already settles this case before any arm runs. Its candidate table gives `invalid-request`, `invalid_capability`/`inconsistent-candidates` for a candidate that does not advertise the item's kind, and `unsupported`, `unsupported_projection`/`unsupported-requested-capability` for an empty set. So the Process arm is reached only with exactly one candidate that advertises the kind. Either state that step 1 is FR-290's candidate table, with those two settlements, or name the cause the arm emits. As written, an implementer can't tell which cause a step-1 failure produces. | spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md:507 |
| FND-003 | medium | Step 2 does not say how an absent `domains` reads. In backend-provider/v1, `domains` is optional: it is not in `required`, and it has `minItems: 1` when present. An unbounded-only provider will often omit it. If absent means "no kinds", every bounded item with a substituted bound fails step 2 on that provider. FR-290's `bounded`/`unbounded only` row would instead give the arm's own disposition. If absent means "unconstrained", the check never fires. State which reading applies. A bounded item with empty `bounds` passes step 2 vacuously; state that too. | spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md:508-515 |
| FND-004 | low | One sentence was left over from the old text: "That descriptor is CG's own descriptor for the Process variant … CG sources `domains` and `bounds` from the FR-331 manifest the `BackendDescriptor` came from". The new item 1 says CG receives the FR-331 `BackendDescriptor` itself. "CG sources domains and bounds from the manifest it came from" is now circular, and "the `BackendDescriptor`" could mean QSpec's or `qsl_route`'s. Keep only the contrast with `qsl_route::BackendDescriptor`, for example: "It is not `qsl_route::BackendDescriptor`, which holds only (kind, mode)." | spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md:486-489 |

## Verdict

The rewrite implements the ruling as decided, apart from four gaps in the wording.

- **Item 1** matches QSpec main. backend-provider/v1 `BackendDescriptor` has `advertises`, `domains` (the four boundable ADR-014 kinds) and `bounds` (FR-331: "the published default of each limit it applies, keyed by limit name"). The ADR states that routing never compares against `bounds`. No "QSL-637", "interim" or "pending in QSpec" text remains anywhere in spec/.
- **Item 4** follows ruling (i) to (iii) in order:
  - Step 2's scoping matches ruling (ii). It always applies to a bounded item, and to an unbounded item only when `unbounded` is not advertised. It settles `unsupported-requested-capability` whatever `finite_bound_available` says.
  - Step 3 restates FR-290's two `bounded only` rows exactly.
- **Context, not a finding (decided by the ruling):** step 2 settles some items `unsupported` that FR-290's table alone would settle `requires-bound`. An example is an unbounded item whose domain kind is `quantity`, `loop` or `infinite-trace`, which no manifest `domains` can list.
- **Dependency on #196:** step 2's bounded-item check reads `ProofBound.kind`, which exists only in QSpec #196 (open). That makes the PR body's "merge after #196" order required.
- **Checks:** `quire validate` passes on ADR-029. The `make ci` log (~/dev/worktrees/logs/qsl-654-spec-ci.log) was written after the head commit and ends rc=0, with no failed tests. No code reads the ADR.

Three medium findings and one low, all wording in PV-4. **Mergeable once FND-001 to FND-004 are fixed, and after QSpec #196 merges.**

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | high | The fix round pins a QSpec commit in spec prose: "QSpec FR-290's \"Single-candidate arm\" (quire-specification `74645130bfdeebd64cec116ec5dafb0e770d47ee`)". This is a pin that guards nothing. FR-290 is already addressed by its id, the SHA only goes stale when FR-290 next changes, and no tool resolves it. Delete the parenthesised SHA and keep "QSpec FR-290's \"Single-candidate arm\" … (FR-290-AC-13)". | spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md:508-509 |

## Dispositions

Round 1, reviewed at 660e338dcbc8adb311d4cd7622edd3677ba7e581.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 660e338d |
| FND-002 | fixed | 660e338d |
| FND-003 | fixed | 660e338d |
| FND-004 | fixed | f43bd242 |

How each finding was fixed, per the plan's rulings:

- **FND-001 to FND-003.** PV-4 no longer restates the check order. Item 4 now defers to QSpec FR-290's "Single-candidate arm" (on QSpec main):
  - Its step 1 is the candidate table, which fixes FND-002.
  - Its step 2 reads `extent.bounds[].kind` and the boundable `extent.domains` kinds, which fixes FND-001.
  - Its registration rule refuses a missing or malformed `domains` with `invalid_capability`/`invalid-domains`, which fixes FND-003. Item 1 states that rule without contradicting FR-290.
- **FND-004.** The circular "CG sources `domains` and `bounds`" sentence is gone (f43bd242).

ADR-029 validates at head. **Not mergeable until FND-005 is fixed.** The fix is a one-line deletion.

Round 2, reviewed at 4fe80329bd2cce3b2802061b136ea7f6dadd98fe.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-005 | fixed | 4fe80329 |

PV-4 ruling 4 now cites QSpec FR-290's "Single-candidate arm" and FR-290-AC-13 by id, with no commit SHA. The added lines of #661's spec and docs diff contain no hex SHA. ADR-029 validates at the head commit. Every finding now reads fixed. **Mergeable.**

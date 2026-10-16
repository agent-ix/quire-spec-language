---
id: SR-11405
title: "Spec review of quire-spec-language PR #875: population manifest and closedness behind resolve"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@7a587e80f95eaec701c417294d206243626d0fab; spec/functional/FR-364-carry-relationship-instances-in-an-observation-and-read-them-for-navigation.md, spec/functional/FR-106-admit-snapshots-and-invocations.md, spec/test-cases/TC-1034-navigation-reads-an-extent-with-charges-dangling-first-and-cardinality-refusals.md, spec/test-cases/TC-1038-population-manifest-decides-dangling-and-undecidable-for-resolve.md"
review_set: subset
---
# Spec review of quire-spec-language PR #875

## Summary

Ticket: QSL-800. One pass of spec-review with the contradiction, ambiguity,
completeness, testability and spec-integrity sub-analyses over the PR diff.
Spec-only PR, so no code review. Only major issues block. Mediums and lows
are recorded in the Verdict.

Oracles: FR-364 and FR-106 (checks 6.8, 7, 8 and 11.5) as merged, FR-082,
FR-084, FR-255, FR-362, FR-363, TC-1033, TC-1034 and TC-1035 as merged,
quire-semantic-value FR-119 on main, and quire-specification FR-152
"Navigation" on main (cited by id only).

Examined:
- FR-364 Description, Inputs, condition 3, the dangling-ends paragraph, the
  manifest, covering, closing, `resolve`, bundle and store-boundary sections,
  the read-contract `resolve` row, the navigation outcome paragraph and the
  refusal table rows (examined)
- FR-364-AC-10 to FR-364-AC-14 (examined)
- FR-106 Outputs manifest sentence, check 8 addition, FR-106-AC-33 (examined)
- TC-1038 rows 1 to 20, its fixture variants and its Expected Results (examined)
- TC-1034 row 10 and its model-layer note (examined)
- FR-106 checks 6.1, 6.8, 7 and 8; FR-084 binding rule; FR-255 Description;
  FR-362 items 4 and 5; QSV FR-119 Behavior 2, 5 to 9; QSpec FR-152
  Navigation (context_only)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The manifest row limit contradicts FR-255. FR-364 makes the manifest `closed` only when the enumerator is exhausted "within its row limit", and a row-limit stop gives a `partial` manifest. Every missing reference then resolves `undecidable` and navigation returns `incomplete_population`/`incomplete-scope`. The limit has no setting name and no default, and reaching it produces no `stage_limit_exceeded` outcome naming a setting. FR-255 requires every configurable resource limit to have a stable setting name, and every reached limit to tell the caller how to run again under a larger one. As written, a package with more population declarations than an unstated number silently turns every `dangling` into `incomplete-scope`. Two implementers who pick different limits get different results for the same input. Give the limit a setting, for example `observation.populations` in FR-106's `ObservationLimits` and in FR-255's table, with its default. Alternatively, state that the limit is the package's declaration count, so the authenticated enumerator always exhausts and `partial` comes only from a failure. | spec/functional/FR-364-carry-relationship-instances-in-an-observation-and-read-them-for-navigation.md:194-203 |
| FND-002 | high | The bundle key does not identify the observation, so a manifest can be paired with another observation's closure under one source and clause. The key is the source reference plus the selection: the selected clause and, for a snapshot-built observation, "the digest of each selected document". A model-layer key is therefore the same for every observation built under one source and clause, such as the pre and post observations of a postcondition, or two states. The constructor accepts the manifest of one observation with the closure of another. In the model layer a bound population is a complete row and an unbound one an incomplete row. If the pre observation binds `Spare` and the post does not, a missing `Pump` of the post resolves `dangling` instead of `undecidable`, which is the wrong result. "Each selected document" also does not say whether it means this observation's document or every document of the selection; read as every document, pre and post snapshot bundles share a key too. The section's closing claim ("A manifest from another source or selection therefore never classifies a missing reference") holds only for those two components. Add the observation (its role, or its own document digest or state identity) to the key and to FR-364-AC-14. | spec/functional/FR-364-carry-relationship-instances-in-an-observation-and-read-them-for-navigation.md:225-238 |

## Verdict

**FAIL: 2 blocking findings (FND-001, FND-002).**

All 20 TC-1038 rows were traced through FR-364's conditions, the five
`resolve` rules and the navigation order. For rows 18 to 20 the trace also
followed FR-106 checks 6.1, 6.8, 7 and 8. Every row is reachable and gives
its stated result:

- Rows 1 to 4: the manifest order Q before Spare holds, because a
  DeclarationKey orders by package and node string. Row limit 2 over two rows
  closes, row limit 1 is partial with one row, and a failure at the second
  pull is partial with one row. These agree with QSV FR-119 Behavior 6 and 7.
- Rows 5 to 11: rule 4 for `p9` under **spare**, `t9` dangling, Valve and
  Part uncovered (rule 3), and the unknown type (rule 2) are all reached. Rule 2
  is reachable only through a direct `resolve`, because condition 3 now refuses
  such an end. Row 11 does exactly that, so the rule is not vacuous.
- Rows 12 to 14: the resolve check comes before each visit, giving 1, 1 and 4
  work units, and there is no `above-maximum` refusal.
- Row 15: whichever of `p9` and `t9` sorts first by reference key, the
  **spare** and **spare-tank** variants give opposite kinds. Both
  `incomplete-scope` and `dangling_reference` are really produced across the
  pair, after `model.navigate` alone. Condition 3 admits both ends, because
  `Pump` and `Tank` conform to `Part`.
- Row 16: Valve is uncovered, so the result is `incomplete-scope`. Row 17:
  condition 3 refuses the absent `t9`, admits the absent `p9`, refuses the
  held `t1`, and refuses an unknown type.
- Row 18: `Spare` is not a required population, so B admits and the manifest
  rows hold. Row 19: the end skips check 6.8, check 8 gives dangling with `Q`
  complete, and with `Q` incomplete check 7 gives Incomplete naming `Q`. Row
  20: the part order manifest, closure, types names `types`, then `closure`.

Other checks:
- No contradiction with FR-106 checks 7, 8 and 11.5, FR-082, FR-362, FR-363
  or TC-1033, TC-1034 and TC-1035 as merged. Condition 3 matches check 6.8,
  which type-checks only listed objects.
- QSV FR-119 on main: the author's claim holds. It has a three-way `resolve`
  with Present first, set-valued covered types, `Closed` only on `Ok(None)`
  exhaustion, `Partial` giving Undecidable, and the end type check held or
  not, including unknown types. The four questions for luna are consistent
  with FR-119 as written: (a) FR-119 already disclaims source pairing and
  keeps `ptr::eq` only; (b) an unbound row is an ordinary `complete: false`
  entry; (c) the unknown-type check comes before covering (Behavior 5); (d)
  the row limit is open, see FND-001.
- No test parses FR-106, FR-364, TC-1034 or TC-1038, because only FR-092,
  FR-093, FR-094 and FR-255 are read by tests. `spec/spec.md` is untouched, so
  every relationship entry keeps `type: contains`. The diff has no ticket or
  PR ids and no tracking, hash or version ceremony; the key digest is FR-362's
  content identity. TC-1038, FR-106-AC-33 and FR-364-AC-10 to AC-14 collide
  with nothing on main or in any open PR. The QSL-798 and QSL-790 spec PRs
  are not open yet, so they must number after these.
- The author's choices 1 to 5 and 8 to 11 are internally consistent and
  unsurprising. Choice 6 is FND-001. Choice 7 is FND-002 for the key's
  content; the mismatch code is fine.

Recorded (not blocking):
- medium: The TC-1034 model-layer note says a snapshot "returns Incomplete
  for row 10 at check 7". After the re-point, `p9` has no population. A
  snapshot end naming `Q`/`p9` refuses at check 8, and only one naming
  `Spare`/`p9` gives check 7. Row 10 itself and FR-364-AC-8 still hold.
- medium: Rule 4 (a complete and an incomplete population both covering the
  type gives `undecidable`) refines QSpec FR-152's Navigation dangling rule,
  which does not say which covering population is the target's. It is
  consistent, but worth a QSpec clarification.
- medium: FR-364-AC-14 lists "another selected document digest", but no
  TC-1038 row exercises it.
- medium: AC-14 says "a key other than the others'", while the FR compares
  each part with the selected key. Three parts that agree with each other but
  not with the selected key are covered only by the FR.
- medium: FR-364-AC-10's "no operation closes a manifest ... on request" is an
  inspection claim verified as Test only. No TC-1038 row can fail it. Make it
  Test + Inspection, as QSV FR-119-AC-15 does.
- low: FR-364 does not state what happens when the enumerator yields a
  duplicate population or an unknown covered type (QSV `ManifestFailure`).
- low: the store-boundary section does not list the type environment among
  what the model layer passes.

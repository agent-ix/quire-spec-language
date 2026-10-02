---
id: SR-1214
title: "QSL-579 gap analysis of PR #597 (FR-238, FR-245 DBM half, TC-693, TC-700)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@f26315fe45119052cc22bc46e56a8834868cd091; PR #597 diff against origin/main; spec/functional/FR-238-represent-zones-as-difference-bound-matrices.md; spec/functional/FR-245-check-a-zone-certificate-in-the-qualified-core.md; spec/spec.md; spec/tests.md; tools/arch-lint/api_surface.rs (context)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-238
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-245
    type: reviews
---
## Summary

Ticket: QSL-579. PR: quire-spec-language#597.

Trace:
- FR-238-AC-1: `tc_693_constants_scale_by_the_lcm_of_their_denominators`
  and `tc_693_a_huge_constant_scales_and_computes_without_overflow`
  (scale.rs). The second uses 10^30 * 7 bounds and asserts exact values.
- FR-238-AC-2: `tc_693_zero_up_constrain_reset_give_the_stated_zones` and
  `tc_693_includes_orders_a_strict_bound_inside_its_non_strict_twin`.
- FR-238-AC-3: `tc_693_operations_agree_with_the_grid_reference`, 10,000
  random DBMs, windowed enumeration (accepted by ruling).
- FR-245-AC-4: the 18 Kani harnesses in certificate/zone/proofs.rs cover
  close, constrain, reset, up, includes and aLU at dimension 3 or less,
  bounds in [-8, 8]. The two unit tests are tagged TC-700 / FR-245-AC-4.
- FR-245 CF-6 ("its own DBM code"): met, per SR-1213's independence check.
- FR-245-AC-1, 2, 3 and 5 are outside this slice. They need the checker.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `qsl-analyze` depends on `quire-exact` but is missing from arch-lint's `qsl_scan_src_roots`, the crate list the T12-B/C/D constructor rules scan. The list's own doc says "Each later layer crate joins this list when it is extracted". So a `NodeKey`, `EffectiveId` or `PopulationId` constructor call added in qsl-analyze would pass the API-surface check unseen. Add `"qsl-analyze/src"`, and add it to the list in that check's test fixture. | tools/arch-lint/api_surface.rs:1195-1232 |
| FND-002 | low | The two promises "nothing in layers K to 6 depends on" `qsl-analyze` and "the checker never calls the engine's DBM" (CF-6) hold today only by inspection. No test fails if `qsl-replay`, or any core crate, gains a `qsl-analyze` dependency, and that is the exact change that would break the checker's independence. Add a `cargo metadata` assertion, in the style of `tests/it/family_outcome_layering.rs`, that no layer K to 6, F, SV or R crate depends on `qsl-analyze`. | qsl-analyze/src/lib.rs:2-9; qsl-replay/src/certificate/zone.rs:5-11 |
| FND-003 | medium | The status rows were not updated. In spec/spec.md, FR-238 still reads "not yet implemented -- TC-693 planned" and FR-245 "not yet implemented -- TC-700 planned". In spec/tests.md, TC-693 and TC-700 are "🚧 Planned". FR-238 is implemented (with the ruled amendments still to apply), and FR-245-AC-4 passes for the DBM half. Update the FR-238 and TC-693 rows, and mark TC-700 as AC-4 passed locally with AC-1, 2, 3 and 5 planned. | spec/spec.md:1191; spec/spec.md:1198; spec/tests.md:463; spec/tests.md:470 |

## Verdict

Changes requested: FND-001 and FND-003 are medium, FND-002 is low.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | The new `no_core_crate_depends_on_qsl_analyze` is traced `#[trace("TC-700", "FR-245-AC-4")]`, but FR-245-AC-4 is "the Kani harnesses ... pass for dimension at most 3 with bounds in [-8, 8]"; this test checks dependency edges, which AC-4 does not state. It backs FR-245's "DBM code separate from EN-6's" (Behavior, CF-6), which no AC holds. Either add an FR-245 AC for the separation and retarget the trace, or drop the FR-245-AC-4 tag. | tests/it/family_outcome_layering.rs:681-688; spec/functional/FR-245-check-a-zone-certificate-in-the-qualified-core.md:76 |

## Dispositions

Round 1, reviewed at `3653e30e5fc134c8090168417efd3a099657cc80` (fix commit cf5e9a89).
The ruled spec edits are in place:
- FR-238's Outputs now give `constrain`, `reset` and `includes` their
  `Result<_, DbmError>` signatures and list `DbmError`.
- A new Behavior SHALL says a refused operation returns `DbmError`, leaves
  the DBM unchanged and never panics. AC-2 and its test now assert
  `ClockOutOfRange`, `ResetReferenceClock` and `DimensionMismatch`, and
  that the DBM is unchanged after each refusal.
- FR-238-AC-3 and TC-693 step 3 describe the windowed `1/dim` grid as the
  test implements it: a seeded anchor, least corners, past the looser entry
  for a refused inclusion, emptiness checked both ways, and a canonicity
  re-close.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | cf5e9a89d9dce876f7d54fe985996c567129bd88 |
| FND-002 | fixed | cf5e9a89d9dce876f7d54fe985996c567129bd88 |
| FND-003 | fixed | cf5e9a89d9dce876f7d54fe985996c567129bd88 |

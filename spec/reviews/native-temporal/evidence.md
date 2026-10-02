---
id: SR-378
title: "Native temporal verification and evidence review"
type: SpecReview
analysis: evidence
scope: "FR-043, FR-044, FR-045, NFR-008, TC-122-125, TM-008, docs/native-temporal-evaluation.md"
review_set: subset
evaluated_revision: "87bc83f5fad724413737e0a707cf80bdbcba931a"
previous_evaluated_revision: "4c1eee8646b51e00cd141d15eb110a33354b8475"
review_date: "2026-09-11"
---

## Summary

Re-run of this review against 87bc83f, which reworked the temporal scope in
response to the sixteen findings recorded at 4c1eee8. Thirty-two acceptance
criteria across FR-043, FR-044, FR-045 and NFR-008 and five NFR-008 metric rows
were re-examined, together with the newly published counter contract in
`docs/native-temporal-evaluation.md`. All three high findings are resolved: the
metric table now uses catalog ids and honours NFR-040's method assignment row for
row, NFR-008 publishes eight counters with kinds, units and defaults, and the
evaluator now owns the retained-state table that eviction is forced through. The
installed advisor and method catalog were run for the first time on this scope
and their complete outputs are recorded under `data/`. Ten residual findings
remain, none high; the most consequential is that the newly added Inspection,
Analysis and Demonstration methods are invisible to the advisor and name no
evidence artifact of their own.

### Disposition

| Prior finding | Status at 87bc83f | Evidence |
| --- | --- | --- |
| FND-001 weakened eviction method | Resolved | Metric row 3 is `model-based-test-generation`; all five rows carry catalog ids and name a population. |
| FND-002 no counter definitions | Resolved | NFR-008 Counter definitions lists eight counters; `docs/native-temporal-evaluation.md` publishes kinds, units and defaults under `quire.native.temporal-work/1`; `src/temporal/budget.rs` lands `Limits`, `Usage` and `Dimension` with the same eight. |
| FND-003 eviction not falsifiable | Mostly resolved; residual FND-002 below | NFR-008 Scope gives the evaluator its own retained-state table and separates agent F's storage; the eviction selection rule and trigger are still unpublished. |
| FND-004 mutation overclaim | Resolved | NFR-008 Verification states mutation testing is not performed by this revision; TM-008 Overview and the NFR coverage row both record it outstanding on #38. |
| FND-005 "Property Test" label | Resolved as a label; residual FND-004 below | Catalog ids throughout; the technique under row 1 is exhaustive enumeration rather than generation. |
| FND-006 AC-5 unmeasured | Resolved | Metric row 5 covers charged steps after the first unaffordable charge, over the zero/exact/one-short/clamped population. |
| FND-007 AC-3 tautology | Resolved | FR-044-AC-3 is `Analysis, Test (TC-123)`; TC-123 group 3 pairs the mutation control with a compile-fail doctest. Caveat recorded as FND-008 below. |
| FND-008 universal negatives | Resolved | FR-043-AC-1/AC-3 carry Inspection, FR-043-AC-2/AC-13 and NFR-008-AC-1/AC-4 carry Analysis, FR-045-AC-1 carries Inspection. |
| FND-009 non-reuse unobserved | Resolved | FR-043-AC-13 and NFR-008-AC-4 require the earlier result neither reused nor rewritten, carry Analysis, and add an unchanged-configuration identity control. |
| FND-010 TC-122 group 3 inexecutable | Resolved | The three profile comparisons compile separately per profile alias; group 3 mutates only profile identity, revision and clock binding name; the residual gap is recorded in FR-043 Dependencies and TM-008. |
| FND-011 evaluation-order probe | Resolved | FR-044-AC-6 is `Demonstration, Test (TC-123)`; TC-123 groups 6 and 9 inspect a per-instance evaluation counter. |
| FND-012 blocked bridge criterion | Resolved | Moved to FR-045, a total function over FR-095's table consulting no backend report; TC-125 exercises every table row. |
| FND-013 unnamed ticket | Resolved | #38 is named in NFR-008, FR-043, FR-045, TC-124, TC-125 and TM-008. |
| FND-014 unowned TC-123 refusals | Resolved | The `self`, `result` and `pre(expr)` assertions are gone from TC-123. |
| FND-015 dropped history term | Resolved | Row 3 reads "retained valuation or capture". |
| FND-016 absent data files | Resolved | `data/methods.json`, `data/advice.json` and both stderr files are authored by this review from the installed tools. |

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The advisor reads the Verification column as a single value, so the new non-test methods are machine-invisible: FR-044-AC-3 and FR-044-AC-6 are `uncatalogued: true` because Analysis and Demonstration lead their cell, while FR-043-AC-1/2/3/13 and NFR-008-AC-1/4 parse as bare `Test` and their Inspection and Analysis halves are dropped entirely. | FR-043-AC-1; FR-043-AC-2; FR-043-AC-3; FR-043-AC-13; FR-044-AC-3; FR-044-AC-6; NFR-008-AC-1; NFR-008-AC-4; data/advice.json | correct-requirement-no-evidence |
| FND-002 | medium | TC-124 group 3's generated eviction schedules cannot be realized: no document publishes an eviction selection rule or an injection seam, so with only a retention ceiling as the knob the test cannot place an eviction point inside the required set — the victim is chosen by unpublished policy. | NFR-008-AC-3; NFR-008-M-3; TC-124 | correct-requirement-no-evidence |
| FND-003 | medium | Metric row 1's "enumerated nested-interval population" and TC-124 group 1's "declared finite population" are declared nowhere; the denominator is named but never enumerated, unlike row 5 whose population is fully determined as eight dimensions by four ceilings. | NFR-008-M-1; TC-124 | correct-requirement-no-evidence |
| FND-004 | low | Row 1 labels an exhaustive enumeration `property-based-testing`; the repository carries no property harness in `[dev-dependencies]` and TC-124 disclaims randomized campaigns. The technique is defensible and arguably stronger, but the catalog id overstates it and the judgment is not recorded where SR-092 and SR-105 record theirs. | NFR-008-M-1; TC-124 | wrong-requirement |
| FND-005 | low | Eviction may have no trigger other than the retention ceiling. If so, NFR-008-AC-2's ceiling stop and AC-3's eviction are one event with two required payloads, and row 3's silent-eviction target is unreachable by construction rather than by evidence. | NFR-008-AC-2; NFR-008-AC-3; NFR-008-M-3 | wrong-requirement |
| FND-006 | low | NFR-008 Scope extends to FR-045's mapping classification, but no metric row, acceptance criterion or TC-125 group charges a counter or exercises a ceiling against the classifier; none of the eight counters has a meaning there. | NFR-008; FR-045; TC-125 | correct-requirement-no-evidence |
| FND-007 | low | `Usage`'s landed doc comment names instances, depth and horizon as the peak counters and omits retention, while NFR-008 and the published contract make retention peak; TC-124 group 5 requires reported usage to distinguish peak from cumulative. | NFR-008; docs/native-temporal-evaluation.md; src/temporal/budget.rs | implementation-bug-despite-evidence |
| FND-008 | low | A `compile_fail` doctest passes on any compilation error, so FR-044-AC-3's Analysis half is satisfied by an unrelated failure unless the doctest pins the expected error or the criterion records it as a smoke control. | FR-044-AC-3; TC-123 | correct-requirement-no-evidence |
| FND-009 | low | FR-043's Behavior still forbids substituting a period, epoch, unit or sequence authority, but AC-3 was narrowed to profile identity, revision and clock binding name and no criterion verifies non-substitution for the other four; the gap is recorded in FR-043 Dependencies and TM-008 but the Behavior sentence carries no marker. | FR-043; FR-043-AC-3; TM-008 | correct-requirement-no-evidence |
| FND-010 | low | The Analysis discharging non-reuse rests on the evaluator holding no cross-invocation cache, which no document states; `docs/native-temporal-evaluation.md`'s API section implies a pure call but never asserts the absence of a cache. | FR-043-AC-13; NFR-008-AC-4; docs/native-temporal-evaluation.md | correct-requirement-no-evidence |

## Verification method disposition per criterion

Of the thirty-two criteria in scope, twenty-three are discharged by an automated
Rust test alone. Nine now carry a second method, and every one of those nine is
the right call; the residual problems are machine-readability (FND-001) and, in
three cases, an unnamed artifact.

| Criterion | Declared at 87bc83f | Disposition |
| --- | --- | --- |
| FR-043-AC-1 | Test (TC-122), Inspection | Correct. TC-122 group 1 inspects the emitted graph for distinct `Constant` and `Holds` nodes, so the Inspection has a concrete subject. |
| FR-043-AC-2 | Test (TC-122), Analysis | Correct method; the Analysis is discharged by the pairwise identity comparison in group 2, which is a test step, so the Analysis adds nothing the test does not already do. |
| FR-043-AC-3 | Test (TC-122), Inspection | Correct. Group 3 inspects the refusal value for a substituted selection and requires `positions` usage zero. |
| FR-043-AC-4 to AC-12 | Test (TC-122) | Achievable. AC-4's dual agreement, AC-5's authority mismatch, AC-7's five bases, AC-10's one-axis substitutions and AC-12's contradiction refusal are all observable through the public result. |
| FR-043-AC-13 | Test (TC-122), Analysis | Correct method, unnamed artifact. See FND-010. |
| FR-044-AC-1, AC-2, AC-4, AC-5, AC-7, AC-8, AC-9 | Test (TC-123) | Achievable. |
| FR-044-AC-3 | Analysis, Test (TC-123) | Correct. The compile-fail doctest is a real mechanism; see FND-008 for its limit and FND-001 for its advisor visibility. |
| FR-044-AC-6 | Demonstration, Test (TC-123) | Correct. The per-instance evaluation counter gives the Demonstration a concrete instrument, and groups 6 and 9 exercise it across incremental re-evaluation and restoration. |
| FR-045-AC-1 | Test (TC-125), Inspection | Correct. Group 1 inspects the classifier's inputs for a consulted backend report. |
| FR-045-AC-2 to AC-5 | Test (TC-125) | Achievable, and cheaply so: the classifier is a total function of three declaration facts. |
| NFR-008-AC-1 | Test (TC-124), Analysis | Correct. Group 1's independently computed checked horizon is a test oracle; the Analysis covers the no-wrap, no-saturate, no-narrow universal, whose artifact should be named. |
| NFR-008-AC-2, AC-3, AC-5 | Test (TC-124) | AC-2 and AC-5 achievable. AC-3 is achievable only once FND-002 is answered. |
| NFR-008-AC-4 | Test (TC-124), Analysis | Correct method, unnamed artifact. See FND-010. |

## Metric method conformance to NFR-040

Every row now honours its source row, method for method and in substance.

| Row | NFR-040 method | NFR-008 method | Disposition |
| --- | --- | --- | --- |
| Accepted overflow | property-based-testing | property-based-testing | Label honoured; technique is exhaustive enumeration over an undeclared population. FND-003, FND-004. |
| Boolean after exhaustion | fault-injection | fault-injection | Honoured. Five forced stops are enumerated and the population is named; TC-124 group 2 requires the truth field absent, which is stronger than "never true or false". |
| Silent eviction | model-based-test-generation | model-based-test-generation | Label honoured; the generating model and the eviction seam are unpublished. FND-002, FND-005. |
| Stale reuse | integration-testing | integration-testing | Honoured, and now falsifiable: group 4 varies ceiling, binding and declared clock parameter and pins the unchanged-configuration identity. |
| Charged steps after refusal | — (NFR-008 addition) | negative-abuse-testing | Correct id and the same method NFR-006/007 use for charge-before-work; population fully determined. |

The advisor marks all five rows `mismatch: true` for the single reason that the
quantified-threshold rule selects `performance-benchmarking`. That
recommendation is declined by explicit review judgment, on the same grounds
SR-092 and SR-105 declined it and on NFR-040's own instruction that a generic
performance benchmark cannot prove a zero-threshold semantic metric. None of the
five is `uncatalogued` or `inconclusive`.

## Falsifiability of the zero-target metrics

- **Accepted overflow.** Falsifiable. Group 1 rejects before any position is
  visited and compares each accepted composed horizon against an independently
  computed checked value, so a wrap or saturation reads non-zero. Its population
  is undeclared (FND-003), so the metric's denominator is not yet fixed.
- **Boolean after exhaustion.** Falsifiable over the five named stops, and
  honest about that boundary: the population is stated in the metric itself, so
  the row no longer claims anything about unexercised paths. What it cannot do
  is detect a Boolean introduced into a path outside those five — which is
  exactly the gap mutation testing on #38 is assigned, and that assignment is now
  recorded rather than implied.
- **Silent eviction.** Not yet falsifiable, for a narrower reason than before.
  The mechanism now exists and is owned, but the test cannot choose which record
  is evicted, so the generated schedule that the method name promises cannot be
  applied (FND-002), and if the ceiling is the only trigger the target is
  unreachable by construction (FND-005).
- **Stale reuse.** Falsifiable. The unchanged-configuration control gives the
  metric a positive arm, and re-reading the prior result's bytes in TC-122
  group 12 gives "not rewritten" an observation. "Not reused" still rests on
  Analysis rather than observation, which is the correct method but needs the
  artifact of FND-010.
- **Charged steps after refusal.** Falsifiable. Eight dimensions by four
  ceilings is an enumerable population, reported usage is compared against
  independently derived charges, and the clamp and zero-preservation rules are
  published.

## TC-124 group-by-group

1. **Overflow enumeration.** Produces row 1's evidence. Reported `positions`
   usage zero makes "before any position is visited" observable rather than
   asserted, and the independent checked-value comparison is a real oracle. The
   population needs declaring.
2. **Five forced stops.** Produces row 2's evidence. Each stop names a published
   `Dimension`, so the expected value comes from the contract rather than the run.
3. **Eviction schedules.** Does not yet produce row 3's evidence. See FND-002.
4. **Result identity.** Produces row 4's evidence for the observable half.
   Changed clock binding, changed declared clock parameter, admitted restoration
   and the clamp-versus-zero control are all concrete.
5. **Ceiling permutations.** Produces row 5's evidence. All eight dimensions are
   published with defaults, `Limits::bounded` preserves zero and clamps above the
   default, and the peak-versus-cumulative distinction is required of reported
   usage — which is where FND-007 bites.

## Deterministic advice and judgment

`quoin catalog methods --json` and `quoin advise --json` were run from the
installed toolchain at this revision; both exited zero with empty stderr, and
both stderr files are retained alongside their outputs. Quire independently
reports 0.31.0. `quoin advise` has no scope flag, so it was run repo-wide over
336 obligations and `data/advice.json` retains the 37 in-scope records verbatim;
`data/methods.json` is the complete 33-entry catalog, unfiltered.

Of the 37 records, 30 have `mismatch`, `uncatalogued` and `inconclusive` all
false. Five are the metric rows discussed above. Two — FR-044-AC-3 and
FR-044-AC-6 — are `uncatalogued: true` solely because the authored cell leads
with `Analysis` and `Demonstration`; the six criteria that lead with `Test` parse
as a bare `Test` and their second method is silently discarded. Nothing in the
scope is `inconclusive`, so no obligation went unmatched.

The advisor's recommendations were read as recommendations. It offers
`model-checking` and `runtime-monitoring` on FR-043-AC-1/4/5/6/7/9 and
NFR-008-AC-2/3 from temporal wording; by judgment these are evaluations of an
emitted artifact against a supplied trace, not a model checker or a deployed
monitor, and TC-122 and TC-124 remain integration and enumeration suites. It
offers `dast` on FR-043-AC-8 and `sca-sbom` on FR-044-AC-2 from incidental
vocabulary; neither applies to a compiler evaluating its own artifact, and this
scope changes no dependency. `formal-analysis-smt` on FR-043-AC-12 and
FR-044-AC-2 is declined: a contradiction refusal is observed at runtime, not
proved. `golden-approval-testing` on FR-044-AC-3 is partially taken up — the
byte-identical retained capture record is a golden comparison against the
activation-time record, not against a stored snapshot file.

## Evidence artifacts

| Obligation group | Method | Artifact |
| --- | --- | --- |
| FR-043-AC-1 to AC-13 | test, inspection, analysis | `tests/composed_temporal_evaluation.rs` per TC-122; graph inspection in group 1; identity comparison in groups 2 and 13 |
| FR-044-AC-1 to AC-9 | test, analysis, demonstration | `tests/composed_temporal_activation.rs` per TC-123; compile-fail doctest in group 3; per-instance evaluation counter in groups 6 and 9 |
| FR-045-AC-1 to AC-5 | test, inspection | `tests/composed_temporal_mapping.rs` per TC-125; classifier input inspection in group 1 |
| NFR-008-AC-1 to AC-5, M-1 to M-5 | test, analysis | `tests/composed_temporal_limits.rs` per TC-124; counters published in `docs/native-temporal-evaluation.md`; `src/temporal/budget.rs` |
| Advisor and catalog | inspection | `data/advice.json`, `data/methods.json`, `data/advice.stderr`, `data/methods.stderr` |

Three Analysis obligations have no artifact of their own: FR-043-AC-13,
NFR-008-AC-1 and NFR-008-AC-4. Each needs a named location — a paragraph in the
temporal evaluation contract asserting the absence of a cross-invocation cache,
and a recorded reading of the checked-arithmetic paths — rather than a method
label that points back at the test it was added to supplement.

## Verdict and provenance

**PASS** for evidence readiness of FR-043, FR-044, FR-045 and NFR-008 at this
revision. Every criterion has a verification method that matches what can
actually establish it, every NFR-008 metric row honours the NFR-040
method assignment held as a copy in this repo, every metric names its population, the counters those metrics
are measured against are published with kinds, units and defaults, and the two
obligations this revision does not discharge — mutation adequacy and
artifact-side checking of declared clock parameters — are recorded as outstanding
on #38 in the requirement, the test case and the matrix alike, with no metric row
or coverage row claiming either.

The ten residual findings are correctness-of-record issues, not evidence gaps of
the kind that failed the previous revision. FND-002 is the one that should be
answered before TC-124 is written, because group 3 cannot be authored against an
unpublished eviction rule. FND-001 should be answered before the advisor is used
as a gate on this scope, because six criteria's second method is currently
invisible to it.

No FR, NFR, TC, matrix, doc or source file was edited by this review; the only
files authored are this document and the four `data/` files. The installed
`quoin catalog methods --json` and `quoin advise --json` were executed and their
complete outputs retained; no Cargo build, test run, mutation tool or fuzz
campaign was executed, and no execution result is claimed. All TM-008 rows remain
Planned. `src/temporal/` now lands `budget.rs`, `profile.rs` and `result.rs`;
these were read to check that the published counters exist as specified, not
reviewed for correctness, which belongs to a code review.

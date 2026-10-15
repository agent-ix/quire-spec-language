---
id: SR-11262
title: "Spec review of quire-spec-language PR #868: relationship instances in snapshots and runtime navigation"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@710607886751a37f12a9a752f4b7c5bf04ad952d; spec/functional/FR-364-carry-relationship-instances-in-an-observation-and-read-them-for-navigation.md, spec/functional/FR-106-admit-snapshots-and-invocations.md, spec/functional/FR-103-admit-model-operations-and-frames-on-the-spine.md, spec/functional/FR-085-resolve-relationship-end-references.md, spec/test-cases/TC-1033-relationship-extents-admit-and-read-in-reference-key-order.md, spec/test-cases/TC-1034-navigation-reads-an-extent-with-charges-dangling-first-and-cardinality-refusals.md, spec/test-cases/TC-1035-snapshots-carry-relationship-instances-and-admission-refuses-each-defect.md, spec/test-cases/TC-458-spine-admits-model-operations-and-frames.md, spec/spec.md"
review_set: subset
---
# Spec review of quire-spec-language PR #868

## Summary

Ticket: QSL-794. One pass of spec-review with the contradiction, ambiguity,
completeness, testability and spec-integrity sub-analyses over the PR diff
(`git diff origin/main...HEAD`). Spec-only PR, so no code review. Only major
issues are reported; mediums and lows are recorded in the Verdict.

Oracles: FR-106 checks 1 to 11 in order, FR-103 frame rules, QSpec FR-152
"Navigation", QSpec TC-197 Y07 to Y09, QSpec `value-accounting.md`
(`model.navigate` schedule), QSpec FR-154 relationship slot identity, QSpec
FR-013 and FR-340 (relationship `modifies` grant), open PR #867 (FR-363,
TC-1031, TC-1032) and merged PR #866 (FR-106-AC-18..22, TC-1029).

Examined:
- FR-364 statement, extent model, admission conditions 1 to 4, dangling rule, read contract, navigation steps 1 to 5, refusal table, AC-1 to AC-9 (examined)
- FR-106 `relationships` member grammar, walk-order paragraph, required-population extension, checks 6.6 to 6.9, check 7 and 8 additions, check 11.5, values limit sentence, AC-23 to AC-27 (examined)
- FR-103 `modifies` relationship grant and FR-103-AC-2; TC-458 step 2 (examined)
- FR-085 one sentence; spec.md index rows (examined)
- TC-1033 rows 1 to 10, TC-1034 rows 1 to 12, TC-1035 rows 1 to 29 (examined)
- QSpec FR-152, TC-197 Y07 to Y09, value-accounting, FR-154, FR-013, FR-340 (context_only)
- PR #867's FR-363 and PR #866's FR-106 edits (context_only)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | TC-1035 frame rows 25 and 26 expect results an earlier check pre-empts, and rows 23 to 27 select no clause that runs check 11. Rows apply one mutation to base B, and B's `Q` already holds `p3`, so row 26's "p3 created, `created [{Q, p3}]`" makes `p3` a surviving object and check 11.4 refuses `population_delta_mismatch`/`delta-disagreement` before 11.5 can report `p3`-`t1` `added`. Row 25 deletes `p2` from `Q` but keeps B's composition instance `s1`-`p2` in the post snapshot, so check 8 refuses `dangling_reference` on the post before check 11 runs, not "admitted". The fixture adds only the invariant `SysOk` and the operation `rehome`; check 11 runs only for an `Invocation` selected for a postcondition, and an `Invocation` selecting an invariant refuses at check 2. Fix: declare a postcondition on `rehome` and select it; state the pre and post populations of each frame row (row 26's pre without `p3`; row 25's post composition without `s1`-`p2`). | spec/test-cases/TC-1035-snapshots-carry-relationship-instances-and-admission-refuses-each-defect.md:25-39,65-69 |
| FND-002 | high | TC-1034's model-layer note names the wrong rows. It says "Rows 4 and 11 build their extents through the model-layer entry ... every other row's extents would also admit from a snapshot". Row 11 (variant bag, `s1`-`p1` twice) has no dangling end and admits from a snapshot. Row 10 (`s1`-`p9`, `p9` not held, `Q9` not complete) is the one that cannot: from a snapshot, the `s1` end makes `Q9` required and check 7 returns `Incomplete` at admission, so row 10's navigation-level `incomplete_population` after one `model.navigate` is unreachable through a snapshot. Fix: "Rows 4 and 10". | spec/test-cases/TC-1034-navigation-reads-an-extent-with-charges-dangling-first-and-cardinality-refusals.md:39-41,54 |
| FND-003 | high | An instance end that names an unlisted key of a population that is neither complete nor required is decided by no check, so what the extent stores for it is undefined. Check 6.8 skips an end whose object is not listed, check 7 only fires for required populations (an end's population is required only when the other end is in a required population), and check 8 only refuses absent keys of complete populations. Such an instance admits with an end that has no type component, yet check 8 says the closure rule "keeps every stored end a full reference", FR-364 defines an instance as a triple of full references, and check 11.5 sorts instances by reference key. State the outcome: refuse it, return `Incomplete` for its population, or define the stored form. | spec/functional/FR-106-admit-snapshots-and-invocations.md:385-420; spec/functional/FR-364-carry-relationship-instances-in-an-observation-and-read-them-for-navigation.md:129-140 |
| FND-004 | medium | The order of checks 6.6 to 6.9 over several entries and instances is not fixed, so two readers report different refusals for one document. FR-364 says conditions 1 to 4 run "in this order and stop at the first" (all condition-3 decisions before any condition-4 one), while check 6's per-object pattern and the walk-order paragraph read entry by entry and instance by instance. An entry whose instance 1 repeats instance 0 (6.9) and whose instance 2 has a wrong end type (6.8) gives `conflicting-identity` at instance 1 one way and `wrong-value-kind` at instance 2 the other; likewise a bad instance in entry 0 against a duplicate entry 1 (6.7). No AC-24 or TC-1035 row pins it, since every case "fails only" its check. State whether the conditions run per entry and per instance, or as phases over the document. | spec/functional/FR-106-admit-snapshots-and-invocations.md:216-224,373-402; spec/functional/FR-364-carry-relationship-instances-in-an-observation-and-read-them-for-navigation.md:96-99 |
| FND-005 | medium | FR-364's Dependencies cite a pull request number in spec text and an FR that is not on main: "are FR-363's (open pull request #867)". The PR number goes stale on merge, and if #868 merges first, FR-363 dangles. Drop "(open pull request #867)"; keep the FR-363 reference only once #867 is merged, or describe the checker scope without the id. | spec/functional/FR-364-carry-relationship-instances-in-an-observation-and-read-them-for-navigation.md:250-253 |

## Verdict

**FAIL: 5 blocking findings (FND-001 to FND-003 high; FND-004 an unstated refusal order; FND-005 an id in spec text).**

Verified clean:
- TC-1034 rows 1 to 4 charge numbers are QSpec TC-197 Y07's verbatim (`value_occurrences` 1, 2, 3; five work and three result units; the limit-4 incomplete record; six units for count 4; four units for the dangling `p4`). Row 6 matches Y08's undirected case (above-maximum, `[1,1]`, count 2, one `model.navigate`). The navigation order matches `value-accounting.md`'s `model.navigate` paragraph.
- Doubt (a): row 4 (dangling `p4`) is model-layer as claimed; only row 10 is mislabelled (FND-002). The dangling decision is unreachable from an admitted snapshot, which matches QSL's existing precedent for `deref` (check 8 pre-empts it).
- Doubt (b): no id collision with #863, #864, #865, #866 (merged) or #867. Trial merge (`git merge-tree`): with main after #866, two FR-106 conflicts, both adjacent appends (AC rows after AC-17; the Dependencies FR-056 line beside the new FR-364 bullet); with #867, two spec.md conflicts (adjacent `contains` and index rows), FR-085 merges cleanly. The author's textual claim holds.
- Doubt (c): QSpec FR-340 says a relationship `modifies` entry grants every link of the relationship, and model-complete's Frames row lets `modifies` name a relationship, so lifting FR-103's `unsupported-feature` refusal agrees with QSpec. QSpec says nothing about links of created or deleted objects; FR-106 11.5 decides it and does not contradict QSpec.
- Doubt (d): relationship identities `relationship/<Name>-<verb>-<TargetName>` match FR-154. FR-152's dangling-before-count and multiplicity result types are restated correctly; FR-364's `undecidable` outcome fills a case FR-152 leaves open.
- Every TC-1033 row and TC-1035 rows 1 to 22, 24, 27 to 29 are reachable in FR-106 check order. Cardinality is never checked at admission (TC-1035 row 5, TC-1033 row 7).
- No test parses FR-106, FR-103, TC-458 or spec.md tables (only FR-092 is `include_str!`-ed, and `tools/check-index-completeness.sh` reads the spec.md FR index, which carries FR-364). No tracking, hash or version ceremony in the diff.

Recorded, not reported:
- medium: 11.5 requires a `modifies` grant for links of a created object, while check 11.3 exempts a created object's fields; the stated "as its fields do" analogy holds only for deletion.
- medium: FR-364-AC-5 says a non-unique end returns `Set<Reference<U>>[0,3]`; it is a `Bag` (TC-1034 row 11 is right).
- medium: TC-1034 row 6's variant **housing-undirected** is not defined in TC-1033 (only bag, bag-target).
- medium (PR #867, not this PR): FR-363 names a navigation by the end nearest the receiver, while QSpec TC-197 Y07/Y08 and this PR's TC-1034 name the destination end (`self.parts` from `Sys`, `self.pump` from `Tank`).
- low: FR-106-AC-23 lists `Q` as `s1, p1, p2, t1`; TC-1035's B holds `s1, s2, p1, p2, p3, t1`.
- low: code comments cite FR-106 line numbers (observation.rs:1711, state_clauses.rs:4357), which this insertion shifts further.

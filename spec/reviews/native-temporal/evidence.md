---
id: SR-378
title: "Native temporal verification and evidence review"
type: SpecReview
analysis: evidence
scope: "FR-043, FR-044, NFR-008, TC-122-124, TM-008"
review_set: subset
evaluated_revision: "4c1eee8646b51e00cd141d15eb110a33354b8475"
review_date: "2026-09-11"
---

## Summary

Twenty acceptance criteria and four NFR metrics were examined against their
declared verification method and the evidence TC-122–124 would actually produce.
Nine criteria cannot be discharged by an automated Rust test alone as written,
NFR-008's metric table substitutes a weaker method for the silent-eviction row
that the vendored `ix://agent-ix/quire-specification/NFR-040` singles out, and
NFR-008 publishes no counter definitions, so three of its four zero-target
metrics have no measurement instrument in this repository.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | Metric row 3 substitutes Fault Injection for NFR-040's required model-based-test-generation; the one row NFR-040 warns hardest about is the row whose method was weakened. | NFR-008-M-3; NFR-040 | wrong-requirement |
| FND-002 | high | NFR-008 declares no counter definitions, no named ceilings and no charging contract, yet TC-124 derives expectations from a "published charging contract", "each charged dimension" and a "public accounting boundary" that exist in no document; NFR-006/007 both carry a Counter definitions section. | NFR-008; TC-124; NFR-006; NFR-007 | correct-requirement-no-evidence |
| FND-003 | high | The silent-eviction metric is not falsifiable here: NFR-008's Scope disclaims owning observation storage and retention ("those remain agent F's"), so TC-124 group 3 must force eviction in a mechanism this repository does not own or expose. | NFR-008-M-3; NFR-008-AC-3; TC-124 | correct-requirement-no-evidence |
| FND-004 | medium | NFR-008's Verification section mandates mutation of each exhaustion path in the present tense while its Scope defers mutation adequacy; no TC-124 step performs it and no metric row carries it, so TM-008 reads fully allocated while an owner-declared verification activity has no owner. | NFR-008; TC-124; TM-008 | wrong-requirement |
| FND-005 | medium | "Property Test" is not the catalog method property-based-testing: TC-124 disclaims randomized campaigns, the repository has no proptest or quickcheck dev-dependency, and the four labels are prose rather than catalog ids as used by NFR-006/007. | NFR-008-M-1..4 | wrong-requirement |
| FND-006 | medium | The four metrics measure AC-1 through AC-4 only; NFR-008-AC-5's charge-before-work and usage-reporting claim has no metric row and no threshold. | NFR-008-AC-5 | correct-requirement-no-evidence |
| FND-007 | medium | FR-044-AC-3 is discharged by Rust ownership rather than by test: if the activation API takes its observations by value, "mutating the source observation after activation" is inexpressible and the control is a tautology. | FR-044-AC-3; TC-123 | correct-requirement-no-evidence |
| FND-008 | medium | Universal negatives over the implementation ("no evaluation or graph rewrite folds the two", "no default or nearest-compatible selection is inserted", "no computation wraps, saturates or narrows silently") cannot be established by the enumerated instances; each needs Inspection or Analysis of the rewrite and arithmetic paths. | FR-043-AC-1; FR-043-AC-3; NFR-008-AC-1; TC-124 | correct-requirement-no-evidence |
| FND-009 | medium | Non-reuse is asserted but not observed: a distinct result identity proves the identities differ, not that the retained result was not consulted; no cache, evaluation-count or freshness probe is named. | FR-043-AC-2; FR-043-AC-8; NFR-008-AC-4; TC-122; TC-124 | correct-requirement-no-evidence |
| FND-010 | medium | TC-122 group 3 is not executable as written: one fixed declaration cannot simultaneously carry a declared sample period, an epoch and a timestamp unit, so the six-dimension mutation sweep needs one declaration per profile. | FR-043-AC-3; TC-122 | wrong-requirement |
| FND-011 | medium | FR-044-AC-6's exactly-once source-order claim needs an observable evaluation-order probe; TC-123 group 6 assumes initializers "have observable evaluation order", which the emitted temporal body does not expose. | FR-044-AC-6; TC-123 | correct-requirement-no-evidence |
| FND-012 | medium | FR-044-AC-7's refusal depends on a bridge that TM-008 records as blocked on quire-contract-ir#63/#64; a refusal returned by an absent bridge tests a constant, not a mapping decision. | FR-044-AC-7; TC-123; TM-008 | correct-requirement-no-evidence |
| FND-013 | low | The mutation-adequacy deferral is recorded in three places but is addressed to an unnamed "owning ticket", so it is honest and unactionable at once. | NFR-008; TC-124; TM-008 | correct-requirement-no-evidence |
| FND-014 | low | TC-123 group 6 asserts refusals for ambient `self`, operation `result` and `pre(expr)` initializers that no in-scope acceptance criterion owns. | FR-044-AC-6; TC-123 | missing-requirement |
| FND-015 | low | NFR-008's eviction metric drops the history term NFR-040 carries ("required capture/history"), narrowing the measured subject to valuations and captures. | NFR-008-M-3; NFR-040 | wrong-requirement |
| FND-016 | low | No `data/` advisor or method-catalog output accompanies this scope, unlike SR-092 and SR-105, so the metric labels cannot be resolved to catalog ids from the record. | NFR-008; SR-092; SR-105 | correct-requirement-no-evidence |

## Verification method disposition per criterion

Every criterion in scope declares `Test (TC-1xx)`. Eleven are achievable by an
automated Rust test in this repository once the evaluator exists. Nine are not,
as written, and are listed with the method they actually require.

| Criterion | Declared | Required in practice | Reason |
| --- | --- | --- | --- |
| FR-043-AC-1 | Test | Test + Inspection | The instance distinction is testable; "no evaluation or graph rewrite folds the two" is a property of the rewrite pass, established by reading it. |
| FR-043-AC-2 | Test | Test + Analysis | Three retained profile identities are testable; "no shared cached result" needs a cache probe or a reading of the memoization key. |
| FR-043-AC-3 | Test | Test + Inspection | The six dimension refusals are testable per profile; "no default or nearest-compatible selection is inserted" is a negative over the selection code. |
| FR-043-AC-4 | Test | Test | Operators, zero-width intervals and absent positions are directly observable. |
| FR-043-AC-5 | Test | Test | The insertion-order reversal control genuinely falsifies index substitution. |
| FR-043-AC-6 | Test | Test | Settled, counterexample and unsettled prefixes are three observable outcomes. |
| FR-043-AC-7 | Test | Test | Each removal is an independent observable incomplete result. |
| FR-043-AC-8 | Test | Test + Analysis | Distinct identities are observable; non-reuse of the earlier result is not. |
| FR-044-AC-1 | Test | Test | Two instances and a cross-instance proposition failure are observable. |
| FR-044-AC-2 | Test | Test | Instance count and retained second provenance are observable. |
| FR-044-AC-3 | Test | Analysis | See FND-007; an owned capture environment makes the mutation control unexpressible, and the immutability argument is a type-level one. |
| FR-044-AC-4 | Test | Test | Five independent bad capture inputs, each with a named capture. |
| FR-044-AC-5 | Test | Test | Four dispositions, each carrying no temporal truth. |
| FR-044-AC-6 | Test | Test + Demonstration | Exactly-once source order requires an evaluation-order probe that does not yet exist; the forward-read refusal alone is testable. |
| FR-044-AC-7 | Test | Test, currently blocked | See FND-012; the unsupported-mapping report has no producer in this repository. |
| NFR-008-AC-1 | Test | Test + Analysis | Overflow rejection is testable; "checked arithmetic" everywhere is an arithmetic-lint and code-reading claim. |
| NFR-008-AC-2 | Test | Test | Forced exhaustion on a named ceiling is observable, once ceilings are named. |
| NFR-008-AC-3 | Test | Blocked | See FND-003; no retention or eviction seam is owned here. |
| NFR-008-AC-4 | Test | Test + Analysis | Result identity is observable; reuse refusal is not, and "admitted restoration state" is undefined in NFR-008. |
| NFR-008-AC-5 | Test | Test | Charge-before-work boundaries are testable exactly as NFR-006 tests them, once NFR-008 defines its dimensions. |

## Metric method conformance to NFR-040

NFR-040 names four catalog methods and warns explicitly that a generic
performance benchmark cannot prove a zero-threshold metric; the method must
falsify the corresponding semantic failure mode. Each row was checked against
its source row.

| Row | NFR-040 method | NFR-008 method | Disposition |
| --- | --- | --- | --- |
| Checked bound overflows accepted | property-based-testing | Property Test | Weakened in substance, not in label. TC-124 enumerates deterministically and disclaims randomized campaigns; the repository carries no property harness. This is unit-testing or combinatorial-tway wearing a property name. |
| Exhausted evaluations emitting a Boolean | fault-injection | Fault Injection | Honoured in substance. The label is prose rather than the catalog id, and the path set it must cover is left unenumerated. |
| Silently evicted required state | model-based-test-generation | Fault Injection | Substituted. Model-based generation would derive eviction scenarios from a retention model; two hand-forced evictions in TC-124 group 3 do not. This is the row NFR-040's warning is aimed at. |
| Results reused after a changed limit or binding | integration-testing | Integration Test | Honoured in substance; the label is prose rather than the catalog id, and the reuse observation is missing per FND-009. |

The contrast with this repository's own practice is sharp: NFR-006 and NFR-007
name `negative-abuse-testing`, an actual catalog id, on every metric row, and
SR-092 and SR-105 record the advisor output that justified retaining it. NFR-008
names none.

## Mutation-adequacy deferral

The deferral is honestly recorded in three places: NFR-008's Scope bullet
("recorded as outstanding assurance work rather than claimed here"), TC-124's
Expected Results, and TM-008's coverage rules, which additionally disclaim any
randomized campaign, fuzzing result or mutation-adequacy score. No metric row,
no acceptance criterion and no TC-124 procedure step claims mutation coverage.

It is overclaimed in exactly one place. NFR-008's Verification section states
"Separately mutate each exhaustion path to return a Boolean and require the
suite to fail" as a present-tense obligation of this requirement, with no
deferral marker, mirroring NFR-040's sentence that assigns mutation testing a
measurement role distinct from fault injection. A reader of the Verification
section alone concludes the measurement is in scope; a reader of Scope concludes
it is not. TM-008 then shows all five NFR-008 criteria allocated to TC-124, so
the matrix presents a fully covered requirement whose own Verification section
names an activity nothing performs. The fix is a Verification sentence that
marks the mutation step as deferred and names its ticket, not a new metric row.

## Falsifiability of the zero-target metrics

A zero-target metric is evidence only if some executable run would have produced
a non-zero value had the failure mode been present.

- **Overflow accepted.** Falsifiable. TC-124 group 1 enumerates formulas whose
  composed horizon exceeds the checked domain and requires rejection, so the
  metric reads non-zero if an overflow is admitted. This is the only one of the
  four that is sound as written.
- **Exhausted evaluation emitting a Boolean.** Conditionally falsifiable. It is
  falsified for the three ceilings TC-124 group 2 forces, and for no other
  exhaustion path, because NFR-008 enumerates no path set. For unexercised
  paths the metric reduces to "the suite ran and nothing happened" — which is
  precisely the gap NFR-040 assigns to mutation testing, and which FND-004
  leaves unassigned.
- **Silent eviction.** Not falsifiable. With no retention model, no named
  retention ceiling and no eviction seam owned by this requirement, no run can
  produce a non-zero value; the metric is satisfied by the absence of a
  mechanism rather than by the correctness of one.
- **Stale reuse.** Not falsifiable as written. The only observation offered is
  that two result identities differ, which is consistent with a cache that
  returned a stale result under a fresh identity. A reuse counter or an
  evaluation-count probe would make it falsifiable.

None of the four rows states a population or a counting instrument. Target 0 of
what, over which runs, counted by which counter, is unanswered for all four.

## TC-124 group-by-group

1. **Overflow enumeration.** Produces the evidence metric 1 claims. Its closing
   sentence, "no computation wraps, saturates or narrows silently", is not
   produced by these cases; it is an arithmetic-lint and code-reading claim.
2. **Ceiling exhaustion.** Produces evidence for metric 2 on three ceilings —
   work, active-instance and retention — none of which NFR-008 names, defines a
   unit for, or gives a counter to. The group cannot be written until those
   exist, and it establishes nothing about exhaustion paths it does not reach.
3. **Retention and eviction.** Does not produce the evidence metric 3 claims.
   NFR-008's Scope removes observation storage and replay from this requirement,
   so "force eviction of one required valuation and one required capture" has no
   subject here. The group also needs incremental re-evaluation, which no
   in-scope requirement specifies.
4. **Result identity.** Produces half the evidence metric 4 claims. Distinct
   identities under a changed ceiling are observable; "the earlier result is not
   reused and is not rewritten" is not, absent a probe. "An admitted restoration
   state" is used as an input without definition in NFR-008.
5. **Charging boundaries.** Does not produce its claimed evidence. It iterates
   "each charged dimension" against a "published charging contract" and exercises
   "counter overflow through the public accounting boundary"; NFR-008 defines no
   dimensions, this repository publishes no temporal charging contract, and
   `docs/compiled-protocol-v1.md`'s dimensions are the package and proof
   counters, not temporal ones. Contrast NFR-006, whose Counter definitions
   section makes the identical group writable for the runtime.

Groups 1, 2 and 4 are repairable by naming dimensions and adding a probe. Groups
3 and 5 are blocked on content NFR-008 has not authored.

## Evidence artifacts and data/methods.json

This review authors no `data/` directory and ran no advisor; its judgments are
documentary. NFR-008 does need a `spec/reviews/native-temporal/data/methods.json`
to reach parity with the two evidence reviews that precede it.

- SR-092 records `quoin advise --json` and `quoin catalog methods --json` in
  `spec/reviews/native-runtime/data/advice.json` and `methods.json`; the latter
  is the complete 33-entry catalog with each method's `id`, `class`,
  `evidenceKind` and `applicability`.
- SR-105 records the same pair for the package scope and uses them to justify
  retaining `negative-abuse-testing` against benchmark advice.

For this scope the file should be the verbatim `quoin catalog methods --json`
dump, unedited and unfiltered, so that `property-based-testing` (Test/Property),
`fault-injection` (Test/Integration), `model-based-test-generation`
(Test/Property), `integration-testing` (Test/Integration) and `mutation-testing`
(Test/Static) are each resolvable from the record. It should be accompanied by
an `advice.json` covering all twenty acceptance criteria and the four metric
rows, so that any mismatch, uncatalogued or inconclusive record is visible rather
than asserted. Without it, NFR-008's four prose labels cannot be reconciled with
the catalog, and FND-001's substitution and FND-005's mislabel rest on this
document alone. Authoring both files is left to the owning task.

## Verdict and provenance

FAIL for evidence readiness of NFR-008 as written; PASS with named method
corrections for FR-043 and FR-044. FR-043's eight criteria and FR-044's seven
have a real test case each, and TC-122 and TC-123 describe procedures that would
produce the claimed evidence apart from the five defects recorded above.
NFR-008's Measurement and Evaluation table does not yet honour the vendored
NFR-040 disposition, and two of TC-124's five groups cannot be written against
the current requirement.

No FR, NFR, TC or matrix file was edited by this review. No advisor, Cargo build,
mutation tool or test run was executed, and no execution result is claimed. All
TM-008 rows remain Planned; this review neither qualifies nor advances them. The
temporal evaluator does not exist in `src/` at this revision, so every judgment
about achievability is about the specified public API, not about observed code.

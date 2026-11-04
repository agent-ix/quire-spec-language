---
id: SR-20261104101706
title: "spec-review of batch D-S1 temporal/timed (quire-spec-language PR 2314)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language PR 2314 head 85db1b37ed44db010e45c3a2c21c0dffb080d427; git diff origin/main...HEAD (27 files: FR-195/196/204/283/286/326/327/330/337/350/383/384/515/525/527 and TC-524/630/642/765/770/836/840/1062/1064/1310/1322/1325)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-330
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-204
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-326
    type: references
---

# Spec review of batch D-S1 (quire-spec-language PR 2314)

## Summary

One pass at head 85db1b37, with the EARS, integrity, dependency, criterion-strength and failure-domain
sub-analyses done by hand. Tickets: QSL-1661 (decisive position), QSL-1611 (finite-window interval
owner), QSL-1599 (clause name, replay request), QSL-1579 (Port role identity, QSL half), QSL-577 and
QSL-1498 (digital-clock text).

Checks that passed: `quire validate` exits 0 on all 27 files (only the module-wide advisory
`semantic.inline-data-schema` and duplicate-archetype warnings, present on main). Every new id
(FR-195-AC-7, FR-204-AC-6/7, FR-326-AC-6, FR-337-AC-24, FR-350-AC-16..18, FR-383-AC-62, FR-384-AC-10) is
unused on main and on the six other open heads (#2313, #2311, #2309, #2297, #2296, #2291; only #2309
touches FR-531/FR-533 and #2291 FR-520, none of this PR's files). No ticket id is introduced into AC
text. Each new AC has a TC row and the TC scope line names it. No frontmatter dependency cycle.

(a) QSL-1661: the retargeting is internally consistent. FR-283-AC-1/7/10, FR-286-AC-10, FR-327 (named
position is the activation's own), FR-330 Outputs and AC-12, FR-328-AC-1 (`always` violation at 0), FR-267-AC-19
and TC-765/770/840 agree; vectors check by hand (always p, on origin, p false only at 2: violation 0,
trace_position 0, decisive.position 2; forall example element 0 index 1 at position 2; Bounded
`eventually[0,1]` decisive.position 0). No AC is vacuous in the new examples. Weaknesses are in the
rule, FND-004.

(b) QSL-1611: legitimate profile rule, not a scope cut, with the caveats in FND-005. The rule is scoped
to `quire.temporal.timestamped-event.finite-window/v1` (FR-326 bullet), whose bounds are ticks of the
clock (FR-501), so integer closed tick intervals lose no expressible window (an open end on integer
ticks is a closed end shifted by one; a sub-tick rational has no position to read). It does not touch
the dense-time profile `quire.temporal.timed/v1` (FR-393, FR-447-AC-15 keep `(a, b]`, rationals and
units), the model-claim intervals of FR-233, or the digital-clock claims of FR-204/FR-195 (`[0 ms, 4 ms]`
under `model-time`). The mirror of FR-233 is exact (unit-bearing bound under a step count is
`ill_typed/type-mismatch`). It does replace a behavior the code has today: `checked_interval`
(src/temporal.rs) and `temporal_interval` (qsl-semantics lowering) both accept a closed integral timed
interval under this profile as ticks, and qsl-eval has `ClauseWork` tests for it; those become refusals.

(c) QSL-1599: FR-515 `name_of`/`name()` reads the `temporal_clause_names` row keyed
`{declaration, occurrence}`, as merged FR-531 (table row "clause name") and FR-532 (`ClauseFacts.name`,
`clause-unreadable`/`clause-name`) already do; `name()` None-for-several mirrors the real `clock()`/`clock_of`
in qsl-package/src/checked_views/subject.rs. QSpec carries `temporal_clause_names` per occurrence
(tests `temporal_clause_name_carrier_is_closed_per_occurrence_and_identity_free` pass). Stale
IR-1278 text remains (FND-006). FR-350 follows the published `$defs.ReplayRequest` (14 properties,
`package` required, `$ref` checked-package v2) and QSpec FR-336-AC-7/8; AC-16..18 are value-asserting
and can fail. Gap in FND-007.

(d) QSL-1579 (QSL half): FR-383 role row, FR-383-AC-62, FR-384 roles bullet and AC-10 agree with QSpec
PR 620 (optional third `model_member` Group of `package`, `node`; `model` still references `Sys`).
Real code names `CheckedRole.model_member` and `RoleSlot.model` as the spec says. Dependency on the
reader seeing declaration kinds is not recorded (FND-008).

(e) Digital clock: FR-195 `MeanTime` and its refusals match FR-233's binding, FR-186's duration
conversion and the code's current `not-yet-implemented` refusal it replaces (qsl-semantics
check/probabilistic/claim.rs). `ProductAction::WorkloadResolve{identity, delay}` exists on main
(qsl-analyze/src/exact_probabilistic.rs:496, digital.rs); the mixed given/free construction in FR-204
matches digital.rs (`window.given.is_none()` loop, d<m / d=m / d>m branches, per-choice mass 1,
test `mixed_given_and_free_races_keep_each_choice_a_distribution`). It conflicts with the code and
with FR-202 elsewhere: FND-001, FND-002. AC-6/AC-7 arithmetic re-derived: min 0 at `B,3`, max 1 at
`B,1`, tie 1/2 at `B,2`; guard `x>=2` gives max 1/2, min 0; every-scheduler refusal names `A`
(the first operation declaring a delay, FR-204). All correct.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | FR-204's new sentence "A race with no given-delay identity has the same choices, in which the free identity wins at d with probability 1" and FR-196's "enabled ... in a state where a free-delay identity races" contradict the code: an all-free workload state (`free_moves`, digital.rs:1865) offers UnitDelay plus one `WorkloadResolve{first enabled identity, delay 0}` that splits mass over every enabled identity by workload weight; a given-only race uses `Identity(representative)`. FR-204-AC-5's witness ("sending at x = 1") is a UnitDelay then a resolve in the code, not `WorkloadResolve{send, 1}`. Spec and code cannot both be right; the PR says it is written from PR 2146's code. | FR-204, FR-196, FR-204-AC-5, digital.rs | wrong-requirement |
| FND-002 | high | `WitnessChoice::FreeDelays` (FR-202, per actual given-delay vector, each free identity's delay) and `WorkloadResolve{identity, delay}` (one choice per free identity and delay, made before the given draws resolve) name the same decision at different information. FreeDelays lets the scheduler see the given draw; WorkloadResolve does not, so min/max differ whenever a given distribution has two or more support points. FR-202 is untouched and still defines `free-delays`; FR-204's witness bullet now lists only WorkloadResolve; the code has no FreeDelays. AC-6/AC-7 use a point-mass given delay (`2 ms: 1`), so they cannot tell the two adversaries apart. | FR-202, FR-204, FR-196, FR-204-AC-6/7 | wrong-requirement |
| FND-003 | medium | `MeanTime` is missing from every exact-only rule: FR-195 `exact_only` (Reach or ExpectedReward), the gate's `ExactOnlyForm` cause, FR-196 (Reach/ExpectedReward/LongRunFraction use the MDP as the product), FR-198 (decides Reach and ExpectedReward) and FR-200. A `MeanTime` claim under a workload with no confidence parameters is refused `MissingConfidence` instead of `ExactOnlyForm`; AC-3/AC-4 do not include it. | FR-195, FR-196, FR-198, FR-200 | missing-requirement |
| FND-004 | medium | The decisive rule "first false `holds` atom in position order over the positions the activation read, then clause-node order" is not the deciding atom. FR-328-AC-2's `always (holds(c.value = 1) implies eventually[0,1] holds(c.value = 3))` over 0, 1, 2 is violated at 0 only because of position 1, but the first false atom is the antecedent at position 0, so `decisive.position` is 0. Taking position order first widens the old in-position flaw (negation: `always not holds(p)` has no false atom at all). Also: "never earlier than the activation's own" is false for past operators (`once`, `since`, `past[..]` aggregates read earlier positions, FR-327-AC-4/FR-328-AC-3); on a lasso past the prefix, "position order" and the position value are unstated; on Progressed/Timestamped input the observation at `decisive.position` may not be in the request. No AC covers an implication, a past operator, a lasso or a Progressed input. | FR-330, FR-283, FR-286, FR-327, FR-328-AC-2 | wrong-requirement |
| FND-005 | medium | FR-326 finite-window rule: (1) a timed interval with an unbounded end (`[3 ms, *]`, parses per FR-393-AC-5) falls under both "open end: ill_typed/type-mismatch" and "`[a,*]`: unsupported_construct/expression-form"; AC-6 tests only the step `[3,*]`. (2) Aggregate windows under this profile take durations converted to the clock unit (FR-337 aggregate item 7, `past[0 s, 300000 ms]`); the rule says "each operator interval" and does not say aggregate windows are outside it. (3) The change removes accepted behavior (closed integral timed intervals, e.g. `eventually[1 ms, 5 ms]`) that the code and qsl-eval tests (`a_timed_interval_is_a_tick_reach_only_when_closed_and_integral`, `ClauseWork` Timed arm) rely on; the code ticket must delete that path, or it is dead. | FR-326, FR-337, FR-393 | missing-requirement |
| FND-006 | medium | IR-1278 text remains: FR-531 "What the bytes do not carry" says IR-1278 gates the `name()` of FR-515; FR-518 (lines 122, 155, 156, 174 AC-11, 183) and TC-1313 row 11 still gate on a reader that carries a name on the clause node and compare it with the sidecar, while FR-515 now says the clause node has no `declaration` member and the reader refuses one. | FR-531, FR-518, TC-1313, FR-515 | wrong-requirement |
| FND-007 | medium | FR-350-AC-16 says `package` decodes to a package reference with `checked_package`, `contract_version`, `sources` and `dependencies`. FR-071's package reference is `package_id`, `sources`, `dependencies` and "invents no additional member"; `checked_package` is not a member anywhere, and `contract_version` is a top-level member of the QSpec checked-package document itself. FR-071 and `ReplayRequestWire` (still `package_name`) are not amended, so the adapter has no stated target for the package document. | FR-350, FR-071 | missing-requirement |
| FND-008 | medium | FR-384 dropped "until IR-1310" and FR-383/FR-384 record no replacement dependency, but a Port role's reader check needs the declaration kinds of the domain package (QSpec FR-465 step 5), which QSL-1579's own attachments say is IR-1310 work. The dependency is now unrecorded in both repos. | FR-383, FR-384, QSpec FR-465 | missing-requirement |
| FND-009 | low | FR-515 `name_of(&ClauseOccurrence) -> &str` has no failure path for an occurrence with no name row, a non-claim role or duplicate rows, while `clock_of` returns `Option` and FR-532 refuses `clause-name` only on the evidence route. | FR-515 | missing-requirement |
| FND-010 | low | FR-196 still says "the four delay and observation members exist in a timed product only"; there are now five. FR-383-AC-62 is placed above AC-61; its fixture names `Sys`, `pump_out`, `tank_in` appear in no other spec text (FR-513-AC-8 names none). | FR-196, FR-383 | wrong-requirement |
| FND-011 | low | EARS/atomicity: rationale inside SHALL bullets (FR-195 "so the form is a type error and not a deferral"; FR-326 "therefore holds only ... except as a fault of its own input"); the decisive position rule is stated twice (FR-283 and FR-330) and can drift. | FR-195, FR-326, FR-283, FR-330 | wrong-requirement |

## New findings (disposition pass 1, head 2f145236)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-012 | medium | FR-204-AC-8 requires "the SD-3 witness scheduler of the same values holds a `FreeDelays` entry" with rows per given draw, while FR-204 states the product does not offer the post-draw choice and the parity is unmet. An implementer cannot satisfy both: either the AC is satisfiable (then the parity is not unmet) or the FreeDelays clause is not a requirement of this product. Also ADR-026 SD-3 does say the scheduler chooses each free delay after the given delays are drawn, so offering only the pre-draw `WorkloadResolve` is a narrowing of an ADR, owned by "QSL-1498", whose text names neither the post-draw policy nor this gap; and the generalization "equal whenever the payoff is monotone, which includes every probability of one event after a single race" is asserted, not shown (I verified it only for the single-race cases of AC-8: both policies give 3/4 and 1/4). | FR-204, FR-202, ADR-026 SD-3 | wrong-requirement |
| FND-013 | medium | The FR-330 descent uses "eligible position" without defining it. A false-extension profile makes positions past the trace's end false with no observation (FR-327-AC-1: `always[0,3] holds(c.value <= 2)` is `Completed(false)` only because of position 3), yet only the before-the-trace case is handled; the `until` rule picks `j` as the least eligible position where `φ1` is false, which for FR-327-AC-9's `holds(c.value = 1) until[1,2] holds(c.value = 3)` depends on whether the anchor and the position of the lower bound are eligible; a `Model` trace's positions are reconstructed states, not `DocumentRef`s, and no rule says what observation is evaluated. FR-283's table (`Violated` members include `decisive`) and FR-286 ("`decisive` ... on a temporal `Violated` item") are not amended for the new no-decisive cases (negation, before-trace, `Timestamped`). | FR-330, FR-283, FR-286, FR-327 | missing-requirement |
| FND-014 | high | FR-350's package mapping does not match the published schema it says is authoritative. `$defs.ReplayRequest.properties.package` is `$ref urn:quire:checked-package:v2`, the checked-package document itself (required: `contract_version` const `quire.checked-package/v2`, `name`, `lock`, `semantic_graph`, `source_map`, ...). It has no `checked_package`, `sources` or `dependencies` member: sources are `package.lock.sources` (`DefinitionRef` items), the package id is `package.name`, and the optional internals are `package.lock.refinement_internals`. `$defs.DependencyEntry` is `{identity, sources}` with `additionalProperties: false`, so FR-350-AC-16's dependency `{identity: "lib/money", checked_package, sources}` is rejected by the schema; `contract_version` is the checked-package constant, not "the corpus contract version"; and `dependencies` is a separate top-level member, not `package.dependencies`. The mapping was taken from FR-323 and the FR-336 prose, which disagree with schema.json. TC-1325 row 5 inherits it. | FR-350-AC-16, TC-1325 | wrong-requirement |

## Dispositions (pass 1)

| FND | Outcome | Evidence |
| --- | --- | --- |
| FND-001 | fixed | FR-204 and FR-196 now state the three cases as the product builds them: given-only race edges carry `Identity(r)` (digital.rs:1852), an all-free state offers `UnitDelay` and one `WorkloadResolve{first enabled identity, delay 0}` splitting mass by weight (free_moves, digital.rs:1865 to 1912, resolve_choice delay 0), the mixed state offers per free identity and delay (digital.rs:1608 to 1642). FR-204-AC-5's witness `UnitDelay` at x=0 then `WorkloadResolve{send, 0}` at x=1 matches: send is not enabled at x=0 (guard x >= 1). |
| FND-002 | partly | Honest wording: FR-204 now says SD-3 lets the scheduler choose after the given draw, that `WorkloadResolve` cannot depend on it, and that parity "is unmet". AC-8 re-derived by hand: draws 1 and 3 at 1/2; d=1 gives 1/2*(1/2)+1/2*1 = 3/4, d=2 gives 1/2, d=3 gives 1/4, so max 3/4 and min 1/4, and the post-draw optimum is the same sum because the per-draw argmax (d=1 for the maximum; d=2 then 3 for the minimum, as written) attains it. A vector where the policies differ needs a payoff that is not monotone in the free delay, which one race cannot give: acceptable that the author found none; the gap itself is not closed. AC-8 cannot fail on the missing capability and demands a FreeDelays witness the text says is not offered: FND-012. |
| FND-003 | fixed | `MeanTime` is in FR-195 `exact_only`, the `ExactOnlyForm` gate cause, FR-196 (null monitor, MDP/DTMC product), FR-198 and FR-200; AC-3, AC-4 and TC-630 step 4 cover it, with `ExactOnlyForm` and not `MissingConfidence`. |
| FND-004 | fixed, with FND-013 | Descent defined per operator. Re-derived by hand: AC-13 FR-328-AC-2 gives `decisive.position` 1 (position 0 antecedent false so the implication holds there; position 1 antecedent true, `eventually[0,1] c=3` false, atom false at 1 and 2, least 1); `holds(c<=1) until holds(c=9)` fails at 0 and `φ1` first false at 2, so 2; `always (c=0 implies once[1,1] c=9)` descends to a position before the trace, no decisive; `always not holds(c=2)` no decisive. AC-14: lasso prefix 0, loop 1, 2 gives position 2 (index prefix length 1 plus 1); `Progressed` 0, 1, 2 with `always[0,2]` gives 2; `Timestamped` carries none. "Timestamped carries no decisive" is consistent with FR-283 (a monitor reads `Finite` and `Lasso` only) and with FR-286 only if "on a temporal Violated item" is read as "when the report holds one": FR-286 and FR-283's member table should say so. |
| FND-005 | fixed | One rule per form: a malformed interval by the every-profile rule; a well-formed timed interval (unit, rational, open end, `[3 ms, *]`) `ill_typed`/`type-mismatch`; a step `[a,*]` `expression-form`; aggregate windows exempt; AC-6 and TC-836 step 6 cover all four plus the aggregate window. The code consequence is recorded (the Dependencies line names QSL-1611 for the code deletion, though QSL-1611 is the spec ticket; low). |
| FND-006 | fixed | No `IR-1278` remains under spec/; FR-531, FR-518 (AC-11 now edits the `temporal_clause_names` row's name) and TC-1313 row 11 restated. |
| FND-007 | fixed to a wrong target | The FR-071-unchanged mapping is explicit and the version is not stored, but it maps members the schema does not have: FND-014. |
| FND-008 | fixed | IR-1310 added to the Dependencies of FR-383 and FR-384 and of QSpec FR-465. |
| FND-009 | fixed | `name_of` returns `Option<&str>`, `None` for an occurrence with no row; AC-14 and TC-1310 row 14 assert ordinal 7 gives `None`. |
| FND-010 | fixed | FR-196 says five members; FR-383-AC-62 sits after AC-61 and now imports the FR-513-AC-8 bundle (the names `Sys`, `pump_out`, `tank_in` are still defined only there by reference; low). |
| FND-011 | partly | The FR-195 rationale clause is gone; FR-283 now defers the decisive rule to FR-330 (single statement). The FR-326 "therefore holds only" sentence remains as its own SHALL. |

`quire validate` exits 0 on the 33 changed spec files at 2f145236. The new ids FR-204-AC-8, FR-330-AC-13 and FR-330-AC-14 are free on main and on the other open QSL heads.

## Verdict

Not mergeable yet: FND-014 (AC-16 values invalid against the schema) and FND-012 (AC-8 contradicts the
unmet-parity statement; the SD-3 narrowing needs a ruling and a ticket that owns it). FND-013 may fix
forward. Everything else in the first round is resolved.

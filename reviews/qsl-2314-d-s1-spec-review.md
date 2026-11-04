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

## Verdict

Not mergeable as is: FND-001 and FND-002 (the digital-clock text contradicts the code and FR-202) and
FND-003. FND-004 to FND-008 may fix forward; FND-005's code consequence and FND-004 need author
decisions. The QSL-1661, QSL-1611 and QSL-1599 spec items are otherwise sound.

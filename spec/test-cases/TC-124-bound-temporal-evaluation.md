---
id: TC-124
title: "Bound temporal evaluation work and required retained state"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/NFR-008
    type: verifies
---
# TC-124: Bound temporal evaluation work and required retained state

## Description

Public Rust API controls for
[NFR-008](../non-functional/NFR-008-bound-temporal-evaluation.md). Each case
derives its expected charges from the authored declaration, the supplied trace
shape and the counter definitions published in
[the temporal evaluation contract](../../docs/native-temporal-evaluation.md)
under accounting label `quire.native.temporal-work/1`, never from the run's own
reported usage. Each numbered group corresponds to the matching acceptance
criterion.

Controls execute in `tests/composed_temporal_limits.rs`. Groups 1 and 5 enumerate
bounded formula, interval and ceiling families deterministically over a declared
finite population; group 3 generates obligation and eviction schedules from a
declared finite model. No randomized campaign, fuzzing result or mutation-adequacy
score is claimed.

## Test Procedure

1. Enumerate the overflow population declared in NFR-008's Verification section —
   eight operators x three profiles x nesting depths one through four x four
   interval bound pairs x three anchor offsets, 1,152 cases — and evaluate each. Require rejection before any position is
   visited, with reported `positions` usage zero and the overflow named against
   its operator and interval. Compare each accepted composed horizon against an
   independently computed checked value, and require refusal rather than a wrapped,
   saturated or narrowed result.
2. Independently force a stop in each of the eight charged dimensions against
   otherwise evaluable declarations, including horizon overflow, work exhaustion,
   active-instance exhaustion, capture exhaustion and retention exhaustion. Require each to produce an
   incomplete or refused result naming the reached dimension and the affected
   obligation. Require the returned truth field to be absent in
   every case, including one whose observed prefix would have settled the
   obligation had evaluation continued. Separately, settle one obligation over
   complete exact decision support and then reach a ceiling while assessing a
   second obligation in the same evaluation; require the settled obligation to
   retain its truth, basis and support and the second to carry the stop, per
   FR-043's recorded selection on that open question.
3. Generate obligation populations paired with eviction schedules over a declared
   finite model: for each unsettled obligation, the set of retained valuations and
   captures it still requires, and an eviction point inside that set. Inject each
   eviction through the trace's explicit eviction list, which names the retained
   valuation or capture record by node and clock coordinate, or by instance and
   declared capture index; that seam is what makes the eviction point selectable
   rather than a side effect of a ceiling. Drive incremental re-evaluation and
   require every still-required record retained until the obligation settles. At
   each eviction point require an explicit incomplete result naming the evicted
   subject and retaining the obligation identity, and require the evaluated
   interval unchanged rather than silently narrowed. Separately reach the
   retention ceiling with no injected eviction and require a distinct result, so
   the two events are not one.
4. Evaluate one declaration to a retained result, then re-evaluate with a changed
   work ceiling, a changed active-instance ceiling, a changed clock binding, a
   changed declared clock parameter and an admitted restoration state. Require each
   to produce a distinct result identity, and require the earlier result neither
   reused nor rewritten. Require an unchanged configuration to reproduce the
   identical result identity, and require a ceiling above the published default to
   clamp while a zero ceiling is preserved.
5. For each of the eight charged dimensions, test a zero ceiling, the exact
   required ceiling, a one-step-insufficient ceiling and a ceiling above the
   published default. Require the first unaffordable operation to remain
   unperformed, require reported usage to reflect only successful charges and to
   distinguish the four peak dimensions — instances, retention, depth and horizon —
   from the four cumulative ones, and require a retry
   under sufficient ceilings to produce the full result from unmutated inputs.
   Exercise counter overflow through the public accounting boundary.

## Expected Results

Every resource stop is a typed incomplete or refused result retaining the
obligation, position and limit identity. No exhaustion, overflow or eviction path
produces a Boolean truth, and no changed ceiling, profile, clock binding or
declared clock parameter reuses a prior result.

Mutation-adequacy measurement of this suite — whether it would detect a Boolean
returned from each forced stop — is not performed here and is recorded as
outstanding assurance work on compiler
[#38](https://github.com/agent-ix/quire-spec-language/issues/38). Agent F's
observation storage, replay and lateness mechanisms are outside this case; every
eviction above is injected through the trace's own eviction list against the
evaluator's own retained-state table. Catalog
validation establishes document conformance, not test completion.

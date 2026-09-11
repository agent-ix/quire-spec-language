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
shape and the published charging contract, never from the run's own usage. Each
numbered group corresponds to the matching acceptance criterion.

Controls execute in `tests/composed_temporal_limits.rs`. Property groups enumerate
bounded formula, interval and instance families deterministically; no randomized
campaign or fuzzing result is claimed.

## Test Procedure

1. Enumerate nested future and past formulas whose composed horizon or history
   need approaches and then exceeds the checked integer domain, including
   intervals at the admitted upper extremum and a nesting depth whose product
   overflows. Require rejection before any position is visited, with the overflow
   named against its operator and interval. Require that no computation wraps,
   saturates or narrows silently.
2. Independently exhaust the work ceiling, the active-instance ceiling and the
   retention ceiling against otherwise evaluable declarations. Require each to
   produce an incomplete or refused result naming the reached limit and the
   affected obligation. Require that the returned truth field carries no `true`
   and no `false` in any of the three cases, including where the observed prefix
   would have settled the obligation had evaluation continued.
3. Drive an unsettled obligation whose required valuations and captures remain
   needed, and require them retained across incremental re-evaluation. Then force
   eviction of one required valuation and one required capture and require an
   explicit incomplete result that records the loss, names the evicted subject and
   retains the obligation identity. Require that no eviction silently narrows the
   evaluated interval.
4. Evaluate one declaration to a retained result, then re-evaluate with a changed
   work ceiling, a changed active-instance ceiling and an admitted restoration
   state. Require each to produce a distinct result identity and require the
   earlier result not to be reused or rewritten. Require an unchanged
   configuration to reproduce the identical result identity.
5. For each charged dimension, test a zero ceiling, the exact required ceiling, a
   one-step-insufficient ceiling and a hard-clamped ceiling. Require the first
   unaffordable operation to remain unperformed, require reported usage to reflect
   only successful charges, and require a retry under sufficient ceilings to
   produce the full result from unmutated inputs. Exercise counter overflow through
   the public accounting boundary.

## Expected Results

Every resource stop is a typed incomplete or refused result that retains the
obligation, position and limit identity. No exhaustion, overflow or eviction path
produces a Boolean truth, and no ceiling change reuses a prior result.

Mutation-adequacy measurement of this suite — whether it would detect an illicit
Boolean fallback introduced into each exhaustion path — remains outstanding
assurance work recorded on the owning ticket. Catalog validation establishes
document conformance, not test completion.

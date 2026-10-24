---
id: TC-1161
title: "Every engine limit has a schema-conformant limit name and setting name"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: verifies
---
# TC-1161: Every engine limit has a schema-conformant limit name and setting name

## Description

Verify FR-255's engine table: each engine limits type maps every member to
one limit name and one dotted setting name matching the backend-provider
`LimitName` and `SettingName` patterns, a stopped engine item names that
pair, and `model_check.max_states` and `explore.states` raise different
limits.

Scope: FR-255-AC-12, FR-255-AC-13.

## Test Procedure

1. For each limits type of FR-255's engine table (`ModelCheckLimits`,
   `ModelCheckRequest`'s method bounds, `ExpansionLimits`, `ExactProbLimits`, `StatisticalLimits`,
   `TimedSampling`, `ZoneCheckLimits`, `CertificateCheckLimits`,
   `ScheduleLimits`, the monitor run's window limit and `hybrid.limits`),
   read its mapping from each limit member (not `TimedSampling`'s `q`; for
   `ExactProbLimits`, only the three product-bound `model` members) to its
   names and match each name against the two patterns; for the three
   method bounds, check that their V-6 stops name the member and its value
   (FR-209, FR-225) and that the settings operation raises each by its row's
   setting; collect every setting name of the setting, engine and
   accounting tables.
2. For each engine limit enum, map every variant that names a member of
   the engine's limits types through its limit-name and setting-name
   functions; skip the variants outside those types (evaluation meter,
   `Time`, `ClauseMeter`, cancellation). Give the settings operation
   `model_check.max_states=0` and `model_check.witness_samples=0`.
3. Run FR-126-AC-8's `max_states` 2 model check; raise it with the
   settings operation operand `model_check.max_states=3`.
4. Run FR-101-AC-15's exploration with `max_states` 2; run the model check
   of step 3 with the operand `explore.states=3` only.
5. Run FR-126-AC-6's `max_automaton_states` 50 check and FR-254-AC-3's
   `max_sample_steps` 1 run.

Tag the tests `#[trace("TC-1161", "<criterion>")]`.

## Expected Results

- Step 1: every name matches its pattern; the mapping equals the table
  rows, `ExactProbLimits.model`'s members giving the `exact_prob.*` product
  rows; no setting name repeats across the three tables.
- Step 2: each mapped variant's pair is its member's row; `max_states=0`
  is a usage refusal naming the operand, `witness_samples=0` returns
  `witness_samples` 0.
- Step 3: `incomplete`, `LimitReached{limit: max_states, value: 2,
  setting: model_check.max_states}`; the raised run passes the limit.
- Step 4: `Outcome::Bounded` at `Limit::States`, value 2, setting
  `explore.states`; the model check given only `explore.states=3` runs
  with `max_states` at its default 10000000.
- Step 5: `model_check.max_automaton_states` and
  `statistical.max_sample_steps` are named.

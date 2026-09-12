# Native fixed-sample temporal definition

Profile identity: `quire.temporal.fixed-sample.false-extension/v1`; revision: `1-draft.3`.
Language: `ix:native` / `1-draft`. **Proposed, not adopted or implemented.**

## Required definition

| Identity | Revision | Definition artifact |
| --- | --- | --- |
| `quire.temporal.bounded-facet/v1` | `1-draft.3` | [Bounded temporal rules](temporal-common.md) |

## Selected interpretation

One interval tick is one required sample of the declared positive rational
period from the declared epoch, with exact unit and clock authority. Epoch,
period and unit are runtime clock-binding inputs constrained by this profile;
changing one changes that binding/assessment identity without selecting an
event-position clock. There is no floating approximation or implicit resampling.

Each required sample supplies a total valuation. A missing sample is a
runtime incomplete disposition under a valid binding, never a false sample or
removal of its position. A malformed, missing or incompatible clock definition
or binding instead refuses before evaluation.
Declared sample progress advances through ticks even without a business event.
Only complete decision-scope closure authorizes false extension of atoms after
the last sample; an authoritative origin controls the past boundary. Constants
retain their values. An omitted history interval cannot be padded.

Future/past operators use inclusive sample offsets and the common exact
lower-bound/dual rules. One complete sample with p=true distinguishes
`always[0,1] holds(p)` (false) from `always[0,1] true` (true).
Current TL future mappings additionally preserve epoch/period/unit and total
sample correspondence; no past-time support is inferred.

This selects exactly the corresponding FR-090 meaning in the required common
definition. The other clock/boundary interpretations are different source
profile selections, not defaults, compatible guesses or backend switches.
All common source, activation, capture, type, progress and resource constraints
remain applicable.

Closing a decision scope does not close the surrounding execution. Equal formula
bytes do not establish equal definition selections.

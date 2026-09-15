# Native event-position temporal definition

Profile identity: `quire.temporal.event-position.false-extension/v1`; revision: `1-draft.3`.
Language: `ix:native` / `1-draft`. **Proposed, not adopted or implemented.**

## Required definition

| Identity | Revision | Definition artifact |
| --- | --- | --- |
| `quire.temporal.bounded-facet/v1` | `1-draft.3` | [Bounded temporal rules](temporal-common.md) |

## Selected interpretation

One interval tick is one admitted semantic-event position in the authoritative
sequence. The binding identifies sequence authority, event positions and origin.
Neither elapsed wall time, a duplicate transport receipt nor silence advances
this clock. A missing required position or valuation cannot be compressed out.

For a closed-complete finite decision scope D = {0,...,n-1}, atomic predicates are false at
integer positions outside D; temporal constants keep their values. An
authoritative origin permits the corresponding past boundary. A history cutoff
or incomplete closed file does not. For one complete position with p=true,
`always[0,1] holds(p)` is false while `always[0,1] true` is true.

The same exact lower-bound and dual rules apply to all bounded future/past
operators. A valid witnessed `eventually[0,30] holds(p)` may settle true while
the sequence is still open; that does not close the execution or prove other
obligations. Current TL future mappings require all FR-095 premises; past
requests remain unsupported by current future-only TL profiles.

This selects exactly the corresponding FR-090 meaning in the required common
definition. The other clock/boundary interpretations are different source
profile selections, not defaults, compatible guesses or backend switches.
All common source, activation, capture, type, progress and resource constraints
remain applicable.

Closing a decision scope does not close the surrounding execution. Equal formula
bytes do not establish equal definition selections.

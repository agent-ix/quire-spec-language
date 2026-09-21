# Native timestamped-window temporal definition

Profile identity: `quire.temporal.timestamped-event.finite-window/v1`; revision: `1-draft.3`.
Language: `ix:native` / `1-draft`. **Proposed, not adopted or implemented.**

## Required definition

| Identity | Revision | Definition artifact |
| --- | --- | --- |
| `quire.temporal.bounded-facet/v1` | `1-draft.3` | [Bounded temporal rules](temporal-common.md) |

## Selected interpretation

One interval tick is one tick of the declared timestamp unit under the exact
clock binding. Quantification ranges over admitted instants whose clock distance
from the anchor lies in the inclusive interval. The binding supplies the unit,
clock, window/scope, admitted order and required completeness authority.

An event exactly at the upper endpoint participates before settlement.
An empty existential window is false and an empty universal window is true
only when the authority establishes the needed complete window; missing
observations cannot establish either. A valid early witness may settle an
existential result without closing the execution or unrelated windows.

Until/since use the admitted order from the interval's lower bound to their
witness; release/triggered are their Boolean duals. Equal timestamps without an
admitted order refuse order-sensitive operators. Ingestion order is not an
implicit causal or temporal tie-breaker. Past windows require complete history
or the appropriate authoritative origin under the selected window meaning.

For a complete window containing only the anchor with p=true,
`always[0,1] holds(p)` is true: no synthetic false atom is added after closure.
The current index-based TL profiles do not preserve this meaning; lowering
there is unsupported and cannot silently convert timestamps into event indices.

This selects exactly the corresponding FR-090 meaning in the required common
definition. The other clock/boundary interpretations are different source
profile selections, not defaults, compatible guesses or backend switches.
All common source, activation, capture, type, progress and resource constraints
remain applicable.

Closing a decision scope does not close the surrounding execution. Equal formula
bytes do not establish equal definition selections.

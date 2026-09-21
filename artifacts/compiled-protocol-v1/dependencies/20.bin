# Native bounded temporal common definition

Definition identity: `quire.temporal.bounded-facet/v1`; revision: `1-draft.3`.
Language: `ix:native` / `1-draft`. **Proposed, not adopted or implemented.**
This is a required semantic facet, not a selectable source profile by itself:
a concrete temporal root must select exactly one FR-090 clock/boundary meaning.

Working draft: rules and required definitions resolve in this same checkout
until acceptance freezes one coherent baseline. Runtime selections still require
the exact identities, revisions and digests prescribed by their contracts.

## Required shared interface

| Identity | Revision | Definition artifact | Consumed interface |
| --- | --- | --- | --- |
| `quire.state.graph/v1` | `1-draft.3` | [State/query/graph](state-graph.md) | Checked shared values, finite queries/graphs and explicit Boolean-predicate references |

The concrete temporal roots admit temporal declarations. They reuse the shared
expression checker for `holds(expr)`, activation guards and capture initializers;
they do not import state declaration permissions. Authored reusable predicates
select their own state profile. Arguments retain that callee's checked meaning.
A temporal expression environment supplies its declared current input and
immutable captures, with the trigger available only in its activation scope.
There is no ambient self, operation result, `pre(expr)` or monitor-to-Boolean
conversion. Explicitly supplied pre/post values remain ordinary anchored data.

## Temporal forms and obligations

Admit temporal constants, grouping, temporal Boolean connectives and the eight
bounded operators: eventually, always, until, release, once, historically, since
and triggered. Intervals are inclusive with integer 0 ≤ a ≤ b; unbounded forms
are refused. Strong previous is expressed as `once[1,1]`; no separate weak
previous spelling or boundary is added.

`holds(expr)` is an atomic temporal valuation. A source constant `true` remains
distinct from `holds(true)`; false-extension behavior cannot be optimized away.
Until/since require the left operand only between the interval's lower bound
and the witness; release/triggered use the selected Boolean duals.

Each activation has its own exact semantic trigger/origin identity and captures.
A duplicated receipt cannot create another trigger. Missing required current
valuations, captures or history remain incomplete/refused; they are not false
atoms or empty populations. A valid open prefix settles a Boolean only when
all admitted continuations preserve it, with completed assessment execution,
a matching decisive settlement basis and complete exact decision support.
Otherwise its future remains pending with an unsettled basis. Missing support
makes that truth unavailable; a missing fact outside completed support remains
visible in completeness/adequacy without erasing the settled truth.
Decision-scope progress and closure, surrounding-execution progress and closure,
and input completeness remain independent. Complete decision-scope closure
permits the selected boundary rule without closing the surrounding workflow.

Only a closed-complete trigger scope with no admitted trigger is inactive.
Open or missing/refused trigger evidence leaves activation unknown with its
distinct completeness/execution disposition. Neither becomes temporal truth.
Same-binding progress regression or a conflicting assertion revision produces a
typed contradiction/refusal without rolling back progress or earlier results.

Retain the clock, history, progress, source and population premises needed by
each result. A late contradiction identifies affected earlier results and their
premises without rewriting their bytes. Resource stops cannot produce Boolean
fallbacks. Replay and incremental consumers use the same selected semantics.

TL is an internal mapping target under FR-095, not the semantic authority for
these native profiles. Source admission of past or timestamped operators does
not claim that the current TL bridge supports them. Model/observation bindings
are exact linked/request inputs governed by the shared package contract.

## Selected temporal rules

| Rule | Meaning |
| --- | --- |
| [FR-048](../../../spec/functional/FR-048-bind-native-temporal-syntax.md) | Native temporal syntax and typed environments |
| [FR-090](../../../spec/functional/FR-090-select-temporal-profile-and-clock.md) | Exact profile, clock and boundary selection |
| [FR-091](../../../spec/functional/FR-091-evaluate-bounded-future.md) | Bounded future truth and decisive settlement |
| [FR-092](../../../spec/functional/FR-092-evaluate-bounded-past.md) | Bounded past truth and history |
| [FR-093](../../../spec/functional/FR-093-bind-temporal-activation-and-captures.md) | Activation, trigger scope and immutable captures |
| [FR-094](../../../spec/functional/FR-094-interpret-progress-history-and-closure.md) | Independent progress/closure, decision support and supersession |
| [FR-095](../../../spec/functional/FR-095-preserve-native-tl-correspondence.md) | Native-to-TL correspondence and refusal |
| [NFR-040](../../../spec/non-functional/NFR-040-bound-temporal-state.md) | Temporal work, retained history and resource limits |

## Shared result interface and compatibility

The shared result interface is
[FR-061](../../../spec/functional/FR-061-report-orthogonal-results.md).
The selected interface is its assessment execution, independent scope/execution
progress and closure, truth, activation, settlement basis, exact decision
support and dependence lineage, completeness, and supersession rules, including
the corresponding result-combination rows. Protocol-specific claims, protocol
adequacy and canonical protocol-result serialization remain owned by the
protocol contract; this facet does not import protocol declaration permissions
or require an E02 projection to evaluate temporal source.

A temporal claim does not manufacture complete-global-conformance closure.
When the shared result carries that record for a non-global claim, FR-061
requires `not-required` with no global premise set.

FR-095 requires the bridge correspondence record to retain the native/shared
result dimensions outside the TL formula when the selected TL wire does not
encode them. Equal TL truth or formula bytes do not supply missing metadata.

The interval algebra and three clock meanings are retained. Exact runtime
definition selections remain part of the linked subject; equal formula bytes
cannot substitute for them. Historical draft text remains in Git.

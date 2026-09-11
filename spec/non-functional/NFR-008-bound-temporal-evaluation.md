---
id: NFR-008
title: "Bound temporal evaluation work and required retained state"
type: NFR
quality_attribute: reliability
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-043
    type: constrains
  - target: ix://agent-ix/quire-spec-language/FR-044
    type: constrains
  - target: ix://agent-ix/quire-spec-language/FR-045
    type: constrains
  - target: ix://agent-ix/quire-specification/NFR-040
    type: depends_on
---
# NFR-008: Bound temporal evaluation work and required retained state

## Statement

If a temporal horizon computation overflows or a selected ceiling is reached,
then the temporal evaluator shall stop with a typed incomplete or refused result
that retains the affected obligation, position and limit identity, and shall not
emit a Boolean truth for that obligation.

## Scope

- Applies to native temporal admission, whole-trace evaluation and incremental
  re-evaluation of an emitted `quire.compiled-protocol/1` temporal body, and to
  the mapping classification in
  [FR-045](../functional/FR-045-classify-temporal-mapping-support.md).
- The evaluator owns its own retained-state table: the valuations and captures an
  unsettled obligation still requires between incremental evaluations. That table
  is what the retention ceiling bounds and what eviction removes. Agent F's
  observation storage, replay and lateness mechanisms are separate and are not
  constrained here.
- Numeric ceilings are explicit caller-lowered inputs. The defaults tabulated
  below are this evaluator's own hard ceilings, clamping a caller who asks for
  more; they are an implementation bound on one Rust entry point and are not a
  universal semantic maximum for native temporal obligations, which this
  requirement does not invent.
- The counters, their units and the traversal rules that charge them are published
  in [the temporal evaluation contract](../../docs/native-temporal-evaluation.md)
  under the accounting label `quire.native.temporal-work/1`.

## Rationale

Bounded interval syntax does not make horizon arithmetic, operator nesting,
concurrent trigger instances or retained histories safe. A resource stop must stay
distinguishable from a property violation and from successful closure; collapsing
either into `false` would report a passing or failing obligation that was never
evaluated.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|--------|--------|-----------|--------|
| Horizon or history computations accepted after exceeding the checked i64 domain, over the enumerated nested-interval population | 0 | 0 | property-based-testing |
| Evaluations emitting `true` or `false` after a forced overflow, work, instance, capture or retention exhaustion, over the enumerated forced-stop population | 0 | 0 | fault-injection |
| Unsettled obligations whose required retained valuation or capture was removed without an incomplete result naming it, over the generated obligation/eviction-schedule population | 0 | 0 | model-based-test-generation |
| Results reused after a changed ceiling, profile, clock binding or declared clock parameter, over the enumerated re-evaluation population | 0 | 0 | integration-testing |
| Charged steps performed after the first unaffordable charge, over the zero, exact, one-short and clamped ceiling population in every dimension | 0 | 0 | negative-abuse-testing |

## Counter definitions

`Limits`, `Usage` and `Dimension` carry the eight counters below. A refused charge
never increases usage, and the first unaffordable operation is not performed.

| Counter | Kind | Default ceiling | Unit |
| --- | --- | --- | --- |
| `positions` | cumulative | 1,000,000 | admitted trace positions inspected |
| `valuations` | cumulative | 1,000,000 | atomic valuation lookups |
| `instances` | peak | 10,000 | obligation instances active at one time |
| `captures` | cumulative | 100,000 | retained capture records |
| `retention` | peak | 1,000,000 | retained valuation records required at one time |
| `visits` | cumulative | 1,000,000 | temporal graph node visits |
| `depth` | peak | 64 | temporal formula nesting levels |
| `horizon` | peak | `i64::MAX` | greatest admitted interval bound |

`horizon`'s default is the checked arithmetic domain itself, so at the default
the overflow rejection always fires first and the ceiling is unreachable. It
becomes reachable only where a caller lowers it, which is its purpose.

- `positions` counts each admitted trace position the evaluator inspects,
  cumulative, including the same position inspected under two operators.
- `valuations` counts each atomic valuation lookup, cumulative, including a lookup
  that finds the leaf absent.
- `instances` is a peak counter recording the greatest number of obligation
  instances active at one time.
- `captures` counts each capture record retained, cumulative across instances.
- `retention` is a peak counter recording the greatest number of retained
  valuation records an unsettled obligation required at one time.
- `visits` counts each temporal graph node visit, cumulative; a shared node
  visited under two parents counts twice.
- `depth` is a peak counter recording the greatest temporal formula nesting depth
  traversed. Grouping nodes are traversed iteratively.
- `horizon` is a peak counter recording the greatest interval bound admitted after
  checked composition.

Interval composition, horizon computation and history need use checked i64
arithmetic. Overflow refuses before evaluation; it never wraps, saturates or
narrows. Ceilings above the published defaults are clamped, and zero is preserved.
The effective clamped ceilings participate in result identity.

## Verification

Enumerate the declared overflow population: for each of the eight bounded
operators, each of the three profiles, and nesting depths one through four, the
interval bound pairs `(0, i64::MAX)`, `(1, i64::MAX)`, `(i64::MAX - 1, i64::MAX)`
and `(0, i64::MAX / 2)` evaluated at anchor offsets `0`, `1` and `i64::MAX / 2` —
`8 x 3 x 4 x 4 x 3 = 1,152` cases, of which the composed-horizon overflow set is
determined by checked arithmetic rather than by observation. Enumerate
concurrent trigger instances at the active-instance ceiling minus one, at it,
and at it plus one. Independently force
horizon overflow, work exhaustion, active-instance exhaustion, capture exhaustion
and retained-state eviction, and require a typed incomplete or refused result
retaining the affected obligation, position and limit identity.

Eviction is injected through the trace's explicit eviction list, which names the
retained valuation or capture record the caller reports as no longer available.
That seam exists so eviction can be placed at a chosen point rather than inferred
from a ceiling: a reached retention ceiling and an evicted required record are
two distinct events with two distinct results, and metric 3's target is
falsifiable because the injected eviction can be placed on a record the
obligation still requires. For each charged dimension, test the zero, exact,
one-short and clamped ceiling and require the first unaffordable operation to
remain unperformed.

Mutation testing of the exhaustion paths — whether the suite detects a Boolean
returned from each forced stop — is **not performed by this revision**. It is
recorded as outstanding assurance work on compiler
[#38](https://github.com/agent-ix/quire-spec-language/issues/38), consistent with
the owner's direction to track incomplete assurance separately from
implementation delivery. No metric row above claims it.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| NFR-008-AC-1 | Future horizon, past-history need and nested interval composition use checked i64 arithmetic and reject overflow before any position is visited, with no wrap, saturation or silent narrowing. | Test (TC-124); Analysis |
| NFR-008-AC-2 | A reached ceiling in any of the eight charged dimensions produces an incomplete or refused result naming that dimension and the affected obligation, and never `true` or `false`, including where the observed prefix would otherwise have settled. | Test (TC-124) |
| NFR-008-AC-3 | Valuations and captures an unsettled obligation still requires remain in the evaluator's retained-state table until it settles; forced eviction produces an explicit incomplete result naming the evicted subject and never narrows the evaluated interval. | Test (TC-124) |
| NFR-008-AC-4 | Effective clamped ceilings and any admitted restoration state participate in result identity; an unchanged configuration reproduces one identity and a changed ceiling refuses reuse of the earlier result. | Test (TC-124); Analysis |
| NFR-008-AC-5 | For every charged dimension, the first unaffordable operation remains unperformed, reported usage reflects only successful charges, and a retry under sufficient ceilings produces the full result from unmutated inputs. | Test (TC-124) |

## Dependencies

- **Upstream**: [FR-043](../functional/FR-043-evaluate-bounded-native-temporal.md),
  [FR-044](../functional/FR-044-activate-temporal-obligations.md) and
  [FR-045](../functional/FR-045-classify-temporal-mapping-support.md) define the
  outcomes this requirement bounds.
- **Peer**: [NFR-006](./NFR-006-bound-native-runtime.md) bounds the historical
  native runtime; its counters and ceilings are separate from these.
- Shared source requirement: `ix://agent-ix/quire-specification/NFR-040`.

---
id: SR-17185
title: "QSL-1072 PR #1523 failure-domain review of FR-487"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language; spec/functional/FR-487-lower-hyper-atoms-and-match-conjuncts-to-checked-nodes.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-487
    type: reviews
---

## Summary

Ticket: QSL-1072. PR: quire-spec-language#1523 (head 56043f5ba). This lens looks for unstated failure modes, identity confusion and purity gaps in FR-487's evaluation and seam sections. Each was checked against qsl-eval/src/hyper/seam.rs, qsl-eval/src/simulation/model/letters.rs (`ClauseLetters`), qsl-eval/src/value/expression/mod.rs (`evaluate_state_predicate_at_absent_key`, `Machine::with_absent_parameter`) and qsl-semantics/src/check/observation.rs.

Examined units: FR-487 "Evaluating an atom", "Evaluating a match conjunct" and "The clause seams" (examined), plus FR-487-AC-3, FR-487-AC-4, FR-487-AC-5 and FR-487-AC-6 (examined). FR-107 and FR-104 "Observations of reads" were read for context only.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `CheckedHyperLetters::atom` "maps to `AtomReading` as `ClauseLetters` maps it". But `ClauseLetters` maps a refused evaluation, an unobservable state and an evaluator fault to `Err(AtomStop::{Refused, Unobservable, Fault})`, and `HyperLetters::atom` returns `Result<AtomReading, Incomplete>`. The seam has no channel for those three outcomes. A coder must drop them, turn them into `Incomplete` (which is wrong), or panic. Extend `HyperLetters`' error type as `StepMatch`'s is extended (for example `Err(AtomStop)`), and state how FR-176/177/178 read it. Or state the mapping explicitly. No AC covers a refused atom. | FR-487:238-243, qsl-eval/src/hyper/seam.rs |
| FND-002 | medium | An absent instance key must be bound "the way `ClauseLetters` binds an absent instance key, in that component only". `ClauseLetters` does this through `evaluate_state_predicate_at_absent_key`, which sets `Machine::with_absent_parameter(slot, population)`, so reads through the slot are `absent-key` undefined and a `lookup` guard is false. `evaluate_hyper_holds(clause, atom, slots: Vec<Value>, components, meter)` cannot say that a slot is absent or name its population. As specified, `CheckedHyperLetters` cannot be built on `evaluate_hyper_holds`. Add a per-slot absent marker (slot to population binding) to the signature, and an AC for a key absent in one component only. | FR-487:85-91, FR-487:238-242 |
| FND-003 | medium | It is not said how a reference keeps its component. FR-104/FR-107 observations are static S3 annotations (`Observations::of(root, Current, slots)` keyed by read location), and today's `Machine` takes one `current` environment. FR-487 has every slot read at observation `current`, yet requires that reads through a slot-derived reference use component `t`'s environment, including through `let` and `deref`. It does not say whether S3 records a component per read (extending `Observations`) or the `Machine` tags reference values at run time. The two designs differ for a reference stored in a `let` or chosen by an `if`. The definedness check also treats every component as one `current` observation. Choose one and state it, with an AC where a reference read in copy `a` is dereferenced through a `let`. | FR-487:136-139, FR-487:191-195 |

## Verdict

Changes requested: the atom seam's error path, the absent-key binding and the component-tracking mechanism are all unspecified at the interface. The conjunct side is complete: faults charge nothing, an out-of-range `conjunct` faults, decode failures fault, and short-circuit charging is pinned by AC-4.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | The new static rule gives each model read "the trace variable of the innermost indexed read that encloses it", and claims every model read lies inside some `@ t` because FR-172 gives every member read a trace variable. Main's FR-172 check (`check_atom`) does not guarantee this. It flags only a chain read whose root is an object parameter, so `deref(value(v.next @ a)).l` checks while its `.l` read is enclosed by no `@` and gets no component. Conversely, FR-487-AC-9's `(let r = v.next in ...) @ a` is refused by `check_atom` today: `read_root` of the `let` is none, giving `MismatchedTrace` at the `@`, and `v.next`, whose parent is the `let`, gives `UnindexedRead`. FR-172 states no rule that `@ t` scopes over a compound operand. Amend FR-172 to admit `(e) @ t` with enclosing scope and to refuse, or give the reference's component to, a member read outside every `@`. Add an AC for `deref(value(v.next @ a)).l`. | FR-487:167-174, FR-487-AC-9, qsl-semantics/src/check/hyper_clause.rs check_atom |

## Dispositions

Round 1, head bd9361749 (fix commit be4e0bad6).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `HyperLetters::atom` now returns `Result<AtomReading, AtomStop>`. The mapping follows `ClauseLetters`, products record non-Incomplete stops in `Halt`, and qsl-analyze maps them to `ModelCheckRefusal::{AtomRefused, StateUnobservable, Fault}` (all present in qsl-analyze/src/model_check/check.rs). Covered by FR-487-AC-12. |
| FND-002 | fixed | `HyperSlot::{Bound, Absent { key, population }}` in the signature. Absent slots follow `evaluate_state_predicate_at_absent_key` / `Machine::with_absent_parameter`, with any number allowed. Covered by FR-487-AC-9. |
| FND-003 | fixed | Static choice stated: `component_reads(atom)` is recorded at S3 by read location, definedness is per component, and the evaluator and SMT reader read through the record. Covered by FR-487-AC-1, AC-9 and AC-10. The enclosure rule has a residual gap, FND-004. |
| FND-004 | fixed | Round 2, head b23f84fdd: FR-172 "Indexed atoms and step labels" now admits a compound operand of `e @ t`, with `@ t` scoping over every member read not enclosed by an inner `@`, chain reads rooted at an object parameter and field selections on a `deref` included. It refuses `ill_typed` a member read that no `@` encloses (the `.l` of `deref(value(v.next @ a)).l`) and gives such a read no component. FR-487 "Components of reads" cites the rule, and FR-172-AC-8 (TC-1264 step 14) covers the accepted `let` form and both refusals. |

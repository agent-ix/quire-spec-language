---
id: SR-12886
title: "integrity review of STD-163 FR-194 undefined window positions"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@15eca179893ce3d5c95742758a2476f907133f4d; Ticket STD-163; PR #979; spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md, spec/test-cases/TC-629-window-aggregates-in-monitors.md; context: spec/decisions/ADR-024-statistical-and-probabilistic-properties.md, spec/decisions/ADR-014-temporal-trace-and-boundedness-architecture.md, agent-ix/quire-specification@967ee5a spec/functional/temporal/FR-414-evaluate-window-aggregates.md"
review_set: subset
---

## Summary

Ticket: STD-163. A consistency check of the amended FR-194 against the
decisions it depends on (ADR-024 WA-4/WA-5, ADR-014 TR-5) and against QSpec
FR-414, which FR-194's References name as the owner of aggregate semantics and
`InsufficientData`. The ticket names "Owners: FR-414, FR-194".

## Verdict

CONDITIONAL. FR-194 is internally consistent, but four statements elsewhere
still say "per value visited", and the owning QSpec FR-414 has no
undefined-position rule.

## Examined Scope

```yaml
scope:
  - {id: FR-194, path: spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md, role: examined, excerpt: "Evaluation SHALL charge one unit of ADR-014 TR-5 work per window position visited, whether or not the filter admits the position."}
  - {id: ADR-024, path: spec/decisions/ADR-024-statistical-and-probabilistic-properties.md, role: examined, excerpt: "WA-5 ... Evaluation charges TR-5 work per value visited."}
  - {id: ADR-014, path: spec/decisions/ADR-014-temporal-trace-and-boundedness-architecture.md, role: examined, excerpt: "TR-5 ... Amended by ADR-024 WA-5: an aggregate atom also charges one unit per window value it visits"}
  - {id: FR-414, path: "quire-specification:spec/functional/temporal/FR-414-evaluate-window-aggregates.md", role: examined, excerpt: "A Boolean value of each aggregate atom at each position, or the kernel's no value with cause insufficient-data{count, min_count} when the window lacks data, or incomplete, limit-reached naming the limit. ... Evaluation charges one unit of clause work per value visited."}
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | QSpec FR-414 owns the aggregate semantics, and STD-163 names it as an owner, but it is unchanged. Its Outputs list only Boolean, `insufficient-data` or `limit-reached`, which leaves no room for a passed-through undefined cause. It has no rule against skipping an undefined position, and line 87 still says "one unit of clause work per value visited". A second implementation that conforms to QSpec could skip undefined positions. Amend FR-414 (and TC-366) to match, in this ticket or a tracked STD follow-up linked from STD-163. | quire-specification:spec/functional/temporal/FR-414-evaluate-window-aggregates.md:34-35,87 |
| FND-002 | medium | QSL's own decisions still say "per value": ADR-024 WA-5 ("charges TR-5 work per value visited"), ADR-024's amendments list ("aggregate atoms charge per value visited (WA-5)") and ADR-014 TR-5 ("one unit per window value it visits"). FR-194 depends_on both ADRs, so a reader of the ADRs charges the filtered count. Update all three to "per window position visited". | spec/decisions/ADR-024-statistical-and-probabilistic-properties.md:154,423; spec/decisions/ADR-014-temporal-trace-and-boundedness-architecture.md:174 |
| FND-003 | low | ADR-024 WA-4 and FR-414-AC-3 say an `InsufficientData` atom is "never UndefinedEvaluation". The new FR-194 cause is an ordinary kernel undefined, which the temporal engine reports as `UndefinedEvaluation{cause}` (#966 `outcome.rs`). That reading is consistent, but neither ADR-024 nor FR-194 says it outright. One clause naming the undefined-position outcome category would remove the doubt. | spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md:104-110 |

## Evidence and Limits

`git grep "value visited"` over QSL `spec/` and QSpec `origin/main` found
these hits and no others. QSpec was read only.

## Dispositions

Round 1 reviewed at `7d264fc5b` (fix commits `0c8174539`, `7d264fc5b`), with QSpec PR #277 at `66f9538`.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | QSpec PR #277 amends FR-414: Outputs, a new "Undefined positions" section, "per window position visited", AC-7 and AC-8, and TC-366 WA-08 to WA-10. FR-194 Dependencies cite QSpec FR-414. #277 must merge with #979 (still open). |
| FND-002 | fixed | ADR-024 WA-5 ("per window position visited, filtered out or not"), the ADR-024 amendments list, and ADR-014 TR-5 ("one unit per window position it visits"). |
| FND-003 | still-open | The added clause, "carried as `InsufficientData` is, in the undefined-evaluation record of the O-16 undefined result", contradicts ADR-024 WA-4 and FR-327, which say `InsufficientData` is never `UndefinedEvaluation`. My round-0 suggestion was also wrong: it took the clause category to be O-16 undefined. See new FND-004 and FND-005. |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | high | FR-194 makes the clause "O-16 undefined" when a window position is undefined (lines 96, 110, 117-118). QSL FR-327 (Undefined evaluation) says otherwise: an atom that evaluates `Undefined` (only a window's `insufficient-data` excluded) returns `Undefined(UndefinedEvaluation{where, cause})`, category **violation**; only `insufficient-data` settles O-16 `undefined`. So a `division-by-zero` inside an aggregate would be non-failing, while the same division in a plain `holds` atom fails (FR-327-AC-5). That also goes against "a window result never improves by the loss of a position" and against ADR-024 SV-11. Failure scenario: `sum of e.a / e.b over past[0, 9] <= 5` with `e.b = 0` never fails the pipeline. Owner ruling needed; the consistent reading is FR-327's: `UndefinedEvaluation{where, cause}`, category violation, distinct from `insufficient-data`. AC-5's "the clause is undefined there" should then state the category. | spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md:96,110,116-121; spec/functional/FR-327-evaluate-a-temporal-clause-over-a-finite-trace.md:103-117 |
| FND-005 | high | The same contradiction in QSpec PR #277. The new "Undefined positions" section says "The monitor clause at that position settles in the O-16 category `undefined` with that cause". That gives a kernel-undefined window position the treatment FR-414 reserves for `insufficient-data`, which it explains as "not an undefined evaluation of the claim". Align it with whatever FND-004 rules. | quire-specification:spec/functional/temporal/FR-414-evaluate-window-aggregates.md:88-89 |

Observation outside this PR's scope, for PR #966: `TemporalOutcome::category` maps every `Undefined(_)`, `InsufficientData` included, to `Category::Violation`. FR-327 and ADR-024 WA-4 require O-16 `undefined` for `insufficient-data`.

---
id: SR-1384
title: "Code review of quire-spec-language PR #666 (QSL-658): FR-261-AC-3 test doc comment"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@48a6a04a26b55c01feee1416e8fb4d19c178bae8; qsl-semantics/tests/it/state_clauses.rs:3529-3535 (doc comment on a_value_nested_100_000_deep_is_counted_as_observation_values)"
review_set: base
---
# Code review of quire-spec-language PR #666

## Summary

Ticket: QSL-658. The only code change is the doc comment on `a_value_nested_100_000_deep_is_counted_as_observation_values`. It now says the deep value is read, counted and judged `wrong-value-kind`, and no longer says it is admitted. The test body and the harness are unchanged.

Examined:
- the doc comment, lines 3529-3535 (examined)
- the test body, lines 3536-3561 (context_only)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The doc comment says "One below the document's value count refuses naming `observation.values`, its bound and the count reached". The body sets the bound to 50,000 and asserts bound 50000 and actual 50001. The body's own inline comment says so: "A bound of 50,000 is crossed by the nested value alone, at count 50,001". The doc comment contradicts the code it documents. Reword it to the 50,000 bound. | qsl-semantics/tests/it/state_clauses.rs:3533,3546 |
| FND-002 | low | "raising the setting through `ObservationLimits`' builder judges the field again" describes a run after the refusal, as if a judged run also came before it. The body makes one refused run, then one `raised` run, and that is its only wrong-value-kind assertion. Say "with the setting raised through the builder, the field is judged (`wrong-value-kind`)". | qsl-semantics/tests/it/state_clauses.rs:3534-3535,3561 |

## Verdict

The wording otherwise matches the body:
- the 512 KiB thread is the harness's
- the depth is 100,000
- the value is counted as `observation.values`
- with the limits raised the result is `wrong-value-kind`

The comment carries the same "one below" mismatch as FR-261-AC-3 (SR-1383 FND-001). Fix both together. No test logic changed, so no build is needed to review this. **Not mergeable until FND-001 and FND-002 are fixed.**

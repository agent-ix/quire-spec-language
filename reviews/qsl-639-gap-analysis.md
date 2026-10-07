---
id: SR-1377
title: "Gap analysis of quire-spec-language PR #662: quire-exact Arc payload adaptation (QSL-639)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@5ec9b043df24940bda88919ebf4f557f147b9fec; PR #662 diff origin/main...HEAD (qsl-semantics/src/check/check.rs, qsl-semantics/src/check/check/typing.rs, qsl-eval/tests/it/model_reference_queries.rs, qsl-semantics/tests/it/state_clauses.rs); FR-261-AC-3 and its tagged test; FR-104-AC-1 (touched test)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-261
    type: reviews
---
# Gap analysis of quire-spec-language PR #662

## Summary

Ticket: QSL-639. Planless gap analysis over the PR diff. Plan completion: not assessed.

The diff changes types only and adds no behavior and no criteria, so it opens
no reverse gap and adds no stub. The touched test
`the_configversion_state_clauses_check` is tagged `TC-459`/`FR-104-AC-1` and
keeps its assertion. The other touched test, `population_refused_as_option_payload`,
was untagged before this PR and the PR does not change that.

QSL-639 is the ticket this PR closes on the QSL side. Its stated purpose is
FR-261-AC-3: admit a recursive value 100,000 levels deep, with linear memory.
`quire matrix` at 5ec9b043d marks FR-261-AC-3 `tagged`, but only by
`a_value_nested_100_000_deep_is_counted_as_observation_values`. With limits
raised, that test asserts a `wrong-value-kind` refusal, not admission.
`spec/functional/FR-261-...md` Status still reads "admission of the
100,000-deep value waits on QSL-639". Neither this PR nor quire-exact #10
supplies the QSL-side admission test.

Repository-wide matrix at this head: 976 tagged, 1032 untagged, 43
method-without-symbol. None of these change with this diff. They are recorded
here as context, not as findings against this PR.

Examined:
- FR-261-AC-3 (examined): "On a thread with a 512 KiB stack, the observation document reader admits a snapshot whose population's field holds a recursive value 100,000 levels deep, with `observation.input_bytes` and `observation.values` raised to fit and the field's declared type admitting that value. With `observation.values` one below the document's value count, it refuses naming `observation.values`, its bound and the count reached, and admits once the setting is raised through `ObservationLimits`' builder."
- FR-104-AC-1 (examined, through the touched test `the_configversion_state_clauses_check`)
- Semantic review (intent↔test↔code): skipped. This is a reviewer-only dispatch with no user opt-in.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-261-AC-3's admission clause is still untested after QSL-639 lands. The only tagged test asserts `wrong-value-kind` refusal at raised limits, where the AC asks for admission into a field whose declared type admits the 100,000-deep value with linear memory. The FR-261 Status note still defers admission to QSL-639. Either this PR adds the admission test (a recursive-Option-typed field, 100,000 deep, admitted at raised limits) and updates the Status note, or QSL-639 must stay open when #662 merges. Merging #662 does not deliver QSL-639's stated test. | qsl-semantics/tests/it/state_clauses.rs:3536-3561; spec/functional/FR-261-read-other-untrusted-json-at-any-depth.md:63; spec/functional/FR-261-read-other-untrusted-json-at-any-depth.md:80-83 |

## Verdict

**CONDITIONAL**. The adaptation diff has no traceability, reverse-gap or stub
defect. One medium ticket-level gap remains: FR-261-AC-3 admission is not tested
on the QSL side, so QSL-639 cannot be closed on this PR alone.

## Coverage

- Plan completion: not assessed
- Matrix: `quire matrix --scope . --format json` (quire 0.36.1, engine 0.50.1). FR-261-AC-1/2/3 are all tagged. AC-3's tag covers the count/refusal half only (FND-001).
- Reverse gap: none. The diff adds no behavior.
- Stubs and coverage inflation: none in the diff.
- Semantic review: skipped (no opt-in).

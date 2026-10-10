---
id: SR-4946
title: "EARS review of QSL-673 QSL formatter/settings contract"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@6a75170acbe78788941973fc2ecbdcc4b7f074c2; spec/functional/FR-003-format-native-source.md, spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md, spec/functional/FR-277-bound-every-lifecycle-operation-by-caller-limits.md"
review_set: subset
---

## Summary

The changed QSL requirement text is mostly concrete, but the new formatter
builder/seam sentence places two SHALL obligations in one EARS statement and
does not give the second obligation its own subject.

## Verdict

CONDITIONAL — split the two obligations into atomic statements with an explicit
formatter/settings subject.

## Examined Scope

```yaml
scope:
  - {id: FR-003, path: spec/functional/FR-003-format-native-source.md, role: examined, excerpt: "The typed FormatLimits.output_bytes field is the format.output_bytes setting in FR-255, and format_with_limits(source, limits) consumes that value."}
  - {id: FR-255, path: spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md, role: examined, excerpt: "FormatLimits::with_output_bytes SHALL set the formatter's field, and format_with_limits SHALL consume the resulting FormatLimits value."}
  - {id: FR-277, path: spec/functional/FR-277-bound-every-lifecycle-operation-by-caller-limits.md, role: examined, excerpt: "The format operation shall read output_bytes from FormatLimits; the settings operation shall route format.output_bytes=<n> to that field."}
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-255 Behavior 4 now contains two SHALL obligations in one statement: `FormatLimits::with_output_bytes SHALL set...` and `format_with_limits SHALL consume...`. Quire reports `ears:non-singular`, `ears:unclassifiable` and `ears:missing-subject` for the changed statement. Split it into separate requirements naming the formatter and limits consumer. | spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md:139-143 |

## Evidence and Limits

Targeted validation compared the frozen head with its parent: the parent FR-255
was grammar-clean, while the frozen head reports exactly these three EARS
warnings on the newly added sentence. No unrelated repository warning was
treated as a QSL-673 finding.

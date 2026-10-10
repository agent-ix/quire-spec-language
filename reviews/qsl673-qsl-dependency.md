---
id: SR-4948
title: "dependency review of QSL-673 QSL formatter/settings contract"
type: SpecReview
analysis: dependency
scope: "agent-ix/quire-spec-language@6a75170acbe78788941973fc2ecbdcc4b7f074c2; FR-003, FR-255, FR-277, FR-275"
review_set: subset
---

## Summary

The formatter correctly points at FR-255 for the shared setting seam, and
FR-277 points back to FR-003 for formatter behavior. Those edges form a
prerequisite cycle: the shared mapping enablement and the concrete formatter
feature are mutually required without a separable ordering owner.

## Verdict

CONDITIONAL — split or reallocate the shared seam so the dependency graph is
acyclic before task ordering.

## Examined Scope

```yaml
scope:
  - {id: FR-003, path: spec/functional/FR-003-format-native-source.md, role: examined, excerpt: "FR-255 owns the setting row and the shared limits/settings seam."}
  - {id: FR-255, path: spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md, role: examined, excerpt: "FR-003 owns the formatter's output bytes, refusal code and no-partial-output rule."}
  - {id: FR-277, path: spec/functional/FR-277-bound-every-lifecycle-operation-by-caller-limits.md, role: context_only, excerpt: "The format operation shall read output_bytes from FormatLimits; the settings operation shall route format.output_bytes=<n> to that field."}
  - {id: FR-275, path: spec/functional/FR-275-take-a-typed-request-limits-and-cancel-on-every-lifecycle-operation.md, role: context_only, excerpt: "Each operation also takes its limits value (FR-277) and Cancel."}
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The new FR-003 `depends_on` edge to FR-255 and FR-255's normative dependency on FR-003 create FR-003 → FR-255 → FR-003. A formatter cannot consume the shared FormatLimits/SettingLimits seam until FR-255 exists, while FR-255's format row and refusal semantics require FR-003. Separate the shared limits/settings enablement into an owner that precedes both, or make one edge a non-prerequisite reference; otherwise the required topological ordering is cyclic. | spec/functional/FR-003-format-native-source.md:8-14,80-88; spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md:196-208 |

## Evidence and Limits

The cycle is visible from the changed FR-003 frontmatter and FR-255 dependency
section; no build, lock or implementation check was used. The review treats
“owns” and the explicit FR-003 dependency prose as normative prerequisite
claims, as required by dependency analysis.

## Dispositions

Round 1 disposition reviewed at `bac630980b94dca810e1718a2e1b9adbb1420a2d`.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | bac630980b94dca810e1718a2e1b9adbb1420a2d |

FR-003's relationship to FR-255 is now `references`, and FR-255 retains FR-003 as a reference rather than a prerequisite dependency; the prior FR-003 -> FR-255 -> FR-003 prerequisite cycle is removed.

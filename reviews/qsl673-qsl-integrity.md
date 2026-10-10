---
id: SR-4947
title: "integrity review of QSL-673 QSL formatter/settings contract"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@6a75170acbe78788941973fc2ecbdcc4b7f074c2; FR-003, FR-255, FR-277, TC-931, spec/tests.md"
review_set: subset
---

## Summary

The paired FRs and TC-931 cover the intended formatter budget seam, and the
new acceptance criteria are falsifiable. Status prose is internally
contradictory: FR-255 and TC-720 claim mapping/settings work implemented while
the same change says the new formatter row and seam are pending QSL-605.

## Verdict

CONDITIONAL — make the implementation status consistently partial/planned and
keep the pending TC-931 obligations explicit.

## Examined Scope

```yaml
scope:
  - {id: FR-003-AC-10, path: spec/functional/FR-003-format-native-source.md, role: examined, excerpt: "FormatLimits maps its only field, output_bytes, to FR-255's format.output_bytes through SettingLimits; with_output_bytes changes only that field, and the settings operation's format.output_bytes=<n> operand supplies the resulting value to format_with_limits."}
  - {id: FR-255-AC-3, path: spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md, role: examined, excerpt: "Each stage's limits type, including FormatLimits, maps every one of its fields to exactly one setting name..."}
  - {id: FR-255-status, path: spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md, role: examined, excerpt: "The setting table, the one-mapping-per-limits-type rule, the builders, the settings operation, a request's stage_limits and the defaults are implemented... The format.output_bytes row and the FormatLimits to settings-operation seam remain pending QSL-605."}
  - {id: TC-720-status, path: spec/test-cases/TC-720-a-reached-limit-names-its-kind-bound-count-and-setting.md, role: examined, excerpt: "Implemented. Step 1 is stage-driven for every setting row but the pending intake.input_bytes; steps 2 and 3 are implemented."}
  - {id: TC-931, path: spec/test-cases/TC-931-format-setting-integration.md, role: examined, excerpt: "Planned. This case is the QSL integration contract for QSL-673 and QSL-605."}
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-255's status first declares the setting table, every mapping and builder implemented, then says the new `format.output_bytes` row and FormatLimits/settings seam remain pending. TC-720 also says its mapping step is implemented even though its “each limits type” scope now includes the pending FormatLimits type, while TC-931 is Planned. Mark the existing claims partial or narrow TC-720 to implemented types so status and acceptance state agree. | spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md:173-194; spec/test-cases/TC-720-a-reached-limit-names-its-kind-bound-count-and-setting.md:28-48; spec/test-cases/TC-931-format-setting-integration.md:44-46 |

## Object Review Note

The new `FormatLimits`/`SettingLimits` seam is described as an implementation
interface within FR-003/FR-255 and does not introduce a separate domain object
artifact. This is acceptable for the current QSL functional-spec dialect; the
seam's owner and mapping remain covered by the criteria above.

## Evidence and Limits

Targeted validation passed all changed criteria structurally (6/19 criteria
property-extractable on the five-file selection; the remaining criteria are
existing broad rows). The computed matrix marks FR-003-AC-10 and the new
formatter portions of FR-255/FR-277 as untagged future obligations, consistent
with TC-931's Planned status.

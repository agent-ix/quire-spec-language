---
id: SR-4945
title: "base review of QSL-673 QSL formatter/settings contract"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@6a75170acbe78788941973fc2ecbdcc4b7f074c2; spec/functional/FR-003-format-native-source.md, spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md, spec/functional/FR-277-bound-every-lifecycle-operation-by-caller-limits.md, spec/test-cases/TC-931-format-setting-integration.md, spec/tests.md"
review_set: subset
---

## Summary

The QSL change supplies the concrete row, typed `FormatLimits` seam and TC-931
boundary contract. Targeted validation found the new files structurally usable;
the text contains a contradictory generic limit-outcome sentence and labels an
output budget with the input-byte kind/cause without defining the diagnostic
semantics clearly enough.

## Verdict

CONDITIONAL — resolve the outcome exception and byte-kind semantics before
implementation consumes this contract.

## Examined Scope

```yaml
scope:
  - {id: FR-003-AC-10, path: spec/functional/FR-003-format-native-source.md, role: examined, excerpt: "FormatLimits maps its only field, output_bytes, to FR-255's format.output_bytes through SettingLimits; with_output_bytes changes only that field, and the settings operation's format.output_bytes=<n> operand supplies the resulting value to format_with_limits."}
  - {id: FR-255-AC-1, path: spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md, role: examined, excerpt: "The format.output_bytes case uses FR-003's resource_exhausted outcome over emitted UTF-8 bytes."}
  - {id: FR-255, path: spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md, role: examined, excerpt: "The format.output_bytes row bounds the UTF-8 bytes the formatter would return; QSL uses the existing input-bytes limit kind and its input-bytes-exceeded catalog cause."}
  - {id: FR-277-AC-1, path: spec/functional/FR-277-bound-every-lifecycle-operation-by-caller-limits.md, role: examined, excerpt: "Except that format.output_bytes returns FR-003's typed resource_exhausted diagnostic; for execute the outcome is Incomplete."}
  - {id: TC-931, path: spec/test-cases/TC-931-format-setting-integration.md, role: examined, excerpt: "Every bound below the required output length returns resource_exhausted/input-bytes-exceeded, names format.output_bytes and the configured bound, and returns no partial string."}
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-277 Behavior first says every reached bound SHALL return `LimitExceeded`, while the preceding description and Outputs explicitly exempt `format` with `resource_exhausted`. The unqualified behavior bullet can direct an implementation to return the wrong outcome; repeat the formatter exception in that bullet or scope it to non-format operations. | spec/functional/FR-277-bound-every-lifecycle-operation-by-caller-limits.md:23-29,43-50,54-62 |
| FND-002 | medium | FR-255 calls `format.output_bytes` an `input bytes` limit and requires `input-bytes-exceeded` even though the row and FR-003 define the counter as emitted UTF-8 output. The text gives no normative rendering rule that prevents an “input bytes” diagnostic from misleading callers; define an output-byte kind/cause or explicitly specify the shared catalog kind and output wording. | spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md:90,100-107,118-129; spec/test-cases/TC-931-format-setting-integration.md:34-42 |

## Evidence and Limits

Targeted validation passed 4/5 changed QSL documents grammar-clean; the only
warning is the newly added compound EARS statement in FR-255, handled in the
separate EARS artifact. Full-scope validation reports pre-existing structural
failures. No code, build, or lock job was run.

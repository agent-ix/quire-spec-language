---
id: SR-20261109172912000129
title: "QSL-1813 owned clause carrier base specification review"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; QSL-1813; PR #2641; qsl-command; qsl-foundation/src/diagnostic.rs; qsl-replay/src/outcome.rs; root imports and boundary tests; FR-100; FR-287-AC-10/11; FR-001-AC-16"
review_set: base
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-287
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-001
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: references
---

## Summary

Base review of the FR-100 carrier clarification accompanying bounded Rust enum storage. No specification defects found.

## Verdict

**PASS for the changed carrier wording; it agrees with the implemented owned input and changes no request grammar or execution semantics.**

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Review Evidence

Frozen head: `64beae30adb80dedb2c26aa86bf50ace8a641dd9`. Only FR-100's documented clause carrier changes in the specification delta. It now names Box<ClauseRunRequest>, matching PreparedRun::Clause, and explains owned storage indirection. The function carrier remains PreparedFunctionRun; its private inner box does not alter the public into_input return. The already reviewed preparation requirements, input fields, provider responsibility, strict decoder, filesystem boundary and output contract are unchanged.

This clarification adds no requirement ID, acceptance criterion, relationship, semantic branch, wire field, alternate grammar or raw stream mode. Existing native clause/ConfigVersion acceptance tests exercise the moved clause path; external tests exercise function ownership and the carrier-independent contract. IDs and links remain unchanged, and explicit-path quire validation passed. No edited requirement statement requires a separate EARS lens, and no object/dependency or artifact-structure amendment requires an additional sub-analysis. No applicable AssuranceProfile was found.

## Coverage

Plan completion: not assessed

Base specification review only, as selected by the changed scope. Runtime implementation is assessed separately by SR127 and the scoped semantic delta by SR128. The unrelated baseline bin guard is not waived.

## Examined Units

```yaml
{
  "scope": [
    {
      "id": "FR-287-AC-10",
      "path": "spec/functional/FR-287-reach-qsl-through-the-driver-cli.md",
      "role": "context_only",
      "excerpt": "An external Rust consumer with no dependency on `quire-spec-language` calls `qsl_command::command::compile_outcome` over AC-2's compile request and `prepare_run` over its bounded run request."
    },
    {
      "id": "FR-100",
      "path": "spec/functional/FR-100-run-a-named-function-through-the-spine.md",
      "role": "examined",
      "excerpt": "`PreparedRun::Function(PreparedFunctionRun)` and\n`PreparedRun::Clause(Box<qsl_replay::spine::ClauseRunRequest>)`; the owned\nindirection bounds the enum's stack size without changing the clause request."
    },
    {
      "id": "FR-284",
      "path": "spec/functional/FR-284-keep-the-qualified-core-separable-by-crate.md",
      "role": "context_only",
      "excerpt": "The crates above the core are QSL's `qsl-analyze` (layer A), `qsl-inspect`\n(layer P), `qsl-jit` (layer X) and `qsl-command` (the FR-027 and FR-100\nrequest-file adapter), and the driver's `quire-plugin-host`,\n`quire-aot` and `quire-cli` (ADR-029 CB-3 item 3). Each may\ndepend on the core. No core crate depends on any of them."
    }
  ],
  "bindings": []
}
```

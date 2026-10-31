---
id: SR-1956001
title: "code-review review of QSL-852 reference and reader acceptance"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; QSL-852; PR1956 frozen 02aad2ad7; changed qsl-eval, qsl-package, qsl-semantics files and TC-1074"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-spec-language/FR-390", type: references }
---

## Summary

Reviewed all six changed Rust files, their callers, and TC-1074. The behavior is sound for the delivered slice; one new lookup helper duplicates an existing helper and must be removed under the mandatory duplication rule.

## Verdict

PASS after disposition pass 1 at 06b9de9fa7487cc2e5179c7166839b811010e2af. The original review failed on FND-001; that finding is now fixed. QSL-852 remains In Progress.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The new compensation_binding duplicates the existing protocol_terminal::argument lookup; reuse one shared argument helper rather than maintaining a second implementation. | qsl-package/src/emit/tests/protocol_controls.rs:1325-1332; qsl-package/src/emit/tests/protocol_terminal.rs:50-58 |

## Coverage

Searched qsl-package/src/emit/tests for argument/binding helpers and named body-argument searches. protocol_terminal.rs:50-58 already performs exactly this body.arguments lookup by name and returns its value; protocol_graph.rs:101-112 also owns the lookup plus a layout assertion. The new compensation-specific name does not add compensation behavior. Share the existing lookup and remove compensation_binding, preserving any layout assertions. This is high because the code-review Duplication and Vendoring rule mandates that severity.

The foreign-model reference is produced from a second actually admitted model and reaches the intended CatalogCode before general type membership can mask it. The shape guard still requires an expected Reference and actual Reference. The extracted predicate preserves the existing universe rule for nested admitted values. All failures assert the precise typed category, code/cause, binder and unchanged full meter; the subsequent retry matches a fresh entire meter. Tests use the real public evaluator and real reader, with unchanged E admitted before mutations. Source capture tests retain exact read spans. The shared E/L fixture was moved rather than duplicated; no production API, dependency, feature, wire form, gate, async, unsafe or secret-handling surface changed.

## Assurance Context

No applicable AssuranceProfile is declared in spec/assurance: the seven artifacts are AD-001 and MP-001 through MP-006, not enforced review policies. No profile-selected context or exceptions apply. The parent's provided unchanged-main control and current reader dependency were examined; whole-repository matrix/run evidence is unavailable and not inferred clean.

## Examined units

```yaml
{
  "scope": [
    {
      "id": "FR-390-AC-6",
      "path": "spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md",
      "role": "examined",
      "excerpt": "A value of the wrong shape, a record of another type, a value outside its type's domain, a reference to another model's population and a dangling reference each refuse with the code and cause TC-1071 lists; no observation with a reference, and an incomplete population, return `Incomplete`; an observation FR-106 refuses is refused with its record."
    },
    {
      "id": "FR-390-AC-12",
      "path": "spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md",
      "role": "examined",
      "excerpt": "A package whose capture initializer reads itself, a later capture or another template's capture is refused at S3 (FR-381) and no package is built; a package mutated in its bytes so an activation capture's initializer reads its own binder, or another template's capture binder, is refused by the contract reader with `missing_declaration`/`missing-name` (`read` `forward` and `foreign`), and no evaluation call is possible."
    },
    {
      "id": "FR-390-AC-11",
      "path": "spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md",
      "role": "context_only",
      "excerpt": "Running one call twice gives equal results and charge logs; a call over the package the contract reader returns for FR-383's emitted nodes gives the same result and charge log as over the checked package that was emitted; a refused call leaves the meter as it was."
    },
    {
      "id": "TC-1074",
      "path": "spec/test-cases/TC-1074-keep-an-unordered-capture-graph-out-of-evaluation.md",
      "role": "examined",
      "excerpt": "the contract reader refuses the bytes `missing_declaration`/`missing-name` at the owning compensation node `Refund`, retaining name `activated`, stage `activation-capture`, owner `Refund` and `read` `forward`; it returns no package"
    },
    {
      "id": "TC-1074",
      "path": "spec/test-cases/TC-1074-keep-an-unordered-capture-graph-out-of-evaluation.md",
      "role": "examined",
      "excerpt": "the contract reader refuses `missing_declaration`/`missing-name` at the owning compensation node `Refund`, retaining name `partialTarget`, stage `activation-capture`, owner `Refund` and `read` `foreign`"
    },
    {
      "id": "TC-1074",
      "path": "spec/test-cases/TC-1074-keep-an-unordered-capture-graph-out-of-evaluation.md",
      "role": "examined",
      "excerpt": "The compensation, clause and ancestors retain their structural node keys. QSpec FR-322 defines a declared node's path as its declaration path and states that body content is not part of that path; its package contract defines structural equality without a package identifier, hash, digest or seal."
    }
  ]
}
```

## Validation evidence

Independent focused checks passed: formatting; Clippy for the three changed libraries and qsl-eval integration target; 17 protocol tests; one actual-reader test containing both mutations; both source refusal tests; configured make cargo-deny-bans. Logs: /tmp/qsl1956-review-{fmt,clippy-libs,clippy-it,protocol,reader,source1,source2,deny-bans}.log. The first Clippy command rejected repeated --lib flags and was corrected; it is not a code failure. A generic cargo deny check is outside this repository's bans-only configuration; the prescribed bans target passed.

The coder's single required aggregate run stopped at the exact unchanged-main C14 MalformedWire refusal at /semantic_graph/nodes/2/node_id; the CI baseline diff is empty. Separate string-edge/checked-input normalized baseline diffs are empty (17/3 unchanged findings). No aggregate was repeated and later aggregate lanes were not reached; whole CI success is not claimed.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Removed compensation_binding. Both reader-mutation calls use the existing protocol_terminal::argument via named_argument; the existing test helper is pub(super), with its lookup implementation unchanged. No production/public API changes or new regression found. |

Disposition pass 1 inspects only the fix at 06b9de9fa7487cc2e5179c7166839b811010e2af. The original finding text is preserved. Coder confined checks pass: formatting, scoped Clippy, 17 protocol tests, one reader test covering both mutations, and two source tests. No aggregate or matrix was repeated. The partial slice is review-approved, subject to the lead's existing gate evidence and latest artifact custody; full AC-6 and AC-11 remain open.

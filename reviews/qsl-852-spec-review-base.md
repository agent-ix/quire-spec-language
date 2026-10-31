---
id: SR-1955003
title: "spec-review/base review of QSL-852 partial protocol acceptance"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; QSL-852; PR1955 frozen 7f6406d; qsl-eval/, qsl-semantics/, FR-390, TC-1070, TC-1073"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-spec-language/FR-390", type: references }
---

## Summary

Base review of the three changed specification artifacts and their existing accounting and canonical-order dependencies. The owner-approved retention and label-index errata are concrete and preserve the normative owners.

## Verdict

PASS for this method and the stated partial scope. QSL-852 remains In Progress; this review does not authorize whole-ticket completion.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

The retention erratum delegates to existing S6a points and explicitly states zero result_units for forwarded/literal values with no retain point; it introduces no protocol retention charge. TC-1070 no/yes rows now match emitted UTF-8 order. TC-1073 retains navigate-only C and adds D, preserving both coverage paths and separately requiring expected retain points. No relationship edge, public API, dependency, pin, wire shape, or new charge point changes. Verification methods remain Test; inherited Test(TC) cell formatting is outside this erratum. Native validation of each explicit touched artifact passed. No applicable AssuranceProfile exists under spec; direct review is appropriate. No plan or assurance acceptance run was supplied.

## Examined units

Each unit below records its complete acceptance text or the specific changed statement. Examined partial AC-6/11 subproperties do not claim full criterion completion; see the gap report's owned remainder.

```yaml
{
  "scope": [
    {
      "id": "FR-390-AC-7",
      "path": "spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md",
      "role": "context_only",
      "excerpt": "A source supplied `Unavailable` and read returns `Unavailable` naming the binder and the read, with the charges already made; the same source in a call whose short-circuit never reads it completes."
    },
    {
      "id": "FR-390-AC-8",
      "path": "spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md",
      "role": "context_only",
      "excerpt": "A capture initializer whose dispatched call has a false precondition gives the capture slot an `Undefined` result with reason `precondition-false`, category undefined, exit 10; the same slot with the precondition true completes."
    },
    {
      "id": "FR-390-AC-9",
      "path": "spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md",
      "role": "examined",
      "excerpt": "With usage `U` from a completed call, a meter with every charged counter at exactly its measured usage completes; with each charged counter in turn one below its usage and the others at theirs, the call returns `Incomplete` naming that counter with no value; a zero `work_units` returns `Incomplete`; a fresh meter completes the same call with the same value. Forwarded scalars and literal constants with no existing retain point have zero `result_units` by S6a; tests assert those zero usages independently of the charged-counter boundary loop."
    },
    {
      "id": "FR-390-AC-12",
      "path": "spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md",
      "role": "context_only",
      "excerpt": "A package whose capture initializer reads itself, a later capture or another template's capture is refused at S3 (FR-381) and no package is built; a package mutated in its bytes so an activation capture's initializer reads its own binder, or another template's capture binder, is refused by the contract reader with `missing_declaration`/`missing-name` (`read` `forward` and `foreign`), and no evaluation call is possible."
    },
    {
      "id": "FR-390-AC-13",
      "path": "spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md",
      "role": "context_only",
      "excerpt": "With the S6a probe variant of FR-107-AC-6 added, the build fails with E0004 at exactly the checked-in S6a seam list: the arm serving state clauses and protocol slots is the one `ProtocolClause` arm."
    },
    {
      "id": "FR-390-retention",
      "path": "spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md",
      "role": "examined",
      "excerpt": "A completed value is retained and charged only at the existing S6a retain points of its expression. A forwarded scalar or literal constant that reaches none has zero `result_units` by S6a; returning it from a protocol slot does not add a retain charge."
    },
    {
      "id": "TC-1070-rows-29-33",
      "path": "spec/test-cases/TC-1070-evaluate-the-expressions-of-protocol-nodes-from-a-checked-package.md",
      "role": "examined",
      "excerpt": "Case indexes name the emitted UTF-8 label order, so `no` is index 0 and `yes` is index 1 even though the source declares `yes` first."
    },
    {
      "id": "TC-1073-C-D",
      "path": "spec/test-cases/TC-1073-account-protocol-slot-evaluation-on-one-shared-meter.md",
      "role": "examined",
      "excerpt": "**D** is the variant of C whose initializer is `deref(view).total`, with the same observation and supply. C is navigate-only; D charges both `model.deref` and `model.navigate`."
    },
    {
      "id": "TC-1073-retention",
      "path": "spec/test-cases/TC-1073-account-protocol-slot-evaluation-on-one-shared-meter.md",
      "role": "examined",
      "excerpt": "Assert these point sets and the zero usages independently of the measured-boundary loop: a missing charge must not remove its own test coverage by making a counter appear uncharged."
    },
    {
      "id": "FR-107-model-reads",
      "path": "spec/functional/FR-107-evaluate-state-clauses-at-s6a.md",
      "role": "context_only",
      "excerpt": "The evaluator SHALL charge `model.deref` for each `deref(...)` and `model.navigate` for each field read, as `value-accounting.md` states."
    },
    {
      "id": "QSpec-FR-466-related",
      "path": "quire-specification/spec/objects/interfaces/FR-466-checked-package-protocol-control-bodies.md",
      "role": "context_only",
      "excerpt": "sorted by the relationship's node id, then the JCS bytes of `from`, then those of `to`"
    }
  ]
}
```

## Validation evidence

Independent reviewer checks: cargo fmt --all -- --check passed; scoped locked clippy with -D warnings passed; protocol_slots filter passed 16 tests (including two fixture-support tests); explicit model-read unit filter passed one test. Each touched spec validated at its explicit path through make check-spec-validation. Logs: /tmp/codex1-qsl-852-review-{fmt,clippy,protocol,model,fr390,tc1070,tc1073}.log. Full aggregate gate remains the code lead's responsibility; no aggregate or whole-repository qualification claim is made. Installed modules emit duplicate-archetype and inline-data-schema advisory diagnostics, with exit zero.


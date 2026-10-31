---
id: SR-1956002
title: "gap-analysis review of QSL-852 reference and reader acceptance"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; QSL-852; PR1956 frozen 02aad2ad7; changed qsl-eval, qsl-package, qsl-semantics files and TC-1074"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-spec-language/FR-390", type: references }
---

## Summary

Bounded computed-matrix and reverse-gap review found no new gap in the delivered rows. This is not whole-repository qualification or full AC-6/AC-11 completion.

## Verdict

CONDITIONAL for the stated partial scope. QSL-852 remains In Progress.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Plan completion: not assessed

An exact bounded scratch copy at /tmp/qsl1956-matrix contains FR-390 and the four relevant test source files; quire matrix --scope /tmp/qsl1956-matrix --format json exited 0 (CLI 0.37.1, engine 0.50.3). AC-6 and AC-12 each compute tagged with the new tests. The existing AC-11 tag alone does not prove read-back evaluation parity. AC-13/14/15/17/18/19 are untagged in this bounded copy and AC-16 has method-without-symbol; these bridge/source suites were not inventoried and this is not a new-PR repository-wide coverage diagnosis. No whole matrix was attempted because the lead reports a reproducible hang. No run-evidence store was read. Manual reverse reconciliation maps the only production predicate/order change to FR-390 admission check 5 and TC-1071 row 18; there are no stubs or invented authority types. Optional whole-scope semantic analysis was not requested.

Remaining ownership is explicit: full AC-6 population/provenance/FR-106 record handling is QSL-1372/IR-1198; AC-11 evaluation-view parity and related bridge criteria remain with their upstream dependencies. TC-1074 row 6's read-back evaluation is not proved here. AC-12 source rejection and actual reader self/foreign rejection are exercised; no admitted package is yielded on either mutation.

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

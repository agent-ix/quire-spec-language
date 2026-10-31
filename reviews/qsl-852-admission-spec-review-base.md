---
id: SR-1956003
title: "spec-review-base review of QSL-852 reference and reader acceptance"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; QSL-852; PR1956 frozen 02aad2ad7; changed qsl-eval, qsl-package, qsl-semantics files and TC-1074"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-spec-language/FR-390", type: references }
---

## Summary

Reviewed the TC-1074 identity and refusal-location errata against normative QSpec FR-322 and FR-464 and existing FR-390 AC-12. No contradiction or invented wire policy found.

## Verdict

PASS for the stated partial scope. QSL-852 remains In Progress.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

FR-322 structural identity excludes body content from a declaration path and defines no package identifier/hash/digest/seal; the rewritten instructions retain structural keys and update sorted dependencies. FR-464 Binder scope and AC-8 locate a forbidden read at the node whose slot holds its expression and retain name/stage/owner/read. Rows 4/5 now name Refund and preserve the typed details. Source rows 1-3 keep S3 read spans. The prose's hash/digest terms explicitly deny such identifiers; they do not create tracking or pin machinery. The commit/PR quotes agree with the local normative source. No new requirement statement, relationship edge, domain object, external tool interface, security boundary or product scope was added; EARS/dependency/object/evidence/risk/failure-domain/scope lenses are inapplicable beyond the base checks for this narrow TC erratum.

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

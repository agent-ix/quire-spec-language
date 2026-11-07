---
id: SR-20261107070951249604
title: IR-1403 spec-review-ears-conformance compound quantity type references
type: SpecReview
analysis: ears-conformance
scope: agent-ix/quire-spec-language; IR-1403; spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md,
  spec/test-cases/TC-483-assembler-admits-dimensions-and-units.md
review_set: subset
---

## Summary

Ticket: IR-1403. Reviewed PR 2496 at b732481c2 against the merged QSpec source grammar and existing quantity owner. The later conflict-free main merge 23b90d698 retains identical production and dependency code; its upstream additions are tests/support only.

The added requirement statements name S1, S2, assembler or checker as actors and concrete acceptance, syntax retention, resolution, normalization, refusal or ceiling responses. No vague-response or event/state mismatch was found. Targeted engine validation reports 2/2 documents grammar-clean and zero grammar findings.

## Verdict

PASS — no findings in the changed scope. This verdict does not claim the pending aggregate gate or downstream IR-842 U25 runtime qualification.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

No applicable AssuranceProfile is present. Review selection: base, ears-conformance and failure-domain; no structural domain-object or relationship-edge edits require integrity, object or dependency analyses. Other optional lenses are not selected.

```yaml
scope:
- id: FR-091-AC-74
  path: spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md
  role: examined
  excerpt: Over admitted canonical roots `kg`, `m` and `s`, parse and check parameter,
    result and conversion-target types `Quantity<kg*m^2*s^-2>`. Parsed forms retain
    the three qualified unit factors, exponents `1`, `2`, `-2` and their source spans.
    The checked type has the exact compound identity whose canonical terms name those
    roots with those exponents; `Quantity<s^-2*kg*m*m>` resolves to that same compound
    identity. `Quantity<kg>` remains declared, while `Quantity<kg^1>` and `Quantity<kg*m^0>`
    are the same one-term compound identity, distinct from declared `kg`; `Quantity<m*m^-1>`
    is the empty compo
- id: FR-091-AC-75
  path: spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md
  role: examined
  excerpt: '`Quantity<>`, `Quantity<kg*>`, `Quantity<kg**m>`, `Quantity<m^>`, `Quantity<m^n>`
    and `Quantity<m^1.5>` refuse typed `invalid_syntax` at their malformed unit expression.
    `Quantity<kg*ghost^2>` and `Quantity<ghost^0>` refuse `missing_declaration`/`missing-name`
    naming `ghost` at its own span, and a dimension-only factor refuses the same unit-namespace
    lookup. Existing source-integer and stage ceilings apply to long products and
    large exponents, with a typed refusal or limit stop and no truncated exponent
    or authoritative partial compound type.'
- id: TC-483-step-20
  path: spec/test-cases/TC-483-assembler-admits-dimensions-and-units.md
  role: examined
  excerpt: "20. Declare canonical roots `kg`, `m` and `s` of base dimensions `Mass`,\n\
    \    `Length` and `Time`, and a declared root `J` of a distinct derived\n    dimension\
    \ with terms `Mass * Length^2 * Time^-2`."
- id: FR-091-compound-source
  path: spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md
  role: examined
  excerpt: 'In a complete-V1 `Quantity<U>` or `Quantity<U, C>` type reference, S1
    SHALL

    accept `U` as a product of qualified unit names, each optionally followed by

    `^` and a `signed-int` literal. For example, `kg*m^2*s^-2` has three factors

    with exponents `1`, `2` and `-2`. S2 SHALL retain every factor''s qualified

    name, signed exponent and source span as syntax, without substituting a

    declared unit name or minting a semantic identity. The carrier comma and the

    closing `>` delimit the unit expression; the existing named-unit and carrier

    forms keep their meaning.


    The assembler SHALL resolve every written '
```

## Final combined integration review

Final combined production review: merge 231a8deba includes upstream 27adf2f49 (PR 2490). The upstream change threads package-identity sets for unbounded Rational and aliases only into model tuple-position and variant-payload intake. Its refusal predicate matches ValueTypeRef::Package identities; it does not inspect or reject a source ValueType::Quantity, a formed compound unit, the default exact quantity carrier, or source nested TypeForm arguments. Source compound checking/assembly and Cargo.lock remain unchanged from the reviewed feature. Bounded model Rational and source bounded carriers retain their existing paths. No integration defect was found. Refreshed focused checks on 231a8deba passed: compound semantics 4, strict producer/reader 1, existing carrier identity including Rational 1, and semantic library Clippy. The later aggregate gate exposed the forms-core import-boundary violation; 190518b88 fixes that import path without changing the architecture guard. The architecture guard 1, relevant forms 5 and formatting checks then passed. The original aggregate failure and earlier aborted run carry no aggregate pass credit; a fresh root-owned aggregate final gate remains required.

## Current main integration review

Final current-main integration review: combined head b2366d543 includes af9fac7a6 (PR 2497). The upstream production delta deletes retired temporal mapping/profile/result API portions and removes their root re-exports. It changes no Quantity grammar, syntax forms, quantity checker, package producer implementation, default carrier, unit metadata or checked wire representation. The only qsl-package changes are mapping-support tests; compound emission still uses the unchanged producer_bytes/read_back helpers and remains registered in tests/it/main.rs. No compound-quantity integration defect was found. FND-002 remains fixed. This is a source integration review; fresh final-gate evidence for the new combined head is still required.

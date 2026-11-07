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

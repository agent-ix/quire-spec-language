---
id: FR-354
title: "Admit extensions from declarative grammar and typed-node schemas"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-035
    type: implements
  - target: ix://agent-ix/quire-spec-language/US-006
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-002
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-110
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-111
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-133
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-132
    type: depends_on
---
# FR-354: Admit extensions from declarative grammar and typed-node schemas

## Description

When a unit selects an extension definition, QSL SHALL admit the
extension's declaration forms from the definition's declarative grammar
schema and typed-node schema: S1 parses the forms the grammar schema
defines, S3 checks each parsed form against the typed-node schema, and the
checked package carries one typed node per form.

Extension forms fit ADR-012's closed seams as one `Extension` variant of the
parsed form and leading-token enums (S2) and of the checked node enum (S3),
carrying the extension's definition identity, its schema node kind and its
subnodes. One schema-driven builder checks them from the catalog's typed-node
schema (ADR-012 §4.2 and §5.1, amended in place). Adding an extension adds
catalog data, never a Rust variant.

QSpec FR-133 owns what an extension may and may not change in the
language, and QSpec owns the grammar schema and typed-node schema members
of an extension definition (STD-146). This requirement specifies how the QSL
compiler reads them and admits one.

## Inputs

- The definition catalog (FR-111). QSL reads these members of an
  extension definition, which QSpec defines (STD-146):
  - its grammar schema: one or more declaration-form productions over the
    declarative token vocabulary (FR-002), each beginning with a keyword;
  - its typed-node schema: for each production, the node kind and one typed
    subnode per clause, each subnode naming a checked type or node kind
    (ADR-012 §4);
  - its dependency edges and capability ids, as every definition does.
- The unit's header selections (FR-110), which name the extensions the
  unit selects.

## Outputs

A checked package whose extension declarations are typed nodes of the
extension's node kinds, or a typed refusal naming the definitions and the
source locus involved.

## Behavior

### Admission

- QSL SHALL parse a selected extension's declaration forms from its grammar
  schema with the S1 parser, under the same stage limits as core forms.
- S1 SHALL recognise a selected extension's leading keyword only as the
  first token of a declaration. Anywhere else the same word SHALL remain an
  ordinary identifier, so selecting an extension changes the meaning of no
  source that already parses.
- When a declaration begins with a word that is neither a core keyword nor
  the leading keyword of an extension the unit selects, S1 SHALL refuse it
  `unsupported_construct`/`declaration-form`, naming the word, its span and
  the unit's selections, whatever the catalog holds.
- S3 SHALL check each parsed extension form against its typed-node schema:
  each clause SHALL be present, and each clause's subnode SHALL check
  against the type or node kind the schema names.
- The checked package SHALL carry each admitted extension form as a typed
  node of the schema's node kind, naming the extension definition, with
  the form's source span.

### Refusals

| Case | Code | Cause | Names |
| --- | --- | --- | --- |
| an extension definition with no grammar schema, or no typed-node schema, or a production with no node kind | `invalid_package` | `missing-member` | the definition and the missing member |
| a production whose leading keyword is a core keyword or another selected extension's leading keyword | `invalid_package` | `conflicting-definition` | both definitions and the keyword |
| a node kind equal to a core node kind or another selected extension's node kind | `invalid_package` | `conflicting-definition` | both definitions and the node kind |
| a dependency cycle among extension definitions | `invalid_package` | `definition-cycle` | the cycle in path order (FR-111) |
| a declaration beginning with a word that is neither a core keyword nor a selected extension's leading keyword | `unsupported_construct` | `declaration-form` | the word, its span and the unit's selections |
| a form missing a clause | `invalid_package` | `missing-member` | the clause, the form's span and the schema node kind |
| a clause whose subnode does not check against its schema type | `ill_typed` | `type-mismatch` | the clause, its span, the expected and the actual type |
| a QSL reader given a checked package holding a node of an extension its catalog does not hold | `unknown_required_feature` | `unknown-feature` | the extension and the node; the refusal retains the package's source and its exact identity (QSpec FR-133-AC-3) |

### Independence

- A unit that does not select an extension SHALL have the same checked
  package identity whether or not the catalog holds that extension.
- Admission SHALL read only the catalog and the unit. The set of
  registered backends changes no admission and no checked package identity.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-354-AC-1 | A catalog holding an extension whose grammar schema adds a `retention` declaration form (keyword `retention`, clauses `subject: Text` and `days: Int[1, 3650]`) admits a unit that selects it and declares `retention Audit { subject: "audit", days: 30 }`; the checked package holds one typed node of the schema's node kind naming the extension, with both subnodes and the form's span. | Test (TC-882) |
| FR-354-AC-2 | The same declaration with `days: "thirty"` refuses `ill_typed`/`type-mismatch` naming the `days` clause, `Int[1, 3650]` and `Text`; with the `days` clause absent it refuses `invalid_package`/`missing-member` naming the `days` clause. | Test (TC-882) |
| FR-354-AC-3 | An extension definition with no typed-node schema refuses `invalid_package`/`missing-member` naming it. A production with leading keyword `function` refuses `invalid_package`/`conflicting-definition` naming the extension, the core grammar and `function`; two selected extensions with one leading keyword, or one node kind, refuse the same way naming both. Two extensions depending on each other refuse `invalid_package`/`definition-cycle` naming both in path order. | Test (TC-883) |
| FR-354-AC-4 | A unit declaring the `retention` form without selecting the extension refuses `unsupported_construct`/`declaration-form` naming `retention`, its span and the unit's selections, and refuses identically over a catalog with and without the `retention` extension. A unit that selects the extension and names a parameter `retention` inside a function body compiles, with `retention` an ordinary identifier there. | Test (TC-883) |
| FR-354-AC-5 | A unit that selects no extension has an equal checked package identity over a catalog with and without the `retention` extension, and the AC-1 package identity is equal with two registered backends and with none. A QSL reader whose catalog lacks the `retention` extension, given the AC-1 package, refuses `unknown_required_feature`/`unknown-feature` naming the extension and the node, and the refusal retains the package's source and its `package_id`. | Test (TC-884) |

## Dependencies

- QSpec FR-133 (the extension mechanism and its refusals) and FR-132 (the
  package's required-feature set; an unknown required capability refuses).
- [ADR-012](../decisions/ADR-012-semantic-family-extension-contracts.md)
  §4 (a construct with independent clauses is a typed node with one typed
  subnode per clause; the schema-driven builder) and §5.1 (the `Extension`
  variant of S2 and S3).
- STD-146: the QSpec definition of the grammar and typed-node schema
  members.
- [FR-002](FR-002-parse-native-units.md) (declarative token recognition),
  [FR-110](FR-110-resolve-header-profile-selections-at-e3.md) (header
  selections), [FR-111](FR-111-link-a-complete-v1-definition-bundle.md)
  (the definition catalog and cycle refusal).

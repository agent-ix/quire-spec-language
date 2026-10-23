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
  - target: ix://agent-ix/quire-spec-language/FR-092
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-110
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-111
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-133
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-132
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-467
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-469
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-322
    type: depends_on
---
# FR-354: Admit extensions from declarative grammar and typed-node schemas

## Description

When a unit selects an extension definition, QSL SHALL admit the
extension's declaration forms from the definition's declarative grammar
schema and typed-node schema: S1 parses the forms the grammar schema
defines, S3 checks each parsed form against the typed-node schema, and the
checked package carries one `extension`/`extension_form` node per form.

Extension forms fit ADR-012's closed seams as one `Extension` variant of the
parsed form and leading-token enums (S2) and of the checked node enum (S3),
carrying the extension's definition identity, its schema node kind and its
subnodes. One schema-driven builder checks them from the catalog's typed-node
schema (ADR-012 §4.2 and §5.1, amended in place). Adding an extension adds
catalog data, never a Rust variant.

QSpec owns the contract this requirement implements and QSL cites it by
reference: FR-133 owns what an extension may and may not change in the
language; [FR-467](ix://agent-ix/quire-specification/FR-467) owns the
grammar schema and typed-node schema members of an extension definition,
their validation, the production's source shape and its source-level
refusals; [FR-469](ix://agent-ix/quire-specification/FR-469) owns the
checked-package `extension` node, its key preimage, its lock carriage and the
reader joins; [FR-322](ix://agent-ix/quire-specification/FR-322) owns the
graph, the body grammar and the reader's first-refusal order. Where this
requirement names a wire form or a refusal code, the QSpec requirement is
the authority. This requirement specifies how the QSL compiler reads the
members, admits a form, emits its node and reads it back.

## Inputs

- The definition catalog (FR-111). QSL reads these members of an
  extension definition, in QSpec FR-467's "Members" shape:
  - `grammar_schema`: one or more productions, each a leading keyword and
    its clause names in source order, of FR-467's one production shape
    `leading-keyword ident '{' [clause {',' clause}] '}'`, with
    `clause = ident ':' expr`;
  - `typed_node_schema`: one node entry per production, matched by keyword,
    giving the node kind and, per clause, the core `type-ref` or node kind
    its value checks against (ADR-012 §4);
  - its dependency edges and capability ids, as every definition does.
- The unit's header selections (FR-110), which name the extensions the
  unit selects.

## Outputs

A checked package whose extension declarations are `extension` nodes of the
extension's node kinds, or a typed refusal naming the definitions and the
source locus involved.

## Behavior

### Definition validation

- The catalog SHALL validate each selected extension definition by QSpec
  FR-467 "Validation" before S1 reads it, reporting the first violated rule
  (`missing-member`, then `conflicting-definition`, then `definition-cycle`)
  as `invalid_package` naming the definitions involved, and no later one.
- Only a definition that passed that validation is held: S1, S3 and the
  reader's evidence read held definitions only.

### Admission

- QSL SHALL parse a selected extension's declaration forms from its grammar
  schema with the S1 parser, under the same stage limits as core forms.
- S1 SHALL recognise a selected extension's leading keyword only as the
  first token of a top-level declaration of the unit. In a member
  declaration inside a model, record, state or protocol body, in an
  expression and anywhere else, the same word SHALL remain an ordinary
  identifier, so selecting an extension changes the meaning of no source
  that already parses (QSpec FR-467 "Production").
- When a top-level declaration begins with a word that is neither a core
  keyword nor the leading keyword of an extension the unit selects, S1 SHALL
  refuse it `unsupported_construct`/`declaration-form`, naming the word, its
  span and the unit's selections, whatever the catalog holds.
- S1 SHALL refuse a form that does not match its production as QSpec
  FR-467 "Production" states, at the source level: a missing declared name,
  brace, separator or clause value, or a trailing separator, refuses
  `invalid_syntax`/`unexpected-token` (or `unexpected-end` at the end of the
  input) naming the actual token and the expected category; a clause absent
  from its position, out of the production's order, of an unknown name or
  repeated refuses `invalid_syntax`/`unexpected-token` naming the token at
  that position and the clause the production expects there.
- S3 SHALL check each clause value of a parsed form against the type or
  node kind its typed-node schema entry names.

### Emission

The emitter SHALL write each admitted form as QSpec FR-469 states:

- one node with `node_tag` `extension` and `semantic_form`
  `extension_form`, whose body is
  `{term: "extension_form", definition, node_kind, clauses}`: `definition`
  the extension definition's `DefinitionRef` `{authority, identity}`,
  `node_kind` the node kind of the production's typed-node schema entry, and
  `clauses` one `binding` per clause in the production's clause order, each
  binding's `value` a `reference` to the node of the clause's checked value,
  never an inline `literal`, `aggregate` or `application`;
- the form's declared name as the node's `declaration`, the declaring unit's
  `SourceOwner`, exactly one `declaration` occurrence at ordinal 0 whose
  `source_map` region spans the whole form, from the leading keyword through
  the closing brace, and `dependencies` exactly the unique digest-ascending
  clause targets;
- the node keyed by FR-092 rule 3 with FR-469's `quire.structural-node/v1`
  `ExtensionFormNode` preimage `{version, node_tag: "extension",
  semantic_form: "extension_form", semantic_type: null, declaration,
  recursion: null, owner, body}`, `body` holding each clause reference's
  target as its node key, so the definition identity, the node kind, the
  declared name, the owner and every clause subnode's key enter the node id
  and `package_id`, and the source span enters neither;
- the extension definition's `DefinitionRef` in `lock.definition_selections`,
  and no extension definition identity in `lock.required_features`.

### Reading

- QSL's I2 reader `read_checked_package_v2(bytes, identity, limits,
  evidence, pinned)` SHALL receive the extension definitions it holds in its
  `evidence: &CheckedPackageEvidence` argument, the IR evidence it passes to
  `read_checked_package`. Every QSL caller that builds that evidence
  (FR-367's `admission_evidence` included) SHALL insert, with
  `insert_extension_definition(reference, definition)`, each held extension
  definition of its catalog that a `lock.definition_selections` entry of the
  package names, under that `DefinitionRef`, with exactly the definition's
  FR-467 `grammar_schema` and `typed_node_schema`.
- The held definitions are reader input, never artifact data: inserting or
  withholding them changes no package byte, node id or `package_id`.
- The reader SHALL report the first failing FR-469 reader join of the first
  failing `extension` node, in ascending node-id order, as IR reports it,
  located at `Locus::Artifact` with the supplied bytes' `raw-artifact-digest`
  record and the RFC 6901 pointer IR reports (FR-096).

### Refusals

| Case | Stage | Code | Cause | Names |
| --- | --- | --- | --- | --- |
| an extension definition violating QSpec FR-467 rule 1 (a missing member) | catalog | `invalid_package` | `missing-member` | the definition and the missing member |
| a leading keyword or node kind equal to a core one, another selected extension's, or another of the same definition (FR-467 rule 2) | catalog | `invalid_package` | `conflicting-definition` | the definitions, or the core grammar, and the keyword or node kind |
| a dependency cycle among extension definitions (FR-467 rule 3) | catalog | `invalid_package` | `definition-cycle` | the cycle in path order (FR-111) |
| a top-level declaration beginning with a word that is neither a core keyword nor a selected extension's leading keyword | S1 | `unsupported_construct` | `declaration-form` | the word, its span and the unit's selections |
| a form missing its declared name, a brace, a separator or a clause value, or with a trailing separator | S1 | `invalid_syntax` | `unexpected-token` or `unexpected-end` | the actual token and the expected category |
| a form with a clause absent, out of order, of an unknown name or repeated | S1 | `invalid_syntax` | `unexpected-token` | the token at that position and the expected clause |
| a clause whose value does not check against its schema type or node kind | S3 | `ill_typed` | `type-mismatch` | the clause, its span, the expected and the actual type |
| a QSL reader given a checked package holding an `extension` node whose definition it does not hold (FR-469 join 2) | I2 | `unknown_required_capability` | none | the node's `node_id`, at `/semantic_graph/nodes/{i}/body/definition`; the package's lock, source map and node are unchanged (QSpec FR-133-AC-3) |

### Independence

- A unit that does not select an extension SHALL have the same checked
  package identity whether or not the catalog holds that extension.
- Admission SHALL read only the catalog and the unit. The set of
  registered backends changes no admission and no checked package identity.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-354-AC-1 | A catalog holding an extension whose grammar schema adds a `retention` declaration form (keyword `retention`, clauses `subject: Text` and `days: Int[1, 3650]`) admits a unit that selects it and declares `retention Audit { subject: "audit", days: 30 }`; the checked package holds one typed node of the schema's node kind naming the extension, with both subnodes and the form's span. | Test (TC-882) |
| FR-354-AC-2 | The same declaration with `days: "thirty"` refuses `ill_typed`/`type-mismatch` naming the `days` clause, `Int[1, 3650]` and `Text`. `retention Audit { subject: "audit" }` (the `days` clause absent) refuses at S1 `invalid_syntax`/`unexpected-token` naming the `}` and the expected clause `days`; `retention Audit { days: 30, subject: "audit" }` refuses the same naming `days` and the expected clause `subject`; `retention Audit { subject: "audit", }` refuses `invalid_syntax`/`unexpected-token` naming the `}` and the expected category. None of the four refusals is `invalid_package`. | Test (TC-882) |
| FR-354-AC-3 | An extension definition with no typed-node schema refuses `invalid_package`/`missing-member` naming it. A production with leading keyword `function` refuses `invalid_package`/`conflicting-definition` naming the extension, the core grammar and `function`; two selected extensions with one leading keyword, or one node kind, refuse the same way naming both. Two extensions depending on each other refuse `invalid_package`/`definition-cycle` naming both in path order. | Test (TC-883) |
| FR-354-AC-4 | A unit declaring the `retention` form without selecting the extension refuses `unsupported_construct`/`declaration-form` naming `retention`, its span and the unit's selections, and refuses identically over a catalog with and without the `retention` extension. A unit that selects the extension, names a parameter `retention` inside a function body and declares a record member named `retention` compiles, with `retention` an ordinary identifier in both places. | Test (TC-883) |
| FR-354-AC-5 | A unit that selects no extension has an equal checked package identity over a catalog with and without the `retention` extension, and the AC-1 package identity is equal with two registered backends and with none. QSL's I2 reader given the AC-1 package with evidence holding the `retention` definition admits it; with evidence withholding it, it refuses `unknown_required_capability` with no cause, never `unknown_required_feature`, naming the `Audit` node's `node_id`, at `Locus::Artifact` with the package's `raw-artifact-digest` record and the pointer `/semantic_graph/nodes/{i}/body/definition`, and the package's bytes and `package_id` are those it was given. | Test (TC-884) |
| FR-354-AC-6 | The AC-1 emission holds one `extension`/`extension_form` node with `declaration` `["Audit"]`, the unit's `SourceOwner`, the `retention` definition's `DefinitionRef`, `node_kind` `retention`, the clause bindings `subject` and `days` in that order, each a `reference` to a subnode typed `Text` and `Int[1, 3650]`, `dependencies` exactly those two subnodes in digest order, and one `declaration` occurrence whose `source_map` region spans the whole form; its lock lists the definition in `definition_selections` and lists no definition identity in `required_features`. The node id recomputes from its `ExtensionFormNode` preimage; recompiling with a different declared name, node kind or clause value changes the node id and `package_id`, and recompiling the unit with the form moved by inserted blank lines changes neither. | Test (TC-882) |

## Dependencies

- QSpec [FR-467](ix://agent-ix/quire-specification/FR-467) (the definition
  members, their validation, the production shape and its source-level
  refusals), [FR-469](ix://agent-ix/quire-specification/FR-469) (the
  `extension` node, its key, its lock carriage and the reader joins),
  [FR-322](ix://agent-ix/quire-specification/FR-322) (the graph and the
  reader's refusal order), FR-133 (the extension mechanism) and FR-132 (the
  package's required-feature set).
- [ADR-012](../decisions/ADR-012-semantic-family-extension-contracts.md)
  §4 (a construct with independent clauses is a typed node with one typed
  subnode per clause; the schema-driven builder) and §5.1 (the `Extension`
  variant of S2 and S3).
- [FR-002](FR-002-parse-native-units.md) (declarative token recognition),
  [FR-092](FR-092-key-type-parameter-and-declared-nodes.md) (rule 3, the
  `quire.structural-node/v1` key),
  [FR-096](FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md)
  (the I2 reader's locus),
  [FR-110](FR-110-resolve-header-profile-selections-at-e3.md) (header
  selections), [FR-111](FR-111-link-a-complete-v1-definition-bundle.md)
  (the definition catalog and cycle refusal).

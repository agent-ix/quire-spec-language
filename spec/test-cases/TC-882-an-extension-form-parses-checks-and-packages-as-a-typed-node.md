---
id: TC-882
title: "An extension's declaration form parses, checks against its typed-node schema and packages as a typed node"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-354
    type: verifies
---
# TC-882: An extension's declaration form parses, checks against its typed-node schema and packages as a typed node

## Description

Verify admission of a declaration form from an extension's grammar and typed-node schemas, the source-level and clause type refusals, and the emitted `extension`/`extension_form` node (QSpec FR-467, FR-469).

Scope: FR-354-AC-1, FR-354-AC-2, FR-354-AC-6.

## Test Procedure

Build a definition catalog holding an extension whose grammar schema adds the production `retention <name> { subject: <expr>, days: <expr> }` and whose typed-node schema gives node kind `retention` with clauses `subject: Text` and `days: Int[1, 3650]`.

1. Compile a unit that selects the extension and declares `retention Audit { subject: "audit", days: 30 }`.
2. Compile it with `days: "thirty"`; then as `retention Audit { subject: "audit" }`, `retention Audit { days: 30, subject: "audit" }` and `retention Audit { subject: "audit", }`.
3. Emit step 1's package and read its `Audit` node and lock. Recompute the node id from its `ExtensionFormNode` preimage. Recompile with the declared name `Log`, then with a node kind `audit_retention` in the schema, then with `days: 31`, then with two blank lines inserted before the form, and compare node ids and `package_id`s with step 1's.

Tag the tests `#[trace("TC-882", "FR-354-AC-n")]`.

## Expected Results

- Step 1: the checked package holds one node of kind `retention` naming the extension, with subnodes `"audit"` and `30` and the form's span.
- Step 2: `ill_typed`/`type-mismatch` naming `days`, `Int[1, 3650]` and `Text`; then, at S1, `invalid_syntax`/`unexpected-token` naming `}` and the expected clause `days`; naming `days` and the expected clause `subject`; and naming `}` and the expected category. No step-2 refusal is `invalid_package`.
- Step 3: one `extension`/`extension_form` node with `declaration` `["Audit"]`, the unit's `SourceOwner`, the definition's `DefinitionRef`, `node_kind` `retention`, bindings `subject` then `days`, each a `reference` to a subnode typed `Text` and `Int[1, 3650]`, `dependencies` exactly those two subnodes in digest order, and one `declaration` occurrence spanning the whole form; `lock.definition_selections` lists the definition and `lock.required_features` lists no definition identity. The recomputed node id equals the emitted one. The name, node-kind and clause-value recompiles change the node id and `package_id`; the blank-line recompile changes neither.

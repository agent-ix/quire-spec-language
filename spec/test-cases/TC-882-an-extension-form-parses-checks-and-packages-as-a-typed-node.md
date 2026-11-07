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

Scope: FR-354-AC-1, FR-354-AC-2, FR-354-AC-6, FR-354-AC-7, FR-354-AC-8.

## Test Procedure

Build a definition catalog holding, as a `DefinitionRole::Extension` definition, an extension whose grammar schema adds the production `retention <name> { subject: <expr>, days: <expr> }` and whose typed-node schema gives node kind `retention` with clauses `subject: Text` and `days: Int[1, 3650]`. Every compile runs FR-354 "Compile path" through spine `compile` with a `ProfileCatalogInput` over the catalog: the unit's header profile selects the extension by its identity, E3 resolves it (FR-110 "Extension selection"), the catalog holds it, S1 parses with its production, the builder builds the form, and S3 checks it.

1. Compile a unit that selects the extension and declares `retention Audit { subject: "audit", days: 30 }`.
2. Compile it with `days: "thirty"`; then as `retention Audit { subject: "audit" }`, `retention Audit { days: 30, subject: "audit" }` and `retention Audit { subject: "audit", }`; then with `days: 0` and with `days: 3651`.
3. Emit step 1's package and read its `Audit` node. Recompute the node id from its `ExtensionFormNode` preimage. Recompile with the declared name `Log`, then with a node kind `audit_retention` in the schema, then with `days: 31`, then with two blank lines inserted before the form, and compare node ids and `package_id`s with step 1's.

4. Add an extension whose typed-node schema gives clauses `b: Boolean`, `i: Integer`, `t: Text`, `r: Int[0, 9]` and `w: Window`, and compile a unit that declares `record Window` and a form of it with values `true`, `7`, `"x"`, `3` and a `Window` value. Then respell one clause `Option<Text>`, then `Int[0, 9; floor]`, and compile again.
5. Add a selected extension with production `archive <name> { policy: <expr> }`, node kind `archive`, `policy` typed by node kind `retention`. Compile `archive Logs { policy: Audit }` declared before step 1's form; then with `policy: Missing`; then with `policy: Other` beside `archive Other { policy: Audit }`. Then add an extension with production `link <name> { to: <expr> }`, `to` typed by node kind `link`, and compile `link A { to: B }` with `link B { to: A }`.

Tag the tests `#[trace("TC-882", "FR-354-AC-n")]`.

## Expected Results

- Step 1: the checked package holds one node of kind `retention` naming the extension, with subnodes `"audit"` and `30` and the form's span.
- Step 2: `ill_typed`/`type-mismatch` naming `days`, `Int[1, 3650]` and `Text`; then, at S1, `invalid_syntax`/`unexpected-token` naming `}` and the expected clause `days`; naming `days` and the expected clause `subject`; and naming `}`, the expected category `clause-name` and the clause `days`; then `ill_typed`/`type-mismatch` naming `days`, expected `Int[1, 3650]` and actual `Int[0, 0]`, and the same with actual `Int[3651, 3651]`. No step-2 refusal is `invalid_package`.
- Step 3: one `extension`/`extension_form` node with `declaration` `["Audit"]`, the unit's `SourceOwner`, the definition's `DefinitionRef`, `node_kind` `retention`, bindings `subject` then `days`, each a `reference` to a subnode typed `Text` and `Int[1, 3650]`, `dependencies` exactly those two subnodes in node-key order, and one `declaration` occurrence spanning the whole form. The recomputed node key equals the emitted one. The name, node-kind and clause-value recompiles change the node key; the blank-line recompile changes neither.
- Step 4: the first unit compiles. Each respelled extension is held with no refusal, and its form refuses at S3 `ill_typed`/`type-mismatch` naming the clause and the spelling `Option<Text>`, then `Int[0, 9; floor]`.
- Step 5: `Logs` admits with its `policy` binding a `reference` to the `Audit` node; `policy: Missing` refuses `missing_declaration`/`missing-name` naming `Missing`; `policy: Other` refuses `ill_typed`/`type-mismatch` naming `policy`, expected `retention`, actual `archive`; the `link` pair refuses `ill_typed`/`type-mismatch` from `DeclarationCause::Recursion` naming the cycle, at `A` and at `B`.

---
id: TC-1021
title: "A protocol reference resolves its target by typed key and emits one carrier"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-308
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-092
    type: verifies
---
# TC-1021: A protocol reference resolves its target by typed key and emits one carrier

## Description

Verify that a protocol reference expression builds, resolves its target by typed node key, refuses an unknown, non-protocol or stale target and a duplicate reference, and emits exactly the carrier QSpec FR-322-AC-66 specifies, whose key follows its bound target. Scope: FR-308-AC-6, FR-218-AC-8, FR-218-AC-9, FR-092-AC-16, FR-092-AC-17, FR-092-AC-18.

Catches an implementation that binds by name and looks it up again at emission, one that emits a `dependency_reference` or a separate collection, one that records two occurrences on one carrier, one that carries a semantic type other than the target's, and one that keys the carrier without its target.

## Test Procedure

1. Build a unit that declares `protocol Fill` and a second protocol `Drain`, and holds `protocol Fill`, then `protocol M::Fill` against a model alias `M`, as expressions. Inspect the S2 forms.
2. Check the unit with one `protocol Fill` expression. Inspect the checked reference's bound key and type.
3. Emit it. Inspect the graph, including the node emitted for `Fill`, and read the package with the IR v2 reader.
4. Check a unit with `protocol Missing`; with `protocol Lib::Fill` where `Fill` exists only in an imported library; with a name that resolves to a declared function; and with two `protocol Fill` expressions.
5. Take the checked graph of step 2 and re-key the `Fill` node (change a member of its declaration) without re-binding the reference. Emit it.
6. Emit the unit of step 2 with the reference written against `Fill`, then against `Drain`, then emit the first again. Then change `Fill` so its key changes and emit again.

## Expected Results

- Step 1 builds `ProtocolReference` forms in source order with targets `Fill` and `M::Fill` and the spans of the whole expressions, and resolves nothing.
- Step 2 binds the reference to `Fill`'s node key and gives it `Fill`'s semantic type.
- Step 3 emits one `protocol`/`protocol_clause` node for `Fill` (a `quire.op.protocol.control` application body with the selected `protocol_profile` law and `profile_operator` member, the Boolean `semantic_type`, an application-node key, one `declaration` occurrence) and no node for its roles, `run` tree or `finish`, and one `protocol`/`protocol_reference` node with body exactly `{term: "reference", target: <Fill key>}`, the `Fill` node's `semantic_type`, `dependencies` exactly `[<Fill key>]`, one `expression` occurrence at the reference's region, and no `declaration`, `owner`, nominal preimage or recursion group; one node collection, no protocol-reference collection. The reader admits it as `ProtocolForm::ProtocolReference`.
- Step 4 refuses `missing_declaration`/`missing-name` at the reference naming `Missing`; the import-only name refuses the same way; the function refuses `ill_typed`/`operator-ineligible`; the second `protocol Fill` refuses `unsupported_construct`/`expression-form` at the second reference. None produces a package.
- Step 5 refuses `missing_declaration`/`missing-name` at the reference naming the bound key, with no package and no name lookup.
- Step 6 gives different reference keys for `Fill` and `Drain`, equal keys for the repeated first emission, and a changed reference key and package identity once `Fill`'s key changed. The reference key equals the digest of its `quire.structural-node/v1` preimage computed independently.
- Assertions compare typed fields, causes and keys, never message text.

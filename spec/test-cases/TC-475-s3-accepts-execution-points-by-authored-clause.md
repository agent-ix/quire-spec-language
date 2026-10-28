---
id: TC-475
title: "S3 routes accepted execution points by authored state clause"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-104-AC-15
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-049
    type: references
---
# TC-475: S3 routes accepted execution points by authored state clause

## Description

Verify the supported S3 check input resolves accepted execution points to
authored clauses without inferring missing bindings.

Scope: FR-104-AC-15.

## Test Procedure

1. Using TC-459 step 7's `Flow::go` package, add equal-body invariants
   `Init` and `Handle`, a `constraint Guard`, a precondition `Before` and a
   postcondition `After` on `go`. Call
   `qsl_replay::spine::check_with_execution_points` directly with a
   `StateClauseExecutionPoints` map containing all five entries:
   `Initialization{boot}` for `Init` and `Guard`, `Handler{wake}` for
   `Handle`, `Pre{go}` for `Before`, and `Post{go}` for `After`.
2. Using the corresponding source without `constraint Guard`, compile through
   the native adapter using a complete, validated `CheckBindings.clauses`
   vector for `Init`, `Handle`, `Before` and `After`. Verify the adapter
   projects the matching `ClauseBinding.name` and `.execution_point` values
   into the map.
3. Call `check_with_execution_points` directly with the `Before` entry
   omitted.
4. In a separate check, map `Before` to `Handler{wake}`.
5. In a separate check, map `Before` to `Post{go}`.
6. In a separate check, map `Before` to `Pre{other}`.
7. In a separate check, map `After` to `Pre{go}`.
8. In a separate check, map `Init` to `Pre{go}`.
9. Supply a `NotAuthored` key with no matching clause.
10. Supply a key that names only a function.
11. Add a clause/function name collision as in FR-104-AC-3 and supply a map
    entry for that name.
12. For duplicate native binding names, use TC-049's `CheckBindings`
    validation case.
13. Using TC-1271's Boolean `Switch::set` fixture, compile an eligible
    `post Q1 { self.on }` once with no map entry and once with accepted
    `Post{set}`; compare the authored clause and derived transition outputs.

Tag the tests `#[trace("TC-475", "FR-104-AC-15")]`.

## Expected Results

- Steps 1-2: each supplied point is retained unchanged on its exact checked
  clause. `Init` and `Handle` carry different points and have distinct node
  identities despite equal bodies. `Guard` accepts an initialization point
  as an invariant does. The native route remains subject to TC-049's complete,
  no-extra-entry rule.
- Step 3: `Before` has no point; S3 does not infer one.
- Steps 4-8: each inapplicable variant or mismatched operation name refuses
  `invalid_model_binding`/`malformed-declaration` at the affected authored
  clause-name span before lowering; no checked clause or node identity is
  produced. The operation anchor name is resolved from the operation member
  authored in `on M::T::op`.
- Steps 9-10: an unknown or non-clause map key refuses
  `missing_declaration`/`missing-name` at the check unit's source-start
  region, before lowering or identity creation.
- Step 11: the clause/function name collision refuses first at both source
  declaration spans per FR-104-AC-3, before checking the map entry.
- Step 12: duplicate native bindings refuse at the native check source-start
  region per TC-049.
- Step 13: the absent-point `Q1` retains its FR-105-AC-16 transition
  eligibility; the point-bearing `Q1` is not used in a derived transition.
  Both variants retain the authored clause node.

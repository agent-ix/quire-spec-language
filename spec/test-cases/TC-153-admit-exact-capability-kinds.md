---
id: TC-153
title: "Admit exactly the ten capability kinds and refuse every other label"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: verifies
---
# TC-153: Admit exactly the ten capability kinds and refuse every other label

## Description

Verify that the canonical `Capability` value type admits exactly the ten labels
of FR-057's `quire.capability-kind/v1` table, compares kinds by label, keeps
every requested pair at its index, refuses an absent or unknown label without
mapping it, and that each claim form requests its FR-290 kind. Scope:
FR-057-AC-1, FR-057-AC-2, FR-057-AC-4, FR-057-AC-7 and FR-057-AC-10.

## Test Procedure

1. Hold FR-057's v1 table (quire-specification FR-290 at the revision FR-057
   pins) as a test constant. Parse each label, serialize the result, and compare the admitted
   set with the constant's set.
2. Submit a requested pair with no label, then one whose label is JSON `null`.
3. Submit, one at a time: `""`, the number `1`, `Global-Conformance`,
   `value_validity`, `Operation contract`, ` composition`, `ValueValidity`,
   `FamilyCheck`, `StateOperation`, `FiniteReplay` and `TemporalProjection`. Mark one of them required and one not required.
4. Property: generate arbitrary UTF-8 strings and JSON values; each one not
   byte-equal to an admitted label refuses with `unknown-kind` or `absent-kind`
   as step 2 and 3 define.
5. Build two sets holding the same kinds in different orders and compare them.
   Permute three requested pairs over different kinds. Submit two pairs with the
   same declaration and kind and different `required` flags.
6. Scan the QSL source tree for types that parse or emit an FR-290 label.
7. Check one of each claim form in FR-057's claim-form table: a function
   application clause, a clause with a nested `case` expression and function
   application, an operation precondition, postcondition and invariant, a frame
   obligation, a contract refinement gate, an operation bound by an abstraction
   relation, a finite replay, a temporal requirement, a protocol refinement, a
   `global-conformance` claim, an exhaustive `case` and a non-exhaustive
   `case`.

## Expected Results

- Step 1: every label round-trips to byte-identical JSON, and the admitted set
  equals the constant's set with no extra or missing member.
- Step 2: both pairs refuse with `invalid_capability`/`absent-kind`, and no kind
  is defaulted.
- Steps 3 and 4: each pair refuses with `invalid_capability`/`unknown-kind`,
  naming the exact received bytes, with no replacement kind; a generated JSON
  `null` in step 4 refuses with `absent-kind` as in step 2. Each refused pair
  stays at its index. The refused required pair makes aggregate success
  unavailable; the refused optional pair does not.
- Step 5: the sets compare equal. Each permuted pair keeps its own kind, and
  only the request indices change. The two same-kind pairs stay at their own
  indices with their own flags.
- Step 6: exactly one type, the canonical `Capability`, parses or emits a
  label.
- Step 7: each item records exactly one kind: `value-validity` (twice),
  `operation-contract` (for each of the three clauses, the frame obligation,
  the refinement gate's implications and the bound operation's clauses),
  `finite-replay`, `temporal-satisfaction`, `refinement` and
  `global-conformance`. The abstraction relation and the exhaustive `case`
  record no kind. The non-exhaustive `case` refuses as
  `undefined_expression`/`unproved-exhaustiveness` and records no kind.
- Assertions compare typed codes, causes and received bytes, never message
  text.

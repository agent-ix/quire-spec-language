---
id: FR-316
title: "Check union declarations and admit them into the type environment"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-032
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-313
    type: traces_to
  - target: ix://agent-ix/quire-specification/FR-143
    type: depends_on
---
# FR-316: Check union declarations and admit them into the type environment

## Description

When S2 yields a `UnionForm` (FR-313), the FR-091 assembler SHALL register
the union in the package's type environment as a composite declaration
beside records and tuples, and the `SumCase` family `check` SHALL admit it
or refuse it with one located cause: duplicate member names, unresolved
payload types, and QSpec FR-143's recursion rule (ADR-012 §16.4 S3
declaration row; SC-Q1 option (a)).

## Inputs

- A `UnionForm`; the assembled `PackageDeclarations`; `CheckContext`.

## Outputs

- A checked union type node: a (`composite_type`, `union`) node whose
  members are in declared order, each with its ordered payload types.
- A `quire_semantic_value::declaration::CompositeDeclaration` with a union
  shape in the package's `TypeEnvironment`, so `ValueType::Composite(union
  node key)` is the union's kernel type.
- Or a refusal with a catalog code, or `StageFailure::Limit`.

## Behavior

- The `SumCase` family SHALL implement the shared family contract
  (`FamilyContract`) for union declarations (FR-062).
- The checker SHALL resolve each payload type reference exactly as a record
  field's type is resolved (FR-091). If a payload type names no declaration,
  then the checker SHALL refuse exactly as an unresolved field type is
  refused.
- If one union declares a member name twice, then the checker SHALL refuse
  `invalid_semantic_graph` from `DeclarationCause::DuplicateMember`, naming
  the member, located at the union declaration.
- The checker SHALL add each union member payload position to the shared
  declaration containment graph
  (`quire_semantic_value::declaration::TypeEnvironment`'s recursion check)
  as a named edge, on the same basis as a record field (QSpec FR-143
  "Recursion rule").
- If a containment cycle through a union is in the non-escaping or unnamed
  subgraph, then the checker SHALL refuse `ill_typed` from
  `DeclarationCause::Recursion`, naming the subgraph and the cycle, at every
  declaration on the cycle.
- The checked union type node SHALL be distinct from the enum type node. An
  enum and a union whose members are all nullary SHALL remain different
  declarations with different kernel types (ADR-012 §16.1).
- `SumCaseFamily::requirements()` SHALL return no claim for a union
  declaration (QSpec FR-146 discharges `case` exhaustiveness at admission;
  FR-057 claim-form table).
- The checker SHALL apply no limit to the number of members, the payload
  arity or the depth of nesting in payload types other than the checking
  ceilings a caller configures (NFR-011's node, preimage-byte and work
  ceilings). Reaching one SHALL return `StageFailure::Limit` naming the
  ceiling's kind, its configured bound and the `CheckingLimits` field that
  raises it.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-316-AC-1 | `union Shape { Empty, Circle(Integer), Rect(Integer, Integer) }` is admitted: the type environment holds one union composite keyed by its node key with members `Empty`, `Circle` and `Rect` in declared order and payload types `[]`, `[Integer]` and `[Integer, Integer]`; `enum Kind { Empty, Circle }` beside it keeps its own enum type, and the two are not the same type. | Test (TC-817) |
| FR-316-AC-2 | `union Tree { Leaf, Node(Integer, Option<Tree>, Option<Tree>) }` is admitted (QSpec TC-263). `union L { Nil, Cons(Integer, L) }` refuses `ill_typed` with recursion subgraph non-escaping and cycle `L, L`; the same union with `Option<L>` is admitted. A record `record Box { inner: U; }` and `union U { Wrap(Box) }` refuse `ill_typed` naming the cycle through both, at both declarations. | Test (TC-817, TC-818) |
| FR-316-AC-3 | `union U { A, A(Integer) }` refuses `invalid_semantic_graph` naming member `A`, located at the union declaration. `union U { A(Missing) }` refuses as an unresolved field type does, naming `Missing` at its span. | Test (TC-818) |
| FR-316-AC-4 | A union with 5,000 members and one whose payload type nests `Option<...>` 1,000 levels deep are admitted under the default checking ceilings. The same 5,000-member union checked with the work ceiling set one below the work units its admission charges returns `StageFailure::Limit` with kind work budget, the configured bound and the `CheckingLimits` field to raise; raising that field by one admits it. No outcome names a nesting-depth limit. | Test (TC-818) |

## Dependencies

- QSpec FR-143 ("Declarations and identity", "Recursion rule"),
  FR-143-AC-13.
- FR-091 (assembler), FR-062 (family contract), FR-057 (claim forms),
  NFR-011 and FR-096 (checking ceilings and their limit records).
- ADR-013 §8 OQ-I (SC-Q1 ruling (a)).

## References

- Owning ticket: QSL-383. Design: ADR-012 §16.2, §16.4, §16.5, §16.11.

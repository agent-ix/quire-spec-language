---
id: FR-062
title: "Implement the shared checked-family contract"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-005
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-065
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-063
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
---
# FR-062: Implement the shared checked-family contract

## Description

QSL semantic families (`Value`, `StateModel`, `SumCase`, `TemporalTrace`,
`ProtocolClause`, `Relation`) each own their own grammar, checked nodes and
diagnostics, but check, package, requirement-derivation and evaluation are
reached through the same shape for every family. QSL SHALL implement one
shared contract, with the parts below, that every family implements once and
that no family bypasses.

The contract's six parts (ADR-012 §2):

1. **Identity.** Every checked node the contract's `check` produces SHALL
   carry a stable identity minted once, at check time, from a
   content-addressed preimage over the node's structure, its owner when it
   has one, and its package-local qualified name when it is declared
   (ADR-013 O-04). A declared node has an owner; a builtin or anonymous type
   node has none. Nodes with identical structure, qualified name and
   owner SHALL share one identity. `check` SHALL key each source
   occurrence of a node separately from the node's identity, by (identity,
   role, ordinal).
2. **Provenance.** Every checked node occurrence SHALL map to its source span
   through a source map keyed by that occurrence key, minted only by QSL.
3. **Checked input (typing context).** A family's `check` SHALL receive a
   mutable typing context through which resolved declarations, the type
   environment and limits are read-only, and through which only a
   work-budget meter, a diagnostic sink and a scope stack are mutable.
   `check` SHALL read no global or thread-local state.
4. **Requirements.** A pure function of a checked item SHALL yield one
   `Requirements` value (its one capability kind, its declared extent, and
   any authored bound) for each claim the item carries, each paired with the
   checked site the claim covers: the item's own clause, or, for a `Value`
   function declaration, each scalar operation application in its body
   (FR-057's claim-form table). A claim form with no FR-057 kind SHALL
   yield no `Requirements` value.
5. **Structured outcome.** A family's `check` SHALL return the checked node,
   or a refusal carrying a family-owned typed cause that maps to a catalog
   code through one exhaustive function with no fallback arm; a `check` that
   reaches a limit SHALL return a limit outcome naming the exhausted limit
   kind (input bytes, nesting depth, node count or work budget), distinct
   from a refusal. A family's `evaluate` hook (stage S6a) SHALL be the only
   hook that returns `Incomplete` (a meter-budget outcome). `check` and
   `package` SHALL NOT return `Incomplete`. `check` and `package` SHALL
   instead return a `Limit` outcome when a limit is what they reached.
6. **Stage hooks.** A family SHALL implement one hook for each stage it
   participates in (check, package, requirements, and, for every family
   except `Relation`, evaluate); every hook SHALL take only checked input,
   and none SHALL take a CST, a token stream or a display string. A
   family's `package` hook SHALL be all-or-nothing for one item.
   `package` SHALL emit every v2 node an item requires when it returns
   success for that item.
   `package` SHALL emit no v2 node for an item when it returns the
   refusal for that item.
   `package` SHALL NOT emit a partial set of v2 nodes for one item.
   The layer-6 `replay` facade SHALL reach a family's checked node only
   through that family's `evaluate` hook, widened to accept a replay
   request; the facade SHALL select the node to evaluate by a typed
   identity (a `QualifiedName` resolved against the recompiled package's
   declarations,
   ADR-013 O-11), and SHALL NOT select it by a bare string compared against
   a display name.

A family that does not evaluate natively (`Relation`) SHALL have no
evaluation hook and SHALL NOT be an input to S6a evaluation: S6a's family
kind has no `Relation` variant (FR-090-AC-4). At a lowering or proof stage,
a family that sits out the stage SHALL have an explicit arm that returns
`unsupported` with a catalog code (ADR-012 §2).

## Inputs

- Parsed forms produced by a family's own grammar productions (ADR-012 §3).
- The mutable typing context (`CheckContext`): resolved declarations, the
  type environment, limits, a work-budget meter, a diagnostic sink, a scope
  stack.
- For evaluation: the checked node, an evaluation environment and a
  work-budget meter.

## Outputs

- A checked node carrying a minted identity and, through the package's
  source map, provenance to its source occurrence.
- A `Requirements` value for each claim a checked item carries, at the
  checked site the claim covers, and none for a claim form with no kind.
- The package's requirement records (`CheckedGraph::requirements`): one
  per claim site, keyed by the site's occurrence key (ADR-013 O-07), each
  holding the claim's `Requirements` and, for an operation application, its
  result bound and path condition.
- A structured outcome: the checked node, a typed refusal, a limit outcome
  naming the exhausted budget, or an internal fault distinct from both.
- For evaluation, on every family except `Relation`: an evaluated result or
  a typed refusal built only from checked input. `Relation` has no
  evaluation hook and is not an input to evaluation (FR-090-AC-4).

## Behavior

### The contract is one shape, not six ad hoc functions

The six parts above SHALL be expressed as one static contract (a fixed set of
associated types and methods; ADR-012 uses the design names `FamilyContract`
and `ReferenceEvaluation`) that every family implements exactly once. A
family's own `check`, `package`, `requirements` and `evaluate` code SHALL be
the only code that constructs that family's checked node or reads its
internals; the shared layer SHALL define no family-specific logic.

### No hook reads reconstructed meaning

A stage hook SHALL take its family's checked node, or a type built only from
checked nodes (for example a package emitter), as its only semantic input.
No stage hook SHALL take a CST node, a token, a display string, or a
diagnostic message, and derive semantic meaning from it.

### Identity is independent of position and counters

Two checked nodes with the same content, checked in the same package, SHALL
compare equal by identity, independent of where each occurs, what order they
were checked in, or how many other nodes exist. A source occurrence key
(identity, role, ordinal) SHALL distinguish two occurrences of one
structurally identical node without changing the node's identity.

### Typing context has no side door

`check` SHALL be given no way to read a resolved declaration, the type
environment or a limit except through the typing context parameter. `check`
SHALL be given no way to mutate anything except the meter, the diagnostic
sink and the scope stack the typing context exposes.

### Explicit limits bound every stage entry, including recursion

Every stage entry (`check`, and every other stage ADR-011 §2.3 names) SHALL
take explicit limits: input bytes, nesting depth, node count and work
budget, as that stage needs them. The checking stage's default limits are
finite and recorded with each checked result
([NFR-011](../non-functional/NFR-011-bound-value-checking-work.md)). A stage that is recursive (`check` is one;
ADR-011 §2.3 also names S1, S2 and S6a) SHALL bound its recursion depth by
one of these explicit limits, checked before each recursive step, and SHALL
NOT rely on the native call stack to bound recursion. A family's `check`
called on an arbitrarily deeply nested form (for example, nested function
application) SHALL refuse with a limit outcome naming the nesting-depth
limit once that limit is reached, rather than exhaust the native stack.

The expression-node budget a caller configures for checking a package
(`CheckingLimits::new(nodes, depth)`) bounds the package as a whole: the
expression nodes of every declaration in the package count against that one
budget. It is separate from the per-declaration stage-entry node-count limit
(`StageLimits::node_count`), whose exhaustion is the `Limit` outcome of
FR-062-AC-5. When the package's declarations together exceed the package
budget, package checking SHALL stop with a stage limit: the contract's
`check` returns `StageFailure::Limit` with kind node count, the caller's
configured limit as bound, the actual count and the locus of the node whose
entry failed ([FR-096](FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md)),
reported as `stage_limit_exceeded`/`node-count-exceeded`, including when
each declaration alone is within the budget. The `CheckingLimits` ceilings
are stage limits, not a caller work budget: the `quire.native.diagnostics/v1` `stage_limit_exceeded` row, revision `1-draft.6` covers a family `check`
and compiler stages S1 to S4, and says that a semantic maximum is not a
caller work budget, which `resource_exhausted` is.

### Packaging is all-or-nothing

A family's `package` hook, given one checked item, SHALL either emit every
v2 node ADR-012 §2 requires for that item and return success, or emit no v2
node for that item and return the refusal. A partial emission (some but not
all of an item's required v2 nodes, with no accompanying refusal) SHALL NOT
occur; a package reader that later finds a declaration node with no body
because emission stopped partway is evidence of a defect, not an admitted
outcome.

### Requirement records of a value function

**Scalar operation application.** A scalar operation application is an
application node that FR-093 lowers from a checked expression, whose
`operation.identity` is in one of these operation families:
`quire.op.integer`, `quire.op.rational`, `quire.op.decimal`,
`quire.op.ieee`, `quire.op.quantity`, `quire.op.numeric`, `quire.op.text`
and `quire.op.enum`, other than `quire.op.numeric.narrow`. Every other
application (`let`, `if`, a Boolean connective, `quire.op.boolean.eq` and
`.ne`, structural and reference equality, a projection, an option read, a
call, a dispatch, a collection or model operation) carries no claim of its
own.

**Claim sites.** A `Value` function declaration carries one `value-validity`
claim for each occurrence of a scalar operation application whose region
lies inside the function's body (FR-057's claim-form table). An occurrence
in the function's `decreases` measure is not in the body. The claim covers
one site. A claim site (design name `ClaimSite`; the implementing ticket
chooses the Rust spelling) holds:

- the checked application's `check::Location`: the region of the unit the
  checked expression was read from (FR-096);
- the application's **result bound**: the target range of the
  `quire.op.numeric.narrow` that wraps it, when a narrow's operand is this
  application, and otherwise the application's own result type;
- the application's **path condition**: the guards `check` walks the
  occurrence under, outermost first. A guard is an enclosing `if`'s
  condition, required true in the `then` branch and false in the
  `otherwise` branch, or the left operand of an enclosing `and`, `or` or
  `implies` whose right operand holds the occurrence, required true for
  `and` and `implies` and false for `or`. Each guard is recorded as its
  own node's `expression` occurrence key and the required outcome.

Two occurrences of one application node have two regions, so they are two
claim sites, whatever their extents, result bounds or path conditions.

**The claim.** The application is defined and its result lies in its
result bound, for every assignment of its extent roots under which its
path condition holds. `check` proves definedness and narrow ranges under
those same guards (FR-093's definedness walk, `check::facts`), so a body `check` admits never
yields a claim that fails on an assignment `check` excluded.

A narrow wraps exactly one expression. A narrow whose operand is not a
scalar operation application (a literal, a parameter or binder read, a
call, `sum`, `count`, `size`, a projection or a `value` read) yields no
claim: `check`'s `Coerce` range obligation discharges its range (FR-093).

**Records.** `check` SHALL record one requirement record for each claim
site:

- **Key.** The occurrence key (application node id, role `expression`,
  ordinal) that `check` records at the site's `Location` for the node
  lowered from the site's checked application (ADR-013 O-07, FR-093 "One
  node per checked expression"). `check` SHALL pair each site with the
  occurrence recorded at that site's own `Location`, never by the order in
  which sites or occurrences were produced. Ordinals follow lowering
  order (FR-093): a function's body is numbered before its `decreases`
  measure, and within each, in source order. So the records' bytewise key
  order, which is the request order (ADR-012 §13.5), is independent of
  check order.
- **Kind.** `value-validity`.
- **Extent.** `ClaimExtent` by ADR-014 §4's extent rule over the claim's
  extent roots (ADR-014 §4 "Operation application claims"). Each function
  parameter and each query, `count`, `sum`, `fold` or `reduce` binder read
  in the application's argument subtrees or in a guard of its path
  condition is a root, with its checked type, keyed by its parameter node.
  A `fold` binds two roots, its accumulator and its element. A binder bound
  inside the application's own argument subtrees is not a root of that
  application: in `size(filter(v in s: v > 0)) + 1`, the `>` is rooted at
  `v` and the outer `+` at `s` only.
  A read of a `let` binder contributes the roots its bound value reads, and
  a literal contributes none. A claim with no root is `Bounded`. The result
  bound is a finite range or the application's own result type, and adds
  no root.
- **Result bound and path condition**, as the site holds them, the result
  bound as its checked `ValueType` and the `WireNodeId` of its FR-092 type
  node.

Each record is computed from the checked tree at its own site, so two
occurrences of one node each carry their own extent, result bound and path
condition.

Every scalar operation application whose region lies in a function body
has at least one `expression` occurrence (FR-093). If `check` cannot pair
a claim site with an `expression` occurrence at its `Location`, including
an application that has only a `generated` occurrence, then `check` SHALL
return `KeyFault::UnkeyableRequirements` and SHALL produce no checked
package.

`check` SHALL classify each claim's extent while it checks the declaration,
under the declaration's stage limits, and `requirements()` reads the
result. If classification reaches its node-count ceiling, then `check`
SHALL return the declaration's `StageFailure::Limit` of kind node count,
located at the declaration (FR-096). If classification finds a composite
missing from the type environment, then `check` SHALL return an internal
fault (FR-097-AC-2).

**Fixtures.** Each unit below holds the functions shown; `v` is the
`quire.value.complete/v1` profile. A record is written as its
application's operation identity, then its extent, result bound and path
condition.

| ID | Unit | Requirement records |
| --- | --- | --- |
| RR-1 | `function neg using v(z: Int[0, 9]): Int[-9, 0] pure { -z }` | one: `integer.negate`, `Bounded`, `Int[-9, 0]`, no guard |
| RR-2 | `function inc using v(x: Int[0, 9]): Int[0, 10] pure { x + 1 }` | one: `integer.add`, `Bounded`, `Int[0, 10]` (the narrow that wraps it), no guard; none at the narrow |
| RR-3 | `function add using v(x: Int[0, 9], y: Int[0, 9]): Int[0, 18] pure { x + y }` | one: `integer.add`, `Bounded`, `Int[0, 18]`, no guard |
| RR-4 | `function eq using v(x: Int[0, 9], y: Int[0, 9]): Boolean pure { x = y }` | one: `integer.eq`, `Bounded`, `Boolean`, no guard |
| RR-5 | `function sq using v(x: Int[0, 9]): Integer pure { (x + 1) * (x + 1) }` | three: `integer.add` at ordinal 0 and at ordinal 1 of the one `+` node, and `integer.mul`; each `Bounded`, result bound `Integer`, no guard |
| RR-6 | `function big using v(n: Integer): Integer pure { n + 1 }` | one: `integer.add`, `Unbounded` with one `Integer` domain keyed by `n`'s parameter node and the empty path, `Integer`, no guard |
| RR-7 | `function lt using v(x: Int[0, 9]): Integer pure { let t = x + 1 in t * 2 }` | two: `integer.add` and `integer.mul`, each `Bounded` (the `*` through `t`'s bound value), result bound `Integer`, no guard |
| RR-8 | `function two using v(): Integer pure { 1 + 1 }` | one: `integer.add`, `Bounded`, `Integer`, no guard |
| RR-9 | `function both using v(b: Boolean, c: Boolean): Boolean pure { b and c }` | none |
| RR-10 | `function c2 using v(): Int[0, 9] pure { 3 }` | none: the narrow wraps a literal |
| RR-11 | `function clamp using v(n: Integer): Int[0, 10] pure { if n >= 0 and n <= 10 then n else 0 }` | two: `integer.ge`, `Unbounded` at `n`, `Boolean`, no guard; `integer.le`, `Unbounded` at `n`, `Boolean`, guard `n >= 0` true. None at the narrow over `n` |
| RR-12 | `function g using v(p: Int[0, 10]): Boolean pure { true }` and `function f using v(n: Integer): Boolean pure { if n >= 0 and n < 10 then g(n + 1) else true }` | three: `integer.ge`, result bound `Boolean`, no guard; `integer.lt`, result bound `Boolean`, guard `n >= 0` true; `integer.add`, result bound `Int[0, 10]`, guard `n >= 0 and n < 10` true. Each `Unbounded` with one `Integer` domain at `n` |
| RR-13 | `function q using v(x: Int[0, 9], y: Int[-9, 9]): Rational[-9, 9; 1, 9] pure { if y != 0 then x / y else rational(0, 1) }` | two: `integer.ne`, `Bounded`, `Boolean`, no guard; `rational.div`, `Bounded`, `Rational[-9, 9; 1, 9]`, guard `y != 0` true |
| RR-14 | `function all_positive using v(s: Sequence<Integer>[0, 5]): Boolean pure { forall(v in s: v + 1 > 0) }` | two: `integer.add`, result bound `Integer`, and `integer.gt`, result bound `Boolean`, each `Unbounded` with one `Integer` domain keyed by the binder `v`'s parameter node, no guard |
| RR-15 | `function sib using v(x: Int[0, 9], n: Integer): Integer pure { (let t = x + 1 in t * 2) + (let t = n + 1 in t * 2) }` | five, each with result bound `Integer` and no guard: `x + 1` `Bounded`; the first `t * 2` `Bounded` and the second `Unbounded` at `n`, each at its own occurrence; `n + 1` `Unbounded` at `n`; the outer `+` `Unbounded` at `n` |
| RR-16 | `function g using v(p: Int[0, 10]): Boolean pure { true }`, `function h using v(q: Int[0, 20]): Boolean pure { true }` and `function f using v(x: Int[0, 9]): Boolean pure { g(x + 1) and h(x + 1) }` | two, at ordinals 0 and 1 of the one `+` node: the first with result bound `Int[0, 10]` and no guard, the second with result bound `Int[0, 20]` and guard `g(x + 1)` true; each `Bounded` |
| RR-17 | `function m using v(x: Int[0, 9]): Integer pure decreases(x + 1) { x + 1 }` | one: `integer.add` at the body's occurrence, `Bounded`, `Integer`, no guard; none at the measure's |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-062-AC-1 | The contract exposes exactly the six parts (identity, provenance, checked input, requirements, structured outcome, stage hooks) as one set of associated types and methods that a family implements once. A family implementation that omits the checked-input parameter type on `check`, the `requirements` method, the `package` hook, or the `evaluate` hook (for a family other than `Relation`) fails to compile. Identity and provenance are structural properties of the `Checked` node type and the package's source map, not separate trait items a family can individually omit; they are instead enforced behaviorally by FR-062-AC-2. A correct-looking implementation that instead defines its own free-standing `check`/`package`/`requirements`/`evaluate` functions with no shared associated-type binding does not satisfy this criterion. | Test (TC-160) |
| FR-062-AC-2 | Given two parsed forms with identical structure checked into the same package, the checker mints one identity for both, and given the same node occurring twice in the source, the source map carries two distinct occurrence keys (identity, role, ordinal) for the one identity. Reordering the two source occurrences changes only their ordinal, never the identity. | Test (TC-160) |
| FR-062-AC-3 | A family's `check` compiles with no path to global or thread-local state, and a test that mutates only the typing context's meter, diagnostic sink and scope stack observes those mutations reflected in the returned outcome; a test that constructs two typing contexts from the same resolved declarations and checks the same form through each produces identical checked output, showing no hidden shared mutable state. | Test (TC-160) |
| FR-062-AC-4 | A claim form with no FR-057 capability kind yields no `Requirements` value from the pure requirements function, and each claim with a kind yields exactly one `Requirements` value naming that kind at its checked site: a function declaration whose body is `b and c` over Boolean parameters yields none, and one whose body is `x + y` over `Int[0, 9]` parameters yields exactly one, `value-validity`, at the `+` application. Calling the requirements function twice on the same checked item yields equal values. | Test (TC-160) |
| FR-062-AC-5 | A `check` that reaches a limit (input bytes, nesting depth, node count or work budget) returns a `Limit` outcome naming that limit kind, and a test asserts the returned value is not a refusal, not a checked node and not `Incomplete`; a family's `evaluate` hook that exhausts its meter budget returns `Incomplete`, and a test asserts neither `check` nor `package` ever returns `Incomplete` across the same fixture set. | Test (TC-160) |
| FR-062-AC-6 | The `Relation` family has no evaluation hook, and S6a's input type admits no `Relation` node, so a `Relation` node never reaches evaluation and yields neither a panic, a silently omitted call, nor a successful evaluated result (FR-090-AC-4). Every other family's evaluation hook, invoked on a checked node built only from checked input, returns without reading any CST, token or display string (verified by a test double that panics if such an input is touched). | Test (TC-160) |
| FR-062-AC-7 | Given a fixture nested to depth D (for example, function application nested D levels deep), checking it with the nesting-depth limit configured to D-1 returns a `Limit` outcome naming the nesting-depth limit. Checking the identical fixture with the limit configured to D, one greater and nothing else changed, does not return a nesting-depth `Limit` outcome. A test holds the fixture fixed and varies only the configured limit by exactly one, so the limit value, not the fixture's absolute size or the host's available stack, is shown to be the proximate cause of the refusal; this holds regardless of whether `check` walks the form by native recursion or by an explicit-stack iterative loop. | Test (TC-378) |
| FR-062-AC-8 | A family `Cause` enum's `catalog_code()` mapping contains no fallback arm; this is verified by FR-063's seam probe reporting `E0004` at that mapping under the `seam-probe` feature (S4), never by inspecting the source for the absence of a `_` arm. | Test (TC-161) |
| FR-062-AC-9 | Given a checked item requiring more than one v2 node, a fault injected partway through `package`'s emission (after the first node, before the last) yields no v2 bytes for that item and a refusal, never a package containing only the emitted-so-far nodes; a test that reads the v2 bytes after such a fault finds either a complete node set for the item or the item absent entirely, never a declaration node with no body. | Test (TC-160) |
| FR-062-AC-10 | The layer-6 `replay` facade's function-selection key, when it calls a family's widened `evaluate` hook, is a typed `QualifiedName`; a test that attempts to call the facade's entry point with a bare `&str` in place of a `QualifiedName` fails to compile, and a call with an unresolvable `QualifiedName` returns a typed refusal rather than falling back to a string comparison against a display name. | Test (TC-166) |
| FR-062-AC-11 | The package-wide `CheckingLimits` node budget is separate from the per-declaration `StageLimits::node_count` limit of FR-062-AC-5, and exceeding it is a `Limit` outcome with kind node count, reported as `stage_limit_exceeded`/`node-count-exceeded`. Given declarations `a() -> Integer = 1 + 1` and `b() -> Integer = 1 + 1`: package checking with `CheckingLimits::new(4, 128)` admits a package holding `a` alone; with `CheckingLimits::new(100, 128)` it admits a package holding both; with `CheckingLimits::new(4, 128)` it stops on the package holding both with `StageFailure::Limit` of kind node count, bound 4. | Test (TC-381) |
| FR-062-AC-12 | A family `check` that reaches one of its four stage-entry limits returns `StageFailure::Limit` naming the limit kind, the configured bound and the actual counter: the depth the refused entry would reach for nesting depth, the measured preimage byte length for input bytes, the measured expression-node count for node count, and the cumulative spend the denied charge would reach for work budget. Configured one below that counter, or at 0 for a declaration whose counter exceeds 1, `check` returns that same counter; configured at it, that limit does not stop `check`. With a work budget of exactly one declaration's charge `w`, the first check passes and the second returns counter `2w`. | Test (TC-432) |
| FR-062-AC-13 | `CheckedGraph::requirements` is the S3 stage output's requirement records (ADR-012 §13.5, ADR-011 E7), one per claim site, not dropped after `check`, and `qsl_package::CheckedPackage::graph().requirements()` reaches the same records from S4 (ADR-012 §2's package row). For each fixture RR-1 to RR-17 of this requirement's "Requirement records of a value function", the map holds exactly the records the fixture lists and no other: each `value-validity`, keyed by its application node's `expression` occurrence at its own site, with the listed extent, result bound and path condition. Where a fixture has two records at one node (RR-5, RR-15, RR-16), the keys differ only in ordinal, in source order, and each record's extent, result bound and guards are those of its own occurrence. Checking the same unit twice gives equal maps. ADR-012 §13.5's authored bound (#222) is not yet a `Requirements` member; #222 owns adding it. | Test (TC-160) |

## Dependencies

- [ADR-012](../decisions/ADR-012-semantic-family-extension-contracts.md) §2
  designs the six-part contract this requirement implements (including the
  `package` all-or-nothing rule and the replay stage's use of `evaluate`,
  §8), and §1 the closed `FamilyKind` catalogue whose members implement it.
- [FR-063](FR-063-exhaustive-family-extension-seam-probe.md) is this
  requirement's mechanism for demonstrating FR-062-AC-8; this requirement
  does not re-verify exhaustiveness by inspection.
- [ADR-013](../decisions/ADR-013-canonical-type-package-conversion-ownership.md)
  O-04 (checked node identity), O-07 (source occurrence identity), O-11
  (qualified names), O-12 (source locations and provenance) and O-16
  (outcomes) own the canonical types this contract's parts are built from;
  `quire-exact` (#213 S-1) implements them.
- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md) §2.1
  and §2.3 own the stage edges and the structured-result shape
  (`Result<Staged<T>, StageFailure<C>>`) the contract's outcomes are built
  from.
- [US-005](../usecase/US-005-trust-checked-identity-across-packaging.md).
- [FR-065](FR-065-migrate-function-application-to-checked-family.md) is the
  first family slice to implement this contract, for function declaration
  and application.

## Status

Specified under
[#214](https://github.com/agent-ix/quire-spec-language/issues/214). The
design names `FamilyContract` and `ReferenceEvaluation` follow ADR-012;
ADR-012 states these are design names and the implementing ticket chooses
the final Rust spelling within this requirement's rules.

**Scope of what #214 delivers (PR #262 review).** #214 implements the
contract narrowed to what its one migrated family (`Value`'s
function-declaration form) can back with a real, non-fabricated
construction site: `check`, the checked-input parameter, and the
stage-limit outcome shape. `requirements` (AC-1's mention, AC-4 entirely)
and `package` (AC-1's mention, AC-9 entirely) are deleted or never
implemented, not stubbed -- see `qsl-semantics/src/family/mod.rs`'s and
`qsl-semantics/src/family/contract.rs`'s own module docs for why each is a
real deferral rather than an oversight. The `Relation`
non-native-evaluability case (AC-6's first sentence) is a real, permanent
design fact rather than a deferral -- `Relation` never gets an evaluation
hook -- and QSL-152 found it already backed, just untagged for this
criterion (AC-6's own row below). By Acceptance Criterion, with real trace
tags as they exist in the delivered code today:
- FR-062-AC-1: unbacked. The delivered `FamilyContract` has one part,
  `check`, plus `ReferenceEvaluation::evaluate` -- not the six-part shape
  this criterion names. `requirements` and `package` are not stubbed onto
  the trait: `contract.rs`'s own doc records why an unconsumed `package`
  hook is the same fabricated-surface hazard PR #262 already found and
  deleted once, but that is a reason the gap is real, not a reason to stop
  counting it. `requirements` half: owned by QSL-140 (PR #435). `package`
  half: owned by QSL-242 (filed by QSL-152 to replace the QSL-16/QSL-143
  references PR #262 had pointed at, both of which explicitly disclaim the
  work).
- FR-062-AC-2: backed (`TC-160`, `qsl-semantics/src/check/family.rs`, `checking_tests`).
- FR-062-AC-3: backed (`TC-160`, QSL-161, QSL-246), all three clauses, in
  `qsl-semantics/src/check/family.rs`, `checking_tests`:
  - Clause 1 (no path to global or thread-local state):
    `check_stage_sources_have_no_global_or_thread_local_state` parses the
    non-test sources of `src/check` and `src/family` and fails on
    `static mut`, `thread_local!`, `lazy_static!`, and any once-cell, lazy,
    lock, atomic or `UnsafeCell` type. Immutable statics of plain types
    (`BOOLEAN`, `NO_GROUP`) are constants and pass. The scope is this
    crate's check stage; dependencies are not scanned.
  - Clause 2 (meter, sink, scope in the outcome):
    `a_mutated_meter_is_reflected_in_the_check_outcome` (pre-admitted
    charges show in the recorded admission count; a work bound one short
    gives a `WorkBudget` `Limit`),
    `a_mutated_diagnostic_sink_is_reflected_in_the_check_outcome` (seeded
    entry kept, the check's own appended) and
    `a_mutated_scope_stack_is_reflected_in_the_check_outcome` (the diagnostic
    is scoped to the check's frame over the caller's, and the caller's frame
    is restored on success and refusal).
  - Clause 3: `two_contexts_from_the_same_declarations_check_identically`
    compares the two independently constructed contexts' `Debug`-formatted
    checked output (`CheckedDeclaration` carries no `PartialEq`, so this
    repo's established substitute applies), with context `b` seeded with an
    extra signature ahead of the one the form calls and each context's own
    positional `function`/`callee` index normalized, so an extra
    declaration's position does not fail for a reason that is not a state
    leak.
- FR-062-AC-4: backed (`TC-160`, QSL-140, QSL-266).
  `FamilyContract::requirements` returns one claim per claim site
  (`Vec<Self::Claim>`; for `Value`, `check::ValueClaim`: the site and its
  extent keyed by binder until S3 names each root's parameter node).
  `tc_160_the_requirements_function_yields_one_claim_per_scalar_application`
  (`qsl-semantics/src/check/claims/tests.rs`) and
  `a_function_declaration_has_no_requirements`
  (`qsl-eval/src/value/expression/family.rs`).
- FR-062-AC-5: backed at the hook level (`TC-160`, `qsl-eval/src/value/expression/
  family.rs`): `quire_exact::Meter::charge`/`charge_plan` are `pub`
  (QSL-166), which QSL-153 uses as `ValueFunctionFamily::check`'s and
  `::evaluate`'s real call sites to tag the `Limit` half, implement the
  `Incomplete` half, and restore `StageLimits`' `input_bytes`/`node_count`
  fields plus a denied `CheckContext::meter` charge for the work-budget
  kind (`crate::family::contract::StageLimits`'s own doc names each
  field's real producer and consumer). This backs the criterion's first
  two clauses -- a `Limit` outcome naming the right kind, and `evaluate`
  returning `Incomplete` on an exhausted meter -- for the one family
  (`ValueFunctionFamily`) with a `check` hook in #214. The criterion's
  third clause -- a test asserting neither `check` nor `package` ever
  returns `Incomplete` across the same fixture set -- is untestable as
  written: `FamilyContract::package` has no implementation in #214 (see
  AC-9's own note on why it was deleted rather than wired up
  speculatively), so there is no `package` outcome to assert anything
  about. Unbacked. Owner: QSL-242.
- FR-062-AC-6: partly backed (QSL-152) -- the first sentence only. Stale
  as last written: it said "S6a has no family-kind dispatch yet," which
  QSL-148's `S6aFamilyKind` (`qsl-eval/src/value/expression/s6a.rs`) made no
  longer true. This criterion's first sentence is FR-090-AC-4 verbatim, so
  the tests that back FR-090-AC-4 back it: `s6a_family_kind_admits_no_
  relation_and_family_outcome_has_two_arms`
  (`qsl-eval/src/value/expression/mod.rs`, now also tagged `FR-062-AC-6`)
  and `both_family_outcome_arms_reach_a_caller_through_the_s6a_seam`
  (`qsl-eval/tests/it/model_reference_queries.rs`), plus `s6a.rs`'s own
  compile-time check that no `S6aFamilyKind` maps to `FamilyKind::Relation`.
  The second sentence (every other family's `evaluate` hook reads no CST,
  token or display string) is not backed: an earlier round of this fix
  tagged `evaluate_returns_incomplete_when_the_meter_is_exhausted`
  (`qsl-eval/src/value/expression/family.rs`) on the theory that
  `EvaluationEnv`'s own fields are checked-input shaped, but that test
  passes whatever the hook actually does with `env.package` --
  `CheckedPackage` exposes `function_identity(&str)` and function state
  carrying `slot_names: Vec<String>`, both string-shaped, so the hook is not
  in fact foreclosed from reading a display string through `env`, and
  nothing here asserts it does not. The tag is removed; the second sentence
  stays unbacked. Owner: QSL-246.
- FR-062-AC-7: backed by TC-378
  (`the_typer_depth_stop_is_located_at_the_node_whose_entry_failed`,
  `qsl-semantics/src/check/family.rs`, QSL-160). History: it was unbacked
  (PR #303 review, findings 4/5; reverted from an earlier "backed" claim). That earlier claim rested on
  `check::family::charge_recursive_nesting`, a side-walk added purely to
  charge `CheckContext`'s nesting counter once per expression-tree node --
  it re-walked the already-checked form afterward, charging nesting for
  nodes `Typer` had already finished checking, rather than charging nesting
  *at* real recursive descent as AC-7 requires. It also silently dropped
  the real walk's own source location and its early-return-on-first-error
  behavior (findings 4/5), so it was a regression as well as a
  mischaracterization; it has been deleted, not repaired, and
  `real_recursive_descent_is_nesting_depth_bounded` (the test that claimed
  to demonstrate it) is deleted with it.
  There are two distinct depth-limiting mechanisms in this codebase, and
  AC-7 is about the second one: `CheckContext::enter_nesting`
  (`qsl-semantics/src/family/`) is charged once per top-level declaration by
  `ValueFunctionFamily::check`, bounding how many declarations' worth of
  contract-level nesting are in flight -- it does not walk into a
  declaration's body. `Typer`'s own `CheckingLimits.depth`
  (`qsl-semantics/src/check/check.rs`) is charged once per real recursive `infer`/
  `check_as` call and is what actually bounds a function body's real
  recursive descent today; `real_checker_depth_limit_is_the_proximate_cause`
  (`qsl-semantics/src/check/family.rs` `checking_tests`, untagged) demonstrates that bound
  through `ValueFunctionFamily::check` end-to-end (a body nested 4 deep,
  limit 3 refuses via `CheckCause::ResourceExhausted{kind: Depth}`, limit 4
  admits). But that refusal surfaces as `StageFailure::Refused`, not the
  `StageFailure::Limit` outcome AC-7's wording names, and `Typer`'s depth
  counter is not `CheckContext`'s nesting-depth limit -- they are
  configured, charged and reported independently. Making AC-7 literally
  true would require threading `&mut CheckContext` through every recursive
  arm of `Typer::infer_form` (not just the `Call` arm this ticket touches),
  so real descent charges the *contract's* counter and reports through the
  contract's `Limit` outcome. That is real, load-bearing `Typer`
  entanglement -- the same entanglement QSL-148's own ticket asked to be
  reported rather than worked around -- and it is out of scope for this PR.
  Resolved by [FR-096](FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md)
  (QSL-160). AC-7's nesting-depth limit is `Typer`'s `CheckingLimits`
  depth, the bound real recursive descent charges. It is a stage limit
  (the `quire.native.diagnostics/v1` `stage_limit_exceeded` row, revision `1-draft.7`), so `ValueFunctionFamily::check` returns `Typer`'s depth refusal
  as `StageFailure::Limit` with kind nesting depth, carrying the
  `Locus::Region` of the node whose entry failed. No `CheckContext` is
  threaded through `Typer`. The `Locus` keeps the location PR #303
  required; it needs the unit's `RawSourceRef` (FR-001, ADR-013 §7 slice
  S-4b) and the forms' expression spans (FR-091-AC-10, QSL-141). TC-378
  backs AC-7. Owner: QSL-160.
- FR-062-AC-8: backed (`TC-161`, QSL-152). `check::refusal::CheckCause` has
  a `#[cfg(seam_probe)] __SeamProbe` variant; `CheckCause::code` (the one
  family `Cause` enum's `catalog_code()`-shaped mapping S4 names) has no
  arm for it, so `--cfg seam_probe` alone fails that match with `E0004` at
  the checked-in location `xtask::seam_probe::checked_in_locations` names
  (`qsl-semantics/src/check/refusal.rs`, `CheckCause::code`), the same
  shape `FamilyKind::catalog_code_prefix` already demonstrates for S1.
  `CheckCause::cause` carries a `#[cfg(seam_probe)]`-gated arm instead, so
  it keeps compiling under `--cfg seam_probe` alone. `code` also carries a
  `#[cfg(seam_probe_downstream)]`-gated arm, so it (and every other match
  over `CheckCause` outside this crate) keeps compiling when both cfgs are
  set together and downstream crates' own seams stay reachable, plus
  `#[deny(clippy::wildcard_enum_match_arm)]` and
  `#[deny(clippy::match_wildcard_for_single_variants)]`, so a future
  fallback arm is caught at normal compile time too, not only under the
  probe. Backed by the `make seam-probe` gate (part of `make ci`).
- FR-062-AC-9: unbacked. `FamilyContract::package` does not exist -- PR
  #262 review findings F1/F2 deleted it as a hook with one real caller
  (`CheckedPackage::emit_function_package_v2`, itself deleted under
  QSL-248/G2) that wrote into a scratch buffer it never read back, building
  its actual output independently through `family::emit_v2` instead; see
  `contract.rs`'s own doc on
  `FamilyContract` for the fuller reasoning. There is therefore no
  `package` emission to fault-inject partway through, but that is a reason
  the gap is real, not a reason to stop counting it. Owner: QSL-242 (filed
  by QSL-152; see AC-1's own row above).
- FR-062-AC-10: unbacked (untagged). `CheckedPackage::call`'s typed
  `QualifiedName` lookup is implemented (`qsl-eval/src/value/expression/mod.rs`),
  but no test carries this criterion's own trace tag. Owner: QSL-5 / #243.
- FR-062-AC-12: backed (`TC-432`, QSL-160 part 1):
  `nesting_depth_limit_is_the_proximate_cause`,
  `stage_limits_restored_kinds_refuse_one_below_the_real_metric` and
  `work_budget_kind_refuses_from_a_denied_meter_charge`
  (`qsl-semantics/src/check/family.rs`, `checking_tests`). The counter is
  not yet reported past `check`: package checking still reports these
  limits as `resource_exhausted` under catalog revision `1-draft.3`.
  Remaining work: QSL-236.
- FR-062-AC-11: partly backed (`TC-381`): the whole-package count passes;
  the `Limit` outcome is pending S-5b.
  `nodes_limit_is_enforced_across_the_whole_package_not_per_declaration`
  (`qsl-eval/tests/it/total_functions.rs`). The test observes the stop as
  `Refused{ResourceExhausted}`; its `StageFailure::Limit` outcome, amended
  here, is ADR-013 §7 slice S-5b's (QSL-160, FR-096).
- FR-062-AC-13: backed (`TC-160`, QSL-258, QSL-266):
  `qsl-semantics/src/check/claims/tests.rs` checks RR-1 to RR-17 (RR-15
  in both operand orders, RR-5 twice), guards, binder scope and `fold`,
  `reduce` and `flatMap` roots, and the
  keying faults over hand-built occurrence maps;
  `tests/it/request_builder.rs` reads RR-5's records through
  `CheckedPackage::graph()`.

Eight of this requirement's thirteen Acceptance Criteria are backed (AC-2,
AC-3, AC-4, AC-5, AC-7, AC-8, AC-12 and AC-13); two (AC-6, AC-11) are
partly backed, each for the specific clause named in its own row above.
AC-1, AC-9 and AC-10 are unbacked. AC-1's
`requirements` half is owned by QSL-140 the same way; its `package` half,
and AC-9 entirely, are owned by QSL-242, filed to replace the
QSL-16/QSL-143 references PR #262 had pointed at. AC-10 is unbacked
(untagged): its implementation exists, but no test carries the criterion's
own trace tag. Owner: QSL-5 / #243.

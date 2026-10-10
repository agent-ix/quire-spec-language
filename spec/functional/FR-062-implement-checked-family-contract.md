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
diagnostics, but check, requirement-derivation and evaluation are reached
through the same shape for every family, and every family's checked nodes
reach the checked package through the same S4 v2 emitter. QSL SHALL
implement one shared contract, with the parts below, that every family
implements once and that no family bypasses.

Declared names
carried on checked nodes are checked input. Copying a declared name into
`PreconditionFailure.selected`, and using field names as record keys, are
allowed. A "display string" is rendered text (`Display` or `Debug`
output, diagnostic text, source spelling); ADR-011 FB-01 forbids reading
one "to recover semantics".

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
   kind (input bytes, node count or work budget), distinct
   from a refusal. A family's `evaluate` hook (stage S6a) SHALL be the only
   hook that returns `Incomplete` (a meter-budget outcome). `check` SHALL
   NOT return `Incomplete`; it SHALL instead return a `Limit` outcome when a
   limit is what it reached. The S4 v2 emitter SHALL NOT return
   `Incomplete`.
6. **Stage hooks.** A family SHALL implement one hook for each stage it
   participates in: `check` and `requirements` at S3, and, for every family
   except `Relation`, `evaluate` at S6a. Every hook SHALL take only checked
   input, and none SHALL take a CST, a token stream or a display string.
   A family's packaging is its `check` lowering: `check` lowers each
   checked item to the nodes of the checked semantic graph (FR-093), and
   the layer-4 v2 emitter writes every family's nodes through one
   exhaustive arm per node tag (ADR-013 C-03).
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
family's own `check`, `requirements` and `evaluate` code SHALL be
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

### Explicit resource limits bound every stage entry, at any depth

Every stage entry (`check`, and every other stage ADR-011 §2.3 names) SHALL
take explicit resource limits: input bytes, node count and work budget, as
that stage needs them. The checking stage's default limits are finite and
recorded with each checked result
([NFR-011](../non-functional/NFR-011-bound-value-checking-work.md)). Every
walk a stage makes over its input SHALL run over an explicit heap stack whose
growth those limits charge, and SHALL NOT use native recursion that grows
with the input (ADR-030 D-1,
[FR-258](FR-258-check-and-lower-expressions-at-any-depth.md)). A family's
`check` called on a form of any nesting depth (for example, nested function
application) SHALL check it, or stop with a limit outcome naming the input
bytes, node count or work budget it reached.

The expression-node budget a caller configures for checking a package
(`CheckingLimits::new(nodes)`) bounds the package as a whole: the
expression nodes of every declaration in the package count against that one
budget. No per-declaration node count is compared before typing: the budget
is charged per node, at the node. When the package's declarations together exceed the package
budget, package checking SHALL stop with a stage limit: the contract's
`check` returns `StageFailure::Limit` with kind node count, the caller's
configured limit as bound, the actual count and the locus of the node whose
entry failed ([FR-096](FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md)),
reported as `stage_limit_exceeded`/`node-count-exceeded`, including when
each declaration alone is within the budget. The `CheckingLimits` ceilings
are stage limits, not a caller work budget: the `quire.native.diagnostics/v1` `stage_limit_exceeded` row covers a family `check`
and compiler stages S1 to S4, and says that a semantic maximum is not a
caller work budget, which `resource_exhausted` is.

### Packaging is all-or-nothing

The S4 v2 emitter (`qsl_package::emit_checked`) SHALL write a node only
when it writes every node that node names: its `dependencies` (FR-093
"Node dependencies"), its `semantic_type` and its body's type annotations.
When the emitter omits a node, it SHALL omit every node that names it,
transitively, each with the cause `NamesOmittedNode` naming the omitted
node it names, and SHALL write every node that names no omitted node. A
declaration node names the nodes of its body, so the emitted bytes SHALL
NOT hold a declaration node without a node its body names. An emission
that fails after it has read some nodes (an occurrence it cannot place, a
body it cannot encode) SHALL return its `EmitRefusal` and no bytes.

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

- the checked application's `quire_semantic_value::location::Location`: the region of the unit the
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
| FR-062-AC-1 | The contract exposes exactly the six parts (identity, provenance, checked input, requirements, structured outcome, stage hooks) as one set of associated types and methods that a family implements once. Its stage hooks are `check` and `requirements` (`FamilyContract`) and `evaluate` (`ReferenceEvaluation`). A family implementation that omits the checked-input parameter type on `check` or the `requirements` method fails to compile. `evaluate` is a member of `ReferenceEvaluation`, a trait crate-private to `qsl-eval`, so an external implementor cannot name or supply it and its omission is not a case outside `qsl-eval`; inside `qsl-eval` the compiler requires it of the one implementor (`ValueFunctionFamily`). Identity and provenance are structural properties of the `Checked` node type and the package's source map, not separate trait items a family can individually omit; they are instead enforced behaviorally by FR-062-AC-2. A correct-looking implementation that instead defines its own free-standing `check`/`requirements`/`evaluate` functions with no shared associated-type binding does not satisfy this criterion. | Test (TC-160) |
| FR-062-AC-2 | Given two parsed forms with identical structure checked into the same package, the checker mints one identity for both, and given the same node occurring twice in the source, the source map carries two distinct occurrence keys (identity, role, ordinal) for the one identity. Reordering the two source occurrences changes only their ordinal, never the identity. | Test (TC-160) |
| FR-062-AC-3 | A family's `check` compiles with no path to global or thread-local state, and a test that mutates only the typing context's meter, diagnostic sink and scope stack observes those mutations reflected in the returned outcome; a test that constructs two typing contexts from the same resolved declarations and checks the same form through each produces identical checked output, showing no hidden shared mutable state. | Test (TC-160) |
| FR-062-AC-4 | A claim form with no FR-057 capability kind yields no `Requirements` value from the pure requirements function, and each claim with a kind yields exactly one `Requirements` value naming that kind at its checked site: a function declaration whose body is `b and c` over Boolean parameters yields none, and one whose body is `x + y` over `Int[0, 9]` parameters yields exactly one, `value-validity`, at the `+` application. Calling the requirements function twice on the same checked item yields equal values. | Test (TC-160) |
| FR-062-AC-5 | A `check` that reaches a limit (input bytes, node count or work budget) returns a `Limit` outcome naming that limit kind, and a test asserts the returned value is not a refusal, not a checked node and not `Incomplete`; a family's `evaluate` hook that exhausts its meter budget returns `Incomplete`, and a test asserts that neither `check` nor the S4 v2 emitter (`qsl_package::emit_checked`) returns `Incomplete` across each stage's fixture set. | Test (TC-160) |
| FR-062-AC-6 | The `Relation` family has no evaluation hook, and S6a's input type admits no `Relation` node, so a `Relation` node never reaches evaluation and yields neither a panic, a silently omitted call, nor a successful evaluated result (FR-090-AC-4). Every other family's evaluation hook, invoked on a checked node built only from checked input, returns without reading any CST, token or display string. A display string is rendered text (`Display` or `Debug` output, diagnostic text, source spelling), not a declared name carried on a checked node; this is verified by a scan that the evaluator has no path to source text and calls nothing that renders text or reads a string-shaped accessor of the checked package, and by a test that renaming every declared name in a package leaves the evaluation outcome, loss count and metered work unchanged. | Test (TC-160) |
| FR-062-AC-7 | Given a fixture of N expression nodes nested N deep (for example, `not` applied N - 1 times to `true`), checking it with the `CheckingLimits` node limit configured to N - 1 returns a `Limit` outcome naming the node-count limit, bound N - 1, count N and setting `s3.nodes`. Checking the identical fixture with the limit configured to N, one greater and nothing else changed, checks it. A test holds the fixture fixed and varies only the configured limit by exactly one, so the limit value, not the fixture's depth or the host's available stack, is shown to be the proximate cause of the refusal. | Test (TC-378) |
| FR-062-AC-8 | A family `Cause` enum's `catalog_code()` mapping contains no fallback arm; this is verified by FR-063's seam probe reporting `E0004` at that mapping under the `seam-probe` feature (S4), never by inspecting the source for the absence of a `_` arm. | Test (TC-161) |
| FR-062-AC-9 | Given a checked package holding two functions, one of whose body names a node the emitter omits and one that names no omitted node: `emit_checked` omits the first function's declaration node and each node on its path to the omitted node, each with cause `NamesOmittedNode`; it writes the second function and its body; and QSL's I2 read of the bytes reads back Verified and exports the second function and not the first. Given the same package with an occurrence the region conversion cannot place, `emit_package` returns `EmitRefusal::UnlocatedOccurrence` and no bytes. | Test (TC-160) |
| FR-062-AC-10 | The layer-6 `replay` facade's function-selection key, when it calls a family's widened `evaluate` hook, is a typed `QualifiedName`; a test that attempts to call the facade's entry point with a bare `&str` in place of a `QualifiedName` fails to compile, and a call with an unresolvable `QualifiedName` returns a typed refusal rather than falling back to a string comparison against a display name. | Test (TC-166) |
| FR-062-AC-11 | The package-wide `CheckingLimits` node budget counts the nodes of every declaration together, and exceeding it is a `Limit` outcome with kind node count, reported as `stage_limit_exceeded`/`node-count-exceeded`. Given declarations `a() -> Integer = 1 + 1` and `b() -> Integer = 1 + 1`: package checking with `CheckingLimits::new(4)` admits a package holding `a` alone; with `CheckingLimits::new(100)` it admits a package holding both; with `CheckingLimits::new(4)` it stops on the package holding both with `StageFailure::Limit` of kind node count, bound 4. | Test (TC-381) |
| FR-062-AC-12 | A family `check` that reaches one of its two stage-entry limits returns `StageFailure::Limit` naming the limit kind, the configured bound and the actual counter: the measured preimage byte length for input bytes and the cumulative spend the denied charge would reach for work budget. Configured one below that counter, or at 0 for a declaration whose counter exceeds 1, `check` returns that same counter; configured at it, that limit does not stop `check`. A family `check` compares no measured node count before typing: its node limit is `CheckingLimits::nodes`, charged at the node (FR-062-AC-7). With a work budget of exactly one declaration's charge `w`, the first check passes and the second returns counter `2w`. | Test (TC-432) |
| FR-062-AC-13 | `CheckedGraph::requirements` is the S3 stage output's requirement records (ADR-012 §13.5, ADR-011 E7), one per claim site, not dropped after `check`, and `qsl_package::CheckedPackage::graph().requirements()` reaches the same records from S4 (ADR-012 §2's package row). For each fixture RR-1 to RR-17 of this requirement's "Requirement records of a value function", the map holds exactly the records the fixture lists and no other: each `value-validity`, keyed by its application node's `expression` occurrence at its own site, with the listed extent, result bound and path condition. Where a fixture has two records at one node (RR-5, RR-15, RR-16), the keys differ only in ordinal, in source order, and each record's extent, result bound and guards are those of its own occurrence. Checking the same unit twice gives equal maps. ADR-012 §13.5's authored bound (#222) is not yet a `Requirements` member; #222 owns adding it. | Test (TC-160) |

The state scan for FR-062-AC-3 covers the non-test items of every `.rs` file
under `qsl-semantics/src/check` and `qsl-semantics/src/family`
(files with `tests` in the name and `#[cfg(test)]` items are
skipped). Callees outside them (`quire-exact`, `qsl-forms`,
`qsl-foundation` and the rest of `qsl-semantics`) are not scanned.

The evaluator display-string scan for FR-062-AC-6 reads
`qsl-eval/src/value/expression/evaluate.rs` and the `evaluate` method in
`qsl-eval/src/value/expression/family.rs`
only; `causes.rs`, `mod.rs`, `s6a.rs` and callees in other crates are
not scanned.

## Dependencies

- [ADR-012](../decisions/ADR-012-semantic-family-extension-contracts.md) §2
  designs the six-part contract this requirement implements (including the
  S4 emitter's all-or-nothing rule, "Packaging", and the replay stage's use
  of `evaluate`, §8), and §1 the closed `FamilyKind` catalogue whose members
  implement it.
- [FR-093](FR-093-lower-checked-value-expressions-to-fr-322-terms.md) is
  where a family's checked nodes are lowered to the semantic graph the S4
  emitter writes (FR-093-CON-2: the layer-4 `package` crate builds no body
  term and mints no key).
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

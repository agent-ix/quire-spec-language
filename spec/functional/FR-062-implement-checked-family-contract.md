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
   content-addressed preimage over the node's structure and its declaring
   package's `name@version`. Structurally identical nodes SHALL share one
   identity, and `check` SHALL key each source occurrence of a node
   separately from the node's identity, by (identity, role, ordinal).
2. **Provenance.** Every checked node occurrence SHALL map to its source span
   through a source map keyed by that occurrence key, minted only by QSL.
3. **Checked input (typing context).** A family's `check` SHALL receive a
   mutable typing context through which resolved declarations, the type
   environment and limits are read-only, and through which only a
   work-budget meter, a diagnostic sink and a scope stack are mutable.
   `check` SHALL read no global or thread-local state.
4. **Requirements.** A pure function of a checked node SHALL yield the node's
   `Requirements` (its one capability kind, its declared extent, and any
   authored bound) or, for a claim form with no FR-057 kind, SHALL yield no
   `Requirements` value.
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

A family that does not evaluate natively (`Relation`) SHALL return a named
refusal at the evaluation hook rather than omitting the hook silently, so
every family has an entry at every stage its contract lists, distinguishing
"this family sits out this stage" from "this family has no hook."

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
- A `Requirements` value, or none, for a checked node's own claim form.
- A structured outcome: the checked node, a typed refusal, a limit outcome
  naming the exhausted budget, or an internal fault distinct from both.
- For evaluation, on every family except `Relation`: an evaluated result or
  a typed refusal built only from checked input. On `Relation`: a named
  refusal stating that the family is not natively evaluable.

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
budget, as that stage needs them. A stage that is recursive (`check` is one;
ADR-011 §2.3 also names S1, S2 and S6a) SHALL bound its recursion depth by
one of these explicit limits, checked before each recursive step, and SHALL
NOT rely on the native call stack to bound recursion. A family's `check`
called on an arbitrarily deeply nested form (for example, nested function
application) SHALL refuse with a limit outcome naming the nesting-depth
limit once that limit is reached, rather than exhaust the native stack.

The expression-node limit a caller configures for checking a package
(`CheckingLimits::new(nodes, depth)`) bounds the package as a whole: the
expression nodes of every declaration in the package count against that one
budget. When the package's declarations together exceed it, package checking
SHALL refuse with `resource_exhausted`, naming the typing stage, the node
limit kind and the caller's configured limit value
(`ResourceExhausted{Typing, Nodes, <configured limit>}`), including when
each declaration alone is within the limit.

### Packaging is all-or-nothing

A family's `package` hook, given one checked item, SHALL either emit every
v2 node ADR-012 §2 requires for that item and return success, or emit no v2
node for that item and return the refusal. A partial emission (some but not
all of an item's required v2 nodes, with no accompanying refusal) SHALL NOT
occur; a package reader that later finds a declaration node with no body
because emission stopped partway is evidence of a defect, not an admitted
outcome.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-062-AC-1 | The contract exposes exactly the six parts (identity, provenance, checked input, requirements, structured outcome, stage hooks) as one set of associated types and methods that a family implements once. A family implementation that omits the checked-input parameter type on `check`, the `requirements` method, the `package` hook, or the `evaluate` hook (for a family other than `Relation`) fails to compile. Identity and provenance are structural properties of the `Checked` node type and the package's source map, not separate trait items a family can individually omit; they are instead enforced behaviorally by FR-062-AC-2. A correct-looking implementation that instead defines its own free-standing `check`/`package`/`requirements`/`evaluate` functions with no shared associated-type binding does not satisfy this criterion. | Test (TC-160) |
| FR-062-AC-2 | Given two parsed forms with identical structure checked into the same package, the checker mints one identity for both, and given the same node occurring twice in the source, the source map carries two distinct occurrence keys (identity, role, ordinal) for the one identity. Reordering the two source occurrences changes only their ordinal, never the identity. | Test (TC-160) |
| FR-062-AC-3 | A family's `check` compiles with no path to global or thread-local state, and a test that mutates only the typing context's meter, diagnostic sink and scope stack observes those mutations reflected in the returned outcome; a test that constructs two typing contexts from the same resolved declarations and checks the same form through each produces identical checked output, showing no hidden shared mutable state. | Test (TC-160) |
| FR-062-AC-4 | A claim form with no FR-057 capability kind yields no `Requirements` value from the pure requirements function, and a claim form with a kind yields exactly one `Requirements` value naming that kind; calling the requirements function twice on the same checked node yields equal values. | Test (TC-160) |
| FR-062-AC-5 | A `check` that reaches a limit (input bytes, nesting depth, node count or work budget) returns a `Limit` outcome naming that limit kind, and a test asserts the returned value is not a refusal, not a checked node and not `Incomplete`; a family's `evaluate` hook that exhausts its meter budget returns `Incomplete`, and a test asserts neither `check` nor `package` ever returns `Incomplete` across the same fixture set. | Test (TC-160) |
| FR-062-AC-6 | The `Relation` family's evaluation hook, invoked on a checked `Relation` node, returns a named refusal stating non-native evaluability rather than a panic, a silently omitted call, or a successful evaluated result. Every other family's evaluation hook, invoked on a checked node built only from checked input, returns without reading any CST, token or display string (verified by a test double that panics if such an input is touched). | Test (TC-160) |
| FR-062-AC-7 | Given a fixture nested to depth D (for example, function application nested D levels deep), checking it with the nesting-depth limit configured to D-1 returns a `Limit` outcome naming the nesting-depth limit. Checking the identical fixture with the limit configured to D, one greater and nothing else changed, does not return a nesting-depth `Limit` outcome. A test holds the fixture fixed and varies only the configured limit by exactly one, so the limit value, not the fixture's absolute size or the host's available stack, is shown to be the proximate cause of the refusal; this holds regardless of whether `check` walks the form by native recursion or by an explicit-stack iterative loop. | Test (TC-160) |
| FR-062-AC-8 | A family `Cause` enum's `catalog_code()` mapping contains no fallback arm; this is verified by FR-063's seam probe reporting `E0004` at that mapping under the `seam-probe` feature (S4), never by inspecting the source for the absence of a `_` arm. | Test (TC-161) |
| FR-062-AC-9 | Given a checked item requiring more than one v2 node, a fault injected partway through `package`'s emission (after the first node, before the last) yields no v2 bytes for that item and a refusal, never a package containing only the emitted-so-far nodes; a test that reads the v2 bytes after such a fault finds either a complete node set for the item or the item absent entirely, never a declaration node with no body. | Test (TC-160) |
| FR-062-AC-10 | The layer-6 `replay` facade's function-selection key, when it calls a family's widened `evaluate` hook, is a typed `QualifiedName`; a test that attempts to call the facade's entry point with a bare `&str` in place of a `QualifiedName` fails to compile, and a call with an unresolvable `QualifiedName` returns a typed refusal rather than falling back to a string comparison against a display name. | Test (TC-166) |
| FR-062-AC-11 | Given declarations `a() -> Integer = 1 + 1` and `b() -> Integer = 1 + 1`: package checking with `CheckingLimits::new(4, 128)` admits a package holding `a` alone; with `CheckingLimits::new(100, 128)` it admits a package holding both; with `CheckingLimits::new(4, 128)` it refuses the package holding both with `ResourceExhausted{stage: Typing, kind: Nodes, limit: 4}`. | Test (TC-381) |

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
stage-limit outcome shape. `requirements` (AC-1's mention, AC-4 entirely),
`package` (AC-1's mention, AC-9 entirely) and the `Relation`
non-native-evaluability case (AC-6's second sentence) are deleted or never
implemented, not stubbed -- see `src/family/mod.rs`'s and
`src/family/contract.rs`'s own module docs for why each is a real deferral
rather than an oversight. By Acceptance Criterion, with real trace tags as
they exist in the delivered code today:
- FR-062-AC-1: unbacked (untagged; the six-part shape it describes is
  narrowed to `check` alone, per `contract.rs`'s own doc). Owner: QSL-152.
- FR-062-AC-2: backed (`TC-160`, `src/value/expression/family.rs`).
- FR-062-AC-3: unbacked (untagged; PR #262 review, coordinator round 3,
  finding 5). The one test tagged for this criterion,
  `two_contexts_from_the_same_declarations_check_identically`, backs only
  its "no hidden shared mutable state" half: each of two independently
  constructed contexts observes exactly one diagnostic. F6 deleted the
  test's `staged_a.value == staged_b.value` self-comparison, which is the
  only thing that ever stood in for this criterion's central clause -- two
  typing contexts checking the same declarations produce identical checked
  output -- and the tag survived that deletion until this round untagged
  it. Nothing currently asserts the identical-checked-output clause itself.
  Owner: QSL-161.
- FR-062-AC-4: unbacked. `requirements` is not implemented; no family #214
  migrates carries an FR-057 capability kind (`src/family/mod.rs`'s module
  doc). Owner: QSL-152.
- FR-062-AC-5: backed at the hook level (`TC-160`, `src/value/expression/
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
  about. Owner: QSL-152 restores that clause once `package` exists.
- FR-062-AC-6: unbacked. `Relation` has no `FamilyContract` implementation
  in #214; there is nothing to invoke this criterion's hook against yet.
  Owner: QSL-152.
- FR-062-AC-7: **unbacked** (PR #303 review, findings 4/5; reverted from an
  earlier "backed" claim in this round). That earlier claim rested on
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
  (`src/family/`) is charged once per top-level declaration by
  `ValueFunctionFamily::check`, bounding how many declarations' worth of
  contract-level nesting are in flight -- it does not walk into a
  declaration's body. `Typer`'s own `CheckingLimits.depth`
  (`src/check/check.rs`) is charged once per real recursive `infer`/
  `check_as` call and is what actually bounds a function body's real
  recursive descent today; `real_checker_depth_limit_is_the_proximate_cause`
  (`src/value/expression/family.rs`, untagged) demonstrates that bound
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
  Reported to the requirement owner as an open scoping question, not
  decided here. Owner: QSL-148 follow-up (untracked as of this report).
- FR-062-AC-8: unbacked. FR-063's seam probe covers S1 only in #214 (its
  own Status/scope note); S4 (a family `Cause` enum's `catalog_code()`) has
  no cause-bearing family to probe yet. Owner: QSL-152.
- FR-062-AC-9: unbacked, by PR #262 review findings F1/F2.
  `FamilyContract::package` had one real caller
  (`CheckedPackage::emit_function_package_v2`), which wrote `package`'s
  output into a scratch buffer it never read back before building its
  actual returned bytes independently through `family::emit_v2` -- a hook
  nothing consumed. It is deleted rather than wired up speculatively or
  kept as an unbacked "package" row while claiming AC-9's fault-injection
  behavior; see `contract.rs`'s own doc on `FamilyContract`. Owner: QSL-152.
- FR-062-AC-10: unbacked (untagged). `CheckedPackage::call`'s typed
  `QualifiedName` lookup is implemented (`src/value/expression/mod.rs`),
  but no test carries this criterion's own trace tag. Owner: QSL-5 / #243.
- FR-062-AC-11: backed (`TC-381`):
  `nodes_limit_is_enforced_across_the_whole_package_not_per_declaration`
  (`tests/it/total_functions.rs`).

Three of this requirement's eleven Acceptance Criteria are backed (AC-2,
AC-5, AC-11). AC-5's two tagged tests are
`stage_limits_restored_kinds_refuse_one_below_the_real_metric` and
`evaluate_returns_incomplete_when_the_meter_is_exhausted`
(`src/value/expression/family.rs`; PR #303 review round 3, finding F3).
The other eight are unbacked, for the
reasons above -- not silently. AC-7 in particular stays unbacked pending a
scoping decision on the `Typer` entanglement described in its row above.

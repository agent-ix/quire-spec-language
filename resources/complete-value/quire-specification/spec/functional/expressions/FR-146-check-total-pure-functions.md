---
id: FR-146
title: "Check total pure functions and recursion"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-010
    type: implements
  - target: ix://agent-ix/quire-specification/FR-033
    type: references
  - target: ix://agent-ix/quire-specification/AD-005
    type: references
  - target: ix://agent-ix/quire-specification/FR-044
    type: references
  - target: ix://agent-ix/quire-specification/FR-145
    type: references
---

# FR-146: Check total pure functions and recursion

## Description

When admitting a function declaration, the checker SHALL establish typed
purity, total definedness and a well-founded termination argument for every
reachable call cycle.

## Inputs

Function declarations, parameter/result types, call graph, effects declaration,
termination measure and selected type/model closure.

## Outputs

Checked callable identities or located type, effect, definedness or termination
refusals.

## Behavior

Functions may return any complete-V1 value type. Calls bind exact declaration
identity, arity and ordered arguments. Recursion uses a declared well-founded
measure proven to decrease on every cycle. Reflection, mutation, I/O and ambient
lookups are prohibited. Runtime depth/fuel exhaustion remains incomplete.

## Termination and totality contract

The checker constructs a declaration-identity call graph and checks each
strongly connected component. An acyclic component needs no termination
measure. Every recursive component declares one shared lexicographically
ordered tuple whose elements are nonnegative mathematical integers, finite
collection cardinalities or strict-subvalue depths. Each recursive edge must
prove that the substituted callee tuple is lexicographically smaller than the
caller's tuple; this rule also applies across mutual recursion.

Totality is path-sensitive: every conditional path must return the declared
result, and each partial operation must have a proved precondition. The
Complete-V1 expression grammar has no pattern-matching or destructuring form,
and every `if` has an `else`, so exhaustiveness is structural. Purity and typing are checked before
reachability, so an unreachable impure or ill-typed branch is still rejected.
Evaluation fuel is an execution resource rather than a semantic termination
argument: exhausting it returns `incomplete`, never a value or a proof of
nontermination.

## Calls, measures and definedness facts

Functions are first order. A function is not a value, and a call names a
`function` declaration or FR-307 export by its exact identity, so the call graph
is static. The graph includes calls inside `let`, `if`, collection binders, fold
steps and measure expressions, and, under `quire.model.complete/v1`, the FR-151
dispatch edges from the declaration containing a dispatched call to every
candidate `operation-body` and every clause of every candidate's effective
precondition. Operation bodies and contract clauses have no `decreases` form, so
a strongly connected component containing a dispatch edge is refused at link
time as `refused { code: invalid_package, cause: definition-cycle }`. A call to a reserved `quire::value::...`
intrinsic is not a call-graph edge. A function declaration is
`function f using V (p: T, ...): R pure [decreases(m)] { body }`, where `V` is
the declared value-profile alias.

Only a function in a recursive strongly connected component of the call graph,
including a function that calls itself, needs `decreases`. A non-recursive
function needs no measure. A measure supplied on a non-recursive function is
still type checked and imposes no obligation.

`decreases(m)` supplies the measure. `m` is either one measure element or a call
of a declared `tuple` whose arguments are measure elements, and a one-element
measure is a one-tuple. A measure element is exactly one of: an *integer
element*, a parameter of `Integer` or `Int[..]` type; a *cardinality element*,
`size(p)` for a collection parameter `p`; or a *structural element*, a parameter
of a recursive record type ordered by strict containment. Any other expression
in a measure is a `measure-kind` failure.

Every function in one strongly connected component declares a measure. The
checker checks each component's obligations in this order and reports the first
failure: `missing-measure` (a function omits `decreases`); `measure-arity`
(arities differ); `measure-kind` (a position is not a measure element or the
element kinds at one position differ); `nonnegative` (an integer element's
parameter type has a lower bound below zero, so `Integer` never qualifies);
then `decrease` for each recursive call edge. For an edge, let `a` be the call
argument bound to the callee's element parameter at position `i`, and `p` the
caller's element parameter at position `i`. Position `i` is *equal* when `a` is
exactly `p`. It is *strictly smaller* when:

- for an integer element, `a` is `p - k` for an integer literal `k >= 1`;
- for a structural element, `a` is a nonempty projection path from `p`, whose
  each step is a field projection `.f`, or `value(q)` of an `Option` path `q`
  whose `present(q)` is a guard fact at that call site; for example
  `value(n.next)` under `present(n.next)` is a strict subvalue of `n`;
- for a cardinality element, never, because Complete V1 has no source form
  that proves a strictly smaller collection.

The edge decreases when some position is strictly smaller and every earlier
position is equal. The call argument `p - k` must still meet its parameter type
under the ordinary range obligation below. A failure is
`refused { code: undefined_expression, cause: unproved-decrease }`, naming the
component cycle, the call edge and the failed obligation. `undefined_expression`
is the FR-271 code for unproved definedness, and `unproved-decrease` is its
catalogued cause for termination.

## Accepted proof forms

Definedness and range proofs use exactly the
[state semantics](../../../proposals/state-core/state-semantics.md) section 4
guard facts and section 6 interval discharge, closed as follows. A producer
admits nothing that these rules do not prove and refuses nothing that they do.

A *stable path* is a parameter or `let` name, or `self` inside a
`quire.model.complete/v1` invariant, precondition, postcondition or
`operation-body` block, followed by zero or more field projections and proved
`value(q)` steps. When discharging an FR-151 refinement obligation, a projection
`self.f` onto a redefining field or the field it redefines has the declared
facts of the redefined parent member being refined, not the redefining type. The facts are:

1. **Declared facts.** Each integer stable path has its declared type's
   interval, and `size(c)` for a stable collection path `c` has the declared
   cardinality bound `[min, max]`.
2. **Presence facts.** `present(q)` for a stable path `q` yields that fact on
   its true outcome and its negation on its false outcome.
3. **Comparison facts.** `x op k` or `k op x`, where `x` is an integer stable
   path or `size(c)`, `k` is an integer literal (optionally negated) and `op` is
   `=`, `!=`, `<`, `<=`, `>` or `>=`, intersects `x`'s interval on each outcome.
   `!=` removes `k` only when `k` is an endpoint of the interval, and `x != 0`
   also records a nonzero fact for `x`. A comparison of two non-literal
   operands yields no fact.
4. **Propagation.** Facts flow into the `then` and `else` branches of `if`,
   the right operand of `and`, `or` and `implies` by the state-semantics rules,
   through `not` by exchanging outcomes, and through `let` aliases. At a join
   only facts valid on every alternative remain.

The interval of an integer expression is computed over mathematical integers:
a literal is its point; a stable path is its refined interval; `+`, `-`, `*`
and unary `-` are exact interval arithmetic on their operand intervals; and
every other integer form, such as a call, division, `count`, `sum` or `fold`,
has its declared result type's interval. No other reasoning is admitted.

A `Rational[..]` expression has a numerator interval `N` and a positive
denominator interval `D`, both over mathematical integers, and never a value
interval. A literal `rational(n,d)` has the points of its normalized `n/d`; a
stable path has its declared `[n1,n2]` and `[d1,d2]`; an `Integer` or `Int[..]`
operand of a rational `/` (`quire.op.rational.div`, which promotes it
exactly to `n/1` under FR-149) has its integer interval over `[1,1]`; and every other
rational form, such as a call, `sum`, `fold` or `convert`, has its declared
result type's intervals. When every operand of `+`, `-`, `*`, `/` or unary `-`
has point intervals and the divisor point is nonzero, the result has the points
of its exact normalized value. Otherwise, for operands `a/b` in (`Na`, `Da`) and
`c/d` in (`Nc`, `Dc`), the raw intervals are exact interval arithmetic:
`Na×Dc + Nc×Da` or `Na×Dc - Nc×Da` over `Da×Dc` for `+` and `-`; `Na×Nc` over
`Da×Dc` for `*`; `-Na` over `Da` for unary `-`; and for `/` the numerator
`Na×Dc` when `Nc` is positive, `-(Na×Dc)` when `Nc` is negative and their hull
otherwise, over `Da×M`, where `M` is the magnitude interval of `Nc` with zero
removed exactly: `[min|Nc|, max|Nc|]` when `Nc` excludes zero and
`[1, max|Nc|]` when it contains zero. The normalized result intervals follow
from the raw `[l,h]` over `[p,q]`: the numerator is `[1,h]` when `l > 0`,
`[l,-1]` when `h < 0` and `[l,h]` otherwise, and the denominator is `[1,1]` when
`l = h = 0` and `[1,q]` otherwise. A `/` whose divisor interval `Nc` contains
zero first requires the nonzero obligation below; without it the quotient is
`refused { code: undefined_expression, cause: unproved-nonzero }` and has no
interval, never an infinite, clamped or approximated one. A rational range
obligation holds when the normalized numerator interval lies inside `[n1,n2]`
and the denominator interval inside `[d1,d2]` of the required type; it is the
obligation of an FR-149 rational narrowing (`quire.op.rational.narrow`), as the
range obligation is of a range narrowing (`quire.op.numeric.narrow`), and
neither narrowing ever rounds. A
comparison `x != rational(0,1)` or `rational(0,1) != x` for a rational stable
path `x` records a nonzero fact for `x` on its true outcome, and `=` on its
false outcome; no other rational comparison yields a fact.

A `Decimal[..]` divisor satisfies the nonzero obligation only as a
`decimal(c,s)` literal with `c != 0`. A quantity divisor has no admitted
nonzero proof form: every FR-142 quantity division in a linked body is
`refused { code: undefined_expression, cause: unproved-nonzero }` at the
division, whatever guard encloses it, and only direct kernel evaluation decides
a zero quantity divisor, as the FR-142 undefined outcome.

The obligations are discharged as follows. A range obligation holds when the
expression's interval lies inside the required type's interval. A nonzero
obligation holds when the operand's interval excludes zero or the operand is a
stable path with a nonzero fact. A presence obligation for `value(q)` holds when
`q` is a stable path with a `present(q)` fact; a compound `q` must first be
bound with `let`. A `reduce` obligation holds when the operand's `size`
interval, from its declared bound or a comparison fact on its stable path, has
minimum at least one.

The partial operations are closed, each with its `undefined_expression` cause:
a divisor that may be zero in FR-140, FR-142, FR-044 or FR-147
(`unproved-nonzero`); a zero base with a negative integer exponent, whose base
carries the nonzero obligation (`unproved-nonzero`); `value(e)` without a proved
`present(e)` (`unproved-presence`); `reduce` without a proved `size(c) >= 1`
under FR-145 (`unproved-range`); an FR-149 range or rational narrowing whose
converted value is not proved a member of its target type (`unproved-range`).
Each obligation applies on a path
that can execute, and an unproved one is
`refused { code: undefined_expression, cause: ... }` at that operation when the
body is linked.

An FR-148 IEEE-to-`Rational[..]` conversion carries no body obligation. The
FR-044 exception exempts it from static domain proof, and FR-148 decides it at
evaluation: a NaN or infinite operand is undefined after `ieee.operands`, and a
finite non-member is `refused { code: ieee_rational_out_of_domain }`. Both are
evaluation outcomes that propagate; neither refuses linking.

`deref(r)` is not a partial operation of a function body. It requires `r` to
have type `Reference<T>`; an optional reference is first unwrapped with
`value`, whose presence obligation applies. Target existence is a runtime
snapshot-closure input requirement that linking records, as the state semantics
states, and a target missing from a declared-complete universe is
`refused { code: dangling_reference, cause: absent-target-in-complete-population }`
during input validation.

Runtime refusals named by FR-140, FR-142, FR-144 and FR-148 (membership, strict
`exact`, cardinality bound, NaN payload and rational domain) are typed
outcomes. They propagate and do not make a function partial. FR-044 still owns
the static integer and rational result-bound obligations.

Purity, name resolution and typing are checked first, at the offending
expression, even in an unreachable branch. Complete V1 has no source form for
I/O, mutation, reflection or ambient lookup. In a function body, a qualified
call whose name resolves to a declaration other than a checked pure `function`,
tuple constructor or reserved intrinsic, such as a model operation, predicate or
clause, is `refused { code: ill_typed, cause: operator-ineligible }`. A call
whose name resolves to no declaration, such as an undeclared host operation, is
`refused { code: missing_declaration, cause: missing-name }`. Two function
declarations with one name in one package are each
`refused { code: ambiguous_declaration, cause: ambiguous-name }`, listing both
declaration loci, whether or not either is called. Every other typing failure is
`refused { code: ill_typed }` with its FR-272 cause.

Complete V1 defines no expression depth or nesting limit for checking. A checker
that bounds its own checking work must declare that bound as an NFR-010 node
admission limit before accepting the package; exceeding it is
`resource_exhausted` with cause `insufficient-next-charge`, naming the checking
stage, the declared limit and the node that could not proceed. It is never an
`ill_typed`, `undefined_expression` or admission verdict, and a host stack
overflow is not a Complete-V1 outcome.

Each call charges `function.call` under `quire.value.accounting/v1` before its
arguments are bound. Arguments are evaluated left to right before that charge.
Execution fuel is the `work_units` limit. Its exhaustion is
`incomplete { limit_kind: work_units, ... }` at the denied point, and the
function's static totality verdict is unchanged. Call nesting depth is not a
separate counter, and an implementation must not turn a host stack limit into a
Complete-V1 outcome.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-146-AC-1 | A pure recursive function with a proven decreasing finite measure is admitted and evaluates its declared result. | Test (TC-191) |
| FR-146-AC-2 | A nondecreasing or unproved call cycle refuses with the cycle and failed obligation. | Test (TC-191) |
| FR-146-AC-3 | I/O, mutation, reflection, ambient access or wrong argument/result typing refuses even in an unreachable branch. | Test (TC-191) |
| FR-146-AC-4 | A mutually recursive component is admitted only when every substituted cycle edge decreases the shared lexicographic measure. | Test (TC-191) |
| FR-146-AC-5 | Exhausting execution fuel for an admitted function returns incomplete without changing the function's totality verdict. | Test (TC-191) |
| FR-146-AC-6 | Each call charges `function.call` after argument evaluation and before binding, and an unproved partial-operation precondition refuses as `undefined_expression` while runtime typed refusals propagate. | Test (TC-191) |
| FR-146-AC-7 | A missing, mismatched or undecreasing measure in a recursive component is `undefined_expression` with cause `unproved-decrease`, a non-recursive function needs no measure, and a call to a non-function declaration or an undeclared name refuses with its named code. | Test (TC-191) |
| FR-146-AC-8 | Definedness and decrease proofs admit exactly the closed measure elements, guard facts, interval rules and decrease forms, `deref` and IEEE-to-`Rational` conversion have no body obligation, a duplicate function name is `ambiguous_declaration`, and a checking bound exceeded is `resource_exhausted`. | Test (TC-191) |
| FR-146-AC-9 | Rational `+`, `-`, `*`, `/` and unary `-` range proofs use exactly the numerator and denominator interval rules, a rational divisor whose numerator interval contains zero is `unproved-nonzero` without a nonzero fact, and every quantity division in a linked body is `unproved-nonzero`. | Test (TC-191) |

## Dependencies

- [Complete-V1 grammar](../../../proposals/quire-v1/shared-grammar.md) owns every
  source form used by this requirement.
- [Subsystem and interface index](../../subsystems/index.md) identifies the
  typed producer/consumer boundary and dependency direction.
- Every requirement linked in frontmatter is a normative prerequisite.
  Implementation ordering may add enablement edges but cannot change these
  semantics or infer implementation availability from specification acceptance.

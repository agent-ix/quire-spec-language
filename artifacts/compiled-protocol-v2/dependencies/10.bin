# Proposed composed-v1 state contract

Draft FS03 contribution. This selects the state meaning needed by the approved
capability inventory; it is not adopted or implemented by this document.
Historical profile files, source bytes and results remain unchanged. The
[first-profile amendment](../native-source-1-draft/README.md) is the retained
admission baseline, with the explicit extensions below. A full-v1 claim requires
the selected extensions; a first-profile implementation may still identify its
smaller scope accurately.

## Profile composition and type authority

The new definition bundle explicitly declares dependencies between its state
core, reusable predicates/queries and finite-graph facets. These are semantic
features of exact definition artifacts, not a union of installed backend options.
Selecting an alias in native source binds its exact definition closure. Duplicate
or incompatible definitions refuse; a caller cannot widen a callee's admission
by selecting a more permissive local profile. A cross-family predicate keeps its
own checked meaning and imports a compatible typed interface into its caller.

| Rule from the first-profile ruling | Composed-v1 disposition |
| --- | --- |
| Exact finite signed-64 integer bounds, checked arithmetic; normalized bounded rationals | Retained, including exact rational division with proved nonzero divisor and normalized result bounds; no integer division is admitted. |
| Float, implicit integer-to-rational conversion, saturation, silent widening, integer division/remainder | Refused, even in an unreachable branch. Historical 0-draft division results remain historical. |
| Authored presence/null/multiplicity must survive or refuse | Retained. D owns the source/model contract; no native field-presence syntax is invented here. |
| Acyclic typed structural containment | Retained. References are identity-bearing edges, never recursive record expansion. |
| Reference navigation, object identity and closure deferred in the first profile | Explicit finite-graph extension; no promotion in place of the old profile. |
| Ordered duplicate-preserving finite sequences, authored maximum at most 10,000 per wrapper | Retained, including unused/nested declarations. Per-wrapper bounds do not bound total nested work. |
| No user-defined calls in the first profile | Explicit acyclic Boolean-predicate extension; general recursion/functions remain refused. |
| Statically defined Boolean clause root; no partial/nullable truthiness | Retained across shared predicates and state clauses. Monitor status is not a Boolean. |
| Exact source authority and invocation anchors | Retained with the explicit alias/composite pre-state rules below. |

The model producer supplies named type/field/operation identities, numeric/text/
collection bounds, units, presence and reference roles. The checker preserves
them or refuses. A model's wider domain cannot be narrowed by choosing convenient
sample values. In particular, the original ConfigVersion minimum-only integer
and parent relationship remain a design/admission gate; neither is repaired here.

## Exact numeric operations and equality

Nonliteral arithmetic operands and their ordinary result use the same declared
numeric type and exact unit. A literal requires a unique expected declared type;
the checker cannot choose whichever imported scalar happens to fit. Integer and
rational values never implicitly convert to one another. Like-unit addition,
subtraction and comparisons are admitted; multiplication and rational division
require dimensionless operands. Unary negation, addition, subtraction,
multiplication and rational division must be proved
defined for every potentially evaluated value under declared bounds and sound
guard refinements. Unproved definedness is a checking refusal, not a discovered
logical violation.

Integer calculations and their range proofs use exact mathematics. Every result
must fit its declared inclusive bounds within signed-64 representability.
Rational operations use exact fractions, normalize sign/gcd (zero becomes 0/1),
then check authored numerator/denominator bounds after each operation. Raw proof
or normalization intermediates are not approximated or checked as though already
normalized results. Explicit resource exhaustion stops with incompleteness.
Rational `/` is an ordinary binary operator, separate from the `rational(n,d)`
literal. Both operands and the result use the same named dimensionless rational
type. For normalized operands a/b and c/d, prove c nonzero at every potentially
evaluated division, compute (a*d)/(b*c) exactly, normalize sign/gcd, then check
the normalized numerator/denominator against the declared result domain. There
is no truncation, rounded quotient, infinity, zero fallback or implicit unit
algebra. An unknown nonzero/range proof refuses checking; actual resource
exhaustion remains incomplete. Native short-circuit facts may justify the
division only in the branch and exact operand scope where they hold.

For a dimensionless Q with normalized numerator [-1,1] and denominator [1,2],
`rational(1,2) / rational(1,2)` is 1/1: raw 2/2 must normalize before the numerator
bound is tested. `rational(1,1) / rational(1,2)` refuses the result bound;
`rational(1,1) / rational(0,1)` refuses the zero divisor. For d:Q,
`d != rational(0,1) implies rational(0,1) / d = rational(0,1)` has a total
guarded quotient; the unguarded quotient needs a nonzero proof. This retains
ADR-0053's rational admission under native control semantics; it does not
inherit an external language's eager Boolean evaluation or integer rounding.

Equality admits Boolean, same named numeric/text/enum types, and explicitly
admitted object/reference identity. Text uses exact Unicode scalar sequences,
without normalization/collation; ordering is scalar lexicographic. Numeric order
is mathematical; enum/Boolean order is not inferred from host representation.
Whole-record, whole-sequence and whole-option equality remain refused. Their
fields/elements may be compared individually where admitted. Reference identity
comparison additionally follows the graph rules below.

`and`, `or` and `implies` evaluate left once and skip right for false, true and
false respectively. `if` evaluates one selected branch; `let` evaluates its
initializer once. Static name/type/profile admission still covers unselected
syntax. A presence guard is tied to the exact immutable optional value, field
identity and observation; it cannot justify another same-named field or snapshot.
The result and actual source activation/participation are separate facts.

## Ordered queries and exact aggregation

All query domains are validated, explicitly supplied `Seq(T,N)` values with a
declared finite maximum N ≤ 10,000. No operator looks up an ambient population
or reconstructs a missing member. A declared scope/view from D/F supplies any
population/window sequence, its identity and completeness. Missing sequence or
required membership is unavailable input, not an empty sequence. Closure/progress
and binding authority remain the observation contract's responsibility.

Bind each occurrence in source order. Repeated equal values or repeated references
remain distinct occurrences; traversal bookkeeping never deduplicates a source
sequence. Binder bodies are pure, typed and total in the shared expression
language. Runtime budgets count actual work and materialized output; a finite
declared maximum is not permission to exhaust resources.

| Form | Value and evaluation contract |
| --- | --- |
| `size(xs)` or `size<M::Count>(xs)` | Exact length. The explicit or uniquely expected dimensionless integer type must contain every admitted length, including zero. |
| `contains(xs, item)` | Same admitted equality type; scan in occurrence order, stop on the first equality. Empty is false. No implicit equality or coercion is added. |
| `forall(x in xs: p)` | Boolean; stop at first false; empty is true. |
| `exists(x in xs: p)` | Boolean; stop at first true; empty is false. |
| `filter(x in xs: p)` | Evaluate p for every occurrence; retain matching occurrences in their original order, with original values/identities. Output is `Seq(T,N)`, not a set. |
| `map(x in xs: e)` | Evaluate e once per occurrence in order; output `Seq(U,N)` with every duplicate result retained. U is the checked result type. No implicit flattening. |
| `count<M::Count>(x in xs: p)` | Evaluate p for every occurrence and count true results. Output type is the explicit dimensionless integer domain containing 0..N. |
| `sum<M::Total>(x in xs: e)` | Exact checked left fold from numeric zero in the explicitly named result domain, evaluating e once per occurrence. Empty is zero only when that domain admits zero. |

`sum` explicitly selects its result domain; this is not a general cast or an
implicit widening of ordinary arithmetic. Projection values must have the same
numeric representation (integer or rational) and exact unit as that domain.
Their exact values must be representable there, and every accumulated prefix
must be proved within its bounds. The operator's explicit same-representation
domain transfer allows positive bounded item amounts to contribute to a separately
authored total that also admits zero. No integer/rational or unit conversion is
permitted. Normalize each rational prefix before its bound checks.

Do not reorder summands to obtain a different definedness outcome. A final sum
inside the bounds does not rescue an out-of-range intermediate prefix. Sound
static range/length/guard reasoning may establish the obligation; inability to
prove it refuses the expression with a definedness cause. Selecting a stronger
checking method does not change the mathematical requirement.

Examples use **explicit synthetic domains**, not inferred production bounds:
Items have integer Amount 1..20 with unit U; at most five items are supplied;
Total is integer 0..100 with unit U; Count is dimensionless integer 0..5.
For amounts `[2,2,3]`, mapping amounts retains `[2,2,3]`, filtering amount=2
retains two occurrences, count is 2, sum is 7, and size is 3. Empty queries
produce the specified empty/zero/Boolean identities. Six possible items cannot
be admitted for this Total merely because today's sample sums below 100.
A Total with unit V or rational representation refuses this integer/U input.

Sets, bags, OrderedSet, implicit flattening and general recursive query functions
remain outside this selected extension. Explicit `map` plus bounded nested
queries does not remove the total-work/resource obligations.

## Operation anchors, aliases and captures

| Clause | Context self | Parameters | Result | `pre(e)` |
| --- | --- | --- | --- | --- |
| Invariant | Exact named initialization/handler observation's current snapshot | No operation parameters | Unavailable | Refused |
| Precondition | Selected operation invocation's pre observation | Immutable declared inputs of that invocation | Unavailable | Refused |
| Postcondition | Same invocation's post observation | Same immutable inputs | Exact declared post-only result | Selected invocation's pre reads only |

`pre(e)` is a snapshot selector for state reads, not a time machine for captured
values. In a postcondition it changes explicit context-dependent reads performed
inside e to that invocation's pre observation. Pure scalar invocation parameters
and literals can participate in a composite expression but are never replaced
with object fields. The expression must contain an eligible state read (including
a nested selector); a bare parameter/constant/captured value is not historical
state access and refuses. This retains the ruling's `pre(parameter)` refusal.

A `let` binds the value and its observation provenance at its initializer's
original location. `pre` does not re-execute an initializer outside its body or
retag a captured reference. A captured value tied to post/current cannot enter
a pre-selected expression; an observation-independent value may accompany a
permitted state read. Values initialized inside the selector use that selected
observation. Repeated nested pre selection is idempotent for eligible reads.
Named pure predicate calls receive explicit argument values and do not introduce
hidden state; their arguments obey these same rules.

Dereference follows the reference's own universe/type/observation. A pre selector
cannot retag a post-qualified parameter reference. An operation parameter is not
itself a context field; a bare or parameter-only historical selection refuses.
The model/binding must supply any reference qualification before evaluation.

Assume a declared bounded integer operation parameter `delta`, and bounds/guards
sufficient for the arithmetic shown. These are expected source correspondences:

| Postcondition expression | Outcome / observation |
| --- | --- |
| `pre(self.version)` | Read version in this invocation's pre snapshot. |
| `pre(self.version + delta)` | Pre version plus immutable invocation delta; no field named delta is read. |
| `pre(P(self.version, delta))` | P receives pre-read version and immutable delta under its checked signature. |
| `pre(let v = self.version in v + delta)` | v is initialized inside the selector from pre. |
| `pre(pre(self.version))` | Same pre read, with both source selectors retained. |
| `let v = self.version in pre(v)` | Refuse captured post value, without re-executing its initializer. |
| `let s = self in pre(s.version)` | Refuse post-qualified captured receiver, without retagging identity. |
| `let s = pre(self) in s.version` | Capture the selected pre context inside the selector; later reads retain pre qualification under the admitted context type. |
| `pre(delta)` or `pre(result)` | Refuse parameter-only or post-only selection. |
| `present(pre(self.parent)) implies value(self.parent)` | Refuse cross-observation presence fact. |

Pre/post inputs must name one exact operation, invocation, context and universe.
An invariant likewise needs its exact named initialization/handler binding;
a bare context type or arbitrary current snapshot is not a substitute.
A complete post snapshot that lacks required post self cannot supply that
context, even if the frame permits deletion; this is a binding refusal. Missing
observation/completeness evidence is incomplete input. Neither yields a Boolean.
An explicit pre-qualified reference may still navigate the pre graph after its
object was deleted in post; identity equality never creates a post object.
Frames preserve their exact authored write/create/delete permissions and actual
population-delta comparison; a caller cannot substitute a more permissive frame.

## Finite graph extension

The supplied environment is a finite typed universe with explicit immutable
snapshot identities, object IDs and closed reference targets. Duplicate object
identity or a target missing from a declared-complete universe refuses; missing
completeness remains unavailable input. Value containment stays acyclic even
when the reference graph contains cycles. No opaque UUID gains navigation.

`deref` requires a typed reference and resolves only in its qualified environment.
Identity equality compares exact universe, stable object type and object ID.
It may compare pre/post references to the same identity within that universe;
their observations remain distinct for reads. Different universes or types
cannot match through coincident string IDs or structural equality.

`reaches(a,b,edge)` means a path of **one or more** edges, not reflexive closure.
Inputs must share object type, universe and snapshot for traversal. The resolved
edge field on that type must be `Ref(T)`, `Option(Ref(T))` or `Seq(Ref(T),N)`;
other shapes require a separately specified extension. Optional absence/empty
sequence has no outgoing edge. Sequences preserve their declared edge order.
Heterogeneous relationships still support explicit typed navigation and queries;
they do not become an untyped reachability relation.

Explore edges in declared order; test whether a discovered endpoint is the target
before suppressing repeated expansion. Expand each identity at most once, using
the complete finite environment for termination. Thus an isolated a does not
satisfy `reaches(a,a,edge)`; a self-loop or cycle returning to a does. Duplicate
edges do not change reachability truth but remain observed traversal input.
Do not invent a path-length cutoff that returns false. Resource exhaustion is
incomplete, not unreachable. Any returned path witness retains the actual typed
edge and snapshot identities; shortest-path or canonical-witness guarantees are
not added by this Boolean predicate.

## Verification boundary

New requirements and cases are linked in the shared matrix. The selected
extensions require explicit definition artifacts, grammar/type agreement with
D/B/E/F, positive/adverse/boundary cases and the complete baseline review before
implementation. Existing compiler/TL tests are reusable inputs only after exact
correspondence is established. No source here dispatches business actions or
creates a model/evidence store.

# Native reference execution and accounting

Draft LC03 contract under FR-008 and NFR-006. The existing adopted
state-finite/0-draft semantics at specification e897f81 govern values and order.
The selected reference accounting label is native-ref-cost/1-draft. Qualification
must execute its independent exact vectors before claiming that accounting
implemented. Profile meaning, reference accounting and backend fuel are separate.

## API and ownership

runtime::evaluate takes a borrowed constructor-private ValidatedContext,
EvaluationLimits and a cancellation poll. It evaluates exactly that context's
selected CheckedClause over the original native AST. It does not evaluate the
IR definedness witness or build another expression compiler. The context owns
immutable admitted runtime data and borrows its exact CheckedPackage.

The native result retains the context correspondence, usage and ordered events.
Its state is Completed(Boolean), Incomplete(stop diagnostic), or
Refused(runtime-invariant diagnostic). The latter is defensive against a
violated established invariant; it is not a Boolean counterexample. There is no
untyped public constructor for a validated context, and no public evaluation
entry point accepting unchecked source or raw snapshots. A borrowed report
cannot outlive or be rebound to different inputs. B owns mapping these native
observations into TechniqueResult and owns the portable result envelope.

## Values and captured observations

Runtime values are immutable handles/views into the validated flat arena,
literal AST data or small computed scalar values. Reading a local, field,
optional payload, record or sequence does not deep-clone its contents. Native
types come from CheckedClause, never from first-fit runtime values. Object access
is identity plus captured observation, not a structural record copy.

Invariant self uses current; precondition self uses pre; postcondition self uses
post. Direct State reads use the current evaluation observation. Parameters
retain their pre capture and result its post capture. A lexical value retains
its original observation when read inside pre. Dereference uses the reference's
capture, including a pre object deleted in post. A conditional returns exactly
the selected value/capture. Nested pre selects the same pre observation.

## Scalar and structural operations

Boolean operators evaluate left to right with the specified short circuiting.
Implication evaluates its antecedent once, yielding true without evaluating
the consequent when false. A conditional evaluates its condition once and only
its selected branch. Let evaluates its initializer once and binds an immutable
value for its body. Every local use is a read, not textual substitution.

Integer operations use checked i64 operations and check the exact native result
interval. Division truncates toward zero; remainder has the dividend's sign.
No floating intermediary, wrap or saturation is admitted. The already checked
IR proof establishes definedness conditional on valid inputs; an unexpected
failure at execution yields runtime_invariant at the original expression and
no logical value. No second native range prover is introduced.

Text equality/order compare exact Unicode scalar sequences lexicographically,
without normalization, locale or host ordinal coercion. Eligible structural
record equality visits all fields necessary to determine equality, recursively
using the native type; record field order is declaration-name order. Boolean,
integer and enum equality compare their exact values under checked types.
Object/reference equality compares exact model/type/universe/object identity
and excludes observation. Distinct identities with equal fields remain unequal.
The checker already refuses Option/Seq equality and ineligible order operators;
the interpreter does not widen that whitelist.

## Collections and graph traversal

Size reads the actual ordered sequence length using the exact inferred scalar
type. Forall and exists bind each occurrence in sequence order, including every
duplicate. They stop at the first false or true predicate respectively. Empty
forall is true and empty exists false. No value is supplied for unavailable
sequence data because validation must finish first.

Reaches follows the resolved Option<Ref<T>> field in one exact observation.
It follows one or more edges: start=target alone is insufficient. Before
expanding a new identity, check cancellation and graph fuel. Read its edge,
test the reached identity for target, then suppress repeated expansion. Thus
self-loop and a cycle back to start succeed; an isolated object does not.
Traversal terminates from finite validated identity indexes, without a hidden
path-length bound. Graph traversal never evaluates a native predicate callback.

Collect, flatten, bags/sets, user helpers, recursion, casts and temporal forms
remain recognized unsupported constructs under the adopted profile. A collect
of duplicate outputs must be refused at the actual frontend phase, not
implemented as deduplication. These are complete refusal controls in the native
pipeline, not admitted expressions passed to the evaluator.

## Implication event contract

Each native implication preserves its ExprId and original operand spans. Events
are AntecedentEntered, AntecedentCompleted(Boolean) and ConsequentEntered.
The result's retained context supplies the exact authored RequirementRef,
ClauseId, source identity/revision/digest and model/input bindings. Each event
holds the implication ExprId, operand ExprId and original native Span; no source
identity string is cloned into every event. The report exposes their exact
formal source mapping through the retained CheckedPackage.

Entry is recorded after the next non-Group operand node passes cancellation and
its expression-step budget, immediately before operand work. Grouping preserves
the original operand region but adds no step. Exhaustion before that entry
produces no entry event. A completed antecedent records its actual Boolean
before selecting the consequent. Entry preflights expression, active-depth and
event capacity before committing either the expression step or entry event.
If an entry cannot be stored, no expression step is charged for that operand's
first node. AntecedentCompleted is a separate event-capacity check after actual
operand completion; failure there retains the consumed operand work and prior
events without manufacturing a completion event or clause Boolean.
False antecedents have no consequent-entry event. A consequent-entry event
proves entry only, not completion or truth.
Nested and repeated quantifier implications append events in actual evaluation
order, with repeated ExprIds allowed for different occurrences.

Event storage is bounded before append. When it cannot record the next event,
evaluation stops incomplete with no clause Boolean and retains the prior event
prefix. It never silently drops an event and returns completed. This storage
stop is distinct from absence of a backend probe, which remains unavailable
coverage at the separate backend interface. No event asserts deployment-wide
activation or verification adequacy.

## Exact reference steps

Charge one expression step immediately before entering every evaluated
non-Group AST node. This includes self/name/result reads, literal nodes, field
selection, pre/present/value/deref/size calls, operators, let, conditional,
quantifier entry and every actual predicate evaluation. Group is transparent.
Skipped branches and skipped quantifier occurrences cost no expression steps.
AST sharing used by proofs does not cache runtime expression evaluation.

Charge one graph step before expanding each previously unexpanded object in a
reaches call. Each call has its own visited set. Testing a reached target before
the repeat check costs no second graph expansion. Snapshot construction,
validation, model linking, event storage and deep value comparisons do not get
relabelled as expression or graph steps.

Mandatory independent accounting vectors, after successful setup:

| Expression | Expression steps to complete | Graph steps | Result |
| --- | --- | --- | --- |
| true | 1 | 0 | true |
| (true) | 1 | 0 | true |
| false implies true | 2 | 0 | true; antecedent entry/false, no consequent entry |
| true implies false | 3 | 0 | false; antecedent entry/true then consequent entry |
| let x = true in x and x | 5 | 0 | true |
| let x = false in x and x | 4 | 0 | false |
| if true then true else false | 3 | 0 | true |
| reaches(self, self, parent), isolated self | 3 | 1 | false |
| reaches(self, self, parent), self-loop | 3 | 1 | true |
| reaches(self, self, parent), two-object cycle | 3 | 2 | true |

The five-step let with budget four is incomplete. True-implies-false with budget
two retains antecedent entry and true completion, but no consequent entry or
Boolean. False-implies-true with budget two completes. Every vector also tests
one below its required budget, with no negative budget representation. Different
source grouping retains lineage while preserving these step counts.

## Auxiliary bounded work and cancellation

NFR-006 separately bounds deep value comparisons, text inspection, events and
stack depth. A value-comparison visit charges before inspecting one pair of
value nodes, including record/container roots; text charges one scalar-inspection
step per side advanced, including the final end check. This bounds long equal
prefixes and repeated deep comparisons while leaving expression/graph accounting
unchanged. Frame storage comparison uses the validation counters, not evaluation
fuel. Large values are borrowed rather than expanded before charging.

The caller supplies a mutable cancellation poll returning bool; a true result
requests stop. Poll before every charged work unit and loop continuation. Polls
do not consume expression/graph fuel. Production supplies no spawned worker,
clock-derived deadline or shared cancellation global. Tests use a deterministic
poll that cancels at a selected work boundary; they do not sleep or assert
wall-clock timing. A cancellation callback cannot mutate the borrowed checked
package or validated inputs through this API.

The poll is the only caller extension and must return promptly. A caller panic
propagates by ordinary Rust unwinding; the library does not catch it or return
a completed/incomplete report from a panicked call. No partial validated context
escapes. Allocator failure and a caller that never returns are outside the
logical-result API, not resource_exhausted observations. Construction has bounded
work but no cancellation callback in this initial API.

Evaluation stops at the first unavailable budget/cancellation boundary, keeping
only actual prior events and consumed counts. Repeating a run uses fresh usage,
locals, visited sets and events. A previous successful evaluation cannot make a
later smaller-budget request complete. Value, observation and source identities
do not change between retries. No totality, backend fuel parity or all-input
validity claim follows from one completed concrete evaluation.

# Native model roles and static checking

This contract implements FR-015/016 over Contract IR at
690bde7f2dc58662cf9ff0595c2c0e3b17107c6f. Existing formal-environment linking and
its byte artifact remain unchanged. Native semantics require the explicit
native-state-model/1 binding profile and the following typed Rust interface.

## Model interface

The `native_model` module exports NativeModel, NativeRoles, ScalarRole,
ScalarSite, ScalarKind, Unit, ObjectRole, OperationRole, Frame and ModelLimits.

```rust
NativeModel::new(source: FormalSource, environment: DeclarationEnvironment,
                 roles: NativeRoles, limits: ModelLimits)
    -> Result<NativeModel, Box<Diagnostic>>
NativeModel::source(&self) -> &FormalSource
NativeModel::environment(&self) -> &DeclarationEnvironment
NativeModel::roles(&self) -> &NativeRoles
NativeModel::artifact_bytes(&self) -> &[u8]
NativeModel::digest(&self) -> ByteDigest
link_native(unit: ParsedUnit, models: &[NativeModel], limits: LinkLimits)
    -> Result<LinkedPackage<'_>, Box<Diagnostic>>
```

The complete environment and every role are validated before constructing a
model. This first binding profile has one exact model source per environment;
foreign/multiple source documents require a later explicitly qualified profile.
All declaration/role SourceSpans must map back through the supplied FormalSource,
including unused record fields, variants and values. Source labels and bytes are
retained, never inferred from a declaration's display name or runtime sample.

NativeRoles has three explicit vectors: scalars, objects and operations. A scalar
role has a SymbolName name, IR source span, ScalarKind and a nonempty site vector.
ScalarSite selects a value by name or a field by record/name. Its scalar leaf is
reached through zero or more existing Option/Collection wrappers, without
following a record reference. Integer roles declare Unit::Dimensionless or
Unit::Named(SymbolName); their actual IntegerType comes from the selected IR
sites. Every site in one role must have the identical signed/reject-overflow
IntegerType. Text roles declare an inclusive authored maximum in Unicode scalars,
from zero through IR's MAX_TEXT_LENGTH. Every primitive integer/text site needs
one role, even if unused. A role cannot attach to Bool, enum or record leaves.

The scalar's identity is its explicitly authored role name under the exact
environment RequirementRef; it is a native scalar declaration, not an inferred
IR named type. DeclarationKey gains a distinct Scalar variant for those loci.
Different role identities remain different types even when their bounds and
units match. Bool is the profile Boolean. Enum and structural-record identities
remain the existing owner-qualified IR declaration identities. Option and
sequence wrappers retain their nested types and bounds. An ordinary record
admits equality only if each field type does; Option and sequence equality are
not introduced by representation similarity.

Every sequence declaration has a positive maximum no greater than 10,000,
including nested wrappers and unused fields/values. NativeModel::new owns this
admission check; ModelDraft::admit uses it as well. Oversized maxima return
unsupported_construct independently of runtime length and ModelLimits. The
general IR collection constructor supports a wider domain and is not this gate.

An ObjectRole selects an object record, a distinct reference-carrier record,
the carrier's identity-field SymbolName, an explicit universe SymbolName and
a role SourceSpan. The carrier has exactly one nonoptional Text field with an
explicit Text scalar role whose authored maximum is positive. Native object IDs
are nonempty opaque scalar strings within that maximum. Their actual values are
supplied with runtime populations; they are not enumerated in the model.

Object and reference-carrier records are disjoint, with one object-role owner
each; conflicting assignments refuse. IR record fields remain acyclic: a parent
field can be Option<Record<NodeRef>>, where NodeRef contains only its ID carrier
and no Node payload. The explicit native role gives that leaf Ref<Node> semantics.
self/deref denote identity-bearing object access. Ordinary records remain
structural records and ordinary enums remain enums. Reference carrier fields
are not native field-access syntax: self.peer.id cannot bypass deref or expose
an implicit record/reference conversion. The model defines ID representation
and universe/type identity, while FR-007 will validate finite population membership.

An OperationRole declares a context record, operation SymbolName, AnchorName,
source span, ordered parameter value names, optional result value name and Frame.
The context must have an ObjectRole. Parameters/results select actual IR Input
ValueDeclarations; parameters are captured invocation values from pre, result
from post. Duplicate parameters or a result also listed as a parameter refuse.
Input values must have an explicit operation role; no unscoped invocation input
is silently made available. Results are accessed through the native result
keyword, not by bypassing availability through their IR value name.

Frame explicitly lists allowed changed fields (record/name pairs), created
object types and deleted object types. These must resolve to actual object
records/fields. Empty lists mean an explicitly empty frame, not a missing frame.
Operation identity is owner/context/name; DeclarationKey gains Operation for
its distinct provenance. Native operation roles do not consume pure-function
signatures as executable effects. Supplied pure functions, rational values,
unsigned/saturating integers and other unadmitted representations refuse this
first model profile, including in unused declarations.

The artifact is UTF-8 compact JSON with fixed top-level field order: profile,
declarations, source, loci, roles. profile is native-state-model/1; declarations
is the exact existing V1 canonical-declaration UTF-8 JSON encoded as a JSON
string. source contains the opaque native identity/revision, SHA-256 source-byte
digest and explicit formal identity. The local display path is retained in the
model but excluded from portable artifact identity. loci contains every supplied
formal declaration's source span in declaration-key order. roles retains all
explicit semantic metadata and role source spans, with object/operation/scalar
inventories sorted by their declared keys and set-like site/frame lists sorted.
Operation parameter order is retained. Duplicate input entries refuse before
sorting; sorting does not silently deduplicate them.

Artifact numbers use exact base-ten integer spelling, with no floating-point
conversion. Strings use JSON scalar escaping, no Unicode normalization, and
otherwise retain UTF-8; object field order is fixed by the documented structures.
The pinned serde_json writer emits this raw artifact. This is not a new FS05
canonical identity domain or an IR CanonicalDigest. Native import digest is
ByteDigest::of(artifact_bytes); package and version retain the exact IR owner
namespace and positive revision decimal spelling. All included provenance and
unused declarations participate. There is no general artifact decoder in this
API and no new non-Rust producer execution.

ModelLimits defaults and hard ceilings are 10,000 roles, 10,000 scalar-site/frame/
parameter entries in aggregate, 10,000 formal type/declaration nodes, depth 64 and
1 MiB artifact content bytes. Source::read owns the existing 1 MiB source ceiling.
Counts/depth are checked before traversal or serialization; emitted content is
bounded before each append. Caller values can only lower these limits; zero is
effective. This is content accounting, not an exact allocator-capacity promise.

link_native shares exact import, ambiguity and lexical resolution with link.
LinkedPackage exposes its binding profile, and each LinkedModel exposes its
optional NativeModel correspondence. Existing link results have no native model.
In the native profile, deref/reference fields and operation clauses resolve to
their explicit roles. Same formal source identity with different native source
labels/revision/digest anywhere in the supplied inventory refuses. An exact
duplicate model still produces the existing ambiguity refusal, not precedence.
One formal RequirementRef cannot denote different native model artifacts inside
the inventory, even if their model source identities differ.

Model metadata/source inconsistency returns invalid_model_binding; a known
representation outside this binding profile returns unsupported_construct.
Both use the link phase at the model source's byte-zero locus, with original
related IR declaration loci where available. Source-coordinate validation
failures retain their actual structured upstream cause where one exists.
Existing missing/stale/ambiguous import codes remain distinct. Limits return
resource_exhausted. None of these refusals exposes a partially admitted model.

## Checking interface

The `checking` module exports CheckBindings, ClauseBinding, CheckLimits,
CheckedPackage, CheckedClause and native type/proof correspondence views.

```rust
check(package: LinkedPackage<'a>, bindings: CheckBindings, limits: CheckLimits)
    -> Result<CheckedPackage<'a>, Box<Diagnostic>>
```

CheckBindings owns the exact native FormalSource and a complete vector of
ClauseBindings. Each ClauseBinding supplies the exact native clause name,
authored RequirementRef, ClauseId and ExecutionPoint. Missing, duplicate, extra
or foreign bindings refuse. RequirementRef plus ClauseId identifies a clause;
native display names do not mint authored IDs. Invariant execution points must
be explicitly Initialization or Handler; operation Pre/Post points must match
the selected operation's AnchorName and native clause category. The caller owns
assigning authored identities, as with FormalSource; the future Quire manifest
consumer must validate that correspondence against its actual authored manifest.

The source binding must match the package's immutable identity/revision/path/
digest. It cannot collide with a model's formal source identity under a different
native binding. check requires the native model profile; passing a plain formal
link result refuses unsupported_construct. It never guesses native roles.

Native constraints unify exact declared types across arithmetic, comparisons,
conditionals and lexical uses. Roots/Boolean operators impose Bool. Field
receivers must have their resolved declaring record/object type. Option unwrap,
reference dereference and quantifier element constraints preserve their wrappers.
Literal or size classes with no unique declared scalar context refuse ill_typed.
Type constraints may propagate a body's context back to its let initializer;
this is compile-time inference, not duplicated runtime evaluation.

Integer literals must fit the selected declared integer and signed 64-bit
representation. Arithmetic operands/result have identical nominal type and unit;
multiply/divide/remainder require dimensionless units. Text literals respect the
authored scalar maximum. Integer/text ordering is admitted; Bool/enum ordering,
Option/sequence equality, cross-nominal equality and object/record coercion
refuse. size needs a dimensionless integer interval containing [0, sequence max].
Object/reference identity equality excludes observation but requires the same
type/universe. reaches accepts object/reference endpoints of the same target and
one provably identical observation; its edge must be Option<Ref<that object>>.

All native nodes receive type/availability checks, including unreachable nodes.
Active local names cannot shadow. A let initializer is outside its own lexical
scope; quantifier domain is outside its element scope. Present/value require
Option. Invariants use current, preconditions pre, postconditions post. pre is
post-only and changes context-dependent reads, not captured aliases. Reference
and object accesses carry their originating observation through fields, unwrap,
let and deref. A conditional whose reference branches differ in observation
retains a distinct selected-value observation identity; it is not retagged.

CheckedPackage owns the original LinkedPackage, source/clause bindings and
source-ordered CheckedClauses. Each checked clause exposes native expression
types, discharged proof goals/correspondence and runtime input requirements.
Each native ExprId still denotes the original AST node and source span. Groups
and implication nodes remain intact. No normalized proof expression becomes the
runtime evaluator's AST, and no successful static check yields a Boolean value.

## Definedness proof boundary

The native checker constructs a bounded proof graph and maps structured native
value keys to generated IR Input values. A key contains exact declaration,
receiver/path and observation, or a lexical binding/compound-expression identity.
It is not raw source text or a serialized expression used as semantic identity.
Repeated stable reads and stable-path let aliases share a key; pre/post direct
reads differ. Compound optional let values receive a stable lexical key, while
separately written unbound compound optionals do not gain a shared key.

The proof environment is explicitly owned by the authored clause RequirementRef.
Generated symbols are separate from model declarations, with a correspondence
back to their native values and original model loci. It contains no executable
callbacks. Integer proof values retain actual IR IntegerType bounds and policy.
Native nonnumeric payloads may be opaque Boolean carriers beneath Option and
Collection wrappers; their identity/ordering predicates are separate unknown
Boolean symbols, never equality of those artificial carriers. Thus the proof
overapproximates actual native values instead of inventing enum/object semantics.
Native typing remains responsible for admissible operators and exact identities.

An arithmetic/unwrap node receives a proof goal at its actual evaluation point.
The goal is checked through IR check_expression under its preceding guard stack.
A one-item IR collection with an always-true universal predicate can force
checking of a non-Boolean goal; this is a proof witness, not native collection
syntax or a runtime lowering. Guards wrap the goal in evaluation order. A later
body fact never enters an initializer's goal. IR's existing mathematical range,
nonzero and presence proof machinery decides discharge; failure remains
undefined_expression, with the structured IR diagnostic and exact native locus.

Native Boolean constants and their sound short-circuit consequences can prove a
path unreachable before emitting its definedness goals. All branch types and
names were checked separately. At alternative joins the proof makes no stronger
claim than facts true on every alternative; unknown Boolean tests stay unknown.
Conditionals and let remain native AST nodes even when a logically equivalent
Boolean proof formula is used. A compound computed numeric value may be replaced
by a fresh value with its declared interval after its initializer is separately
proved; no stronger range is invented. Quantifier elements are arbitrary values
of the declared element type for definedness, preserving lexical identity and
guard scope; this is conservative even when the runtime collection is empty.

Contextual size is a fresh integer proof value with its selected declared type,
after checking that type contains every possible length. Its native evaluation
will compute actual sequence length. No conversion from IR Length's unsigned
index type is performed. Dereference reads and reaches results use symbolic
values only after the native argument/field typing and nested definedness goals
are established. They remain conditional on complete, valid native populations.

The proof graph retains shared nodes until materialization. Every generated IR
node is charged before construction; repeated Boolean aliases cannot create an
unbounded expansion before accounting. CheckLimits defaults/hard ceilings are
10,000 native nodes, depth 64, 10,000 proof values, 100,000 proof-graph nodes,
100,000 presence-fact work entries, 100,000 total materialized proof nodes and
10,000 materialized nodes per goal.
Caller limits clamp downward, including zero. Expanded IR proof depth is at most
64, below the IR checker's thread-spawn threshold. No concurrent IR proof checks
or additional proof worker threads are started. Bound exhaustion returns
resource_exhausted with no partial CheckedPackage, not undefined_expression.

The native propositional layer derives only structured presence/absence facts;
it does not implement numeric range reasoning. Each Boolean outcome has either
an impossible-path marker or a set of such facts. Sequential paths union facts
and become impossible on a presence/absence conflict for the same key.
Alternative paths intersect their facts, ignoring an impossible alternative.
Present contributes its exact key/polarity, not exchanges outcomes, and Boolean
operators/conditionals combine outcomes according to their short-circuit paths.
Other Boolean predicates conservatively contribute no presence facts.

These derived facts are explicit proof premises, with correspondence back to
the native guard and selected truth outcome. A sufficient common fact is inserted
before the guarded IR expression, including inside a Boolean proof expression
later reused as another guard. This prevents reliance on an IR guard strategy
that cannot itself derive an alternative-join intersection. Each premise must
follow from the native guard rules; an optional tag's artificial payload never
supplies a fact. Fact union/intersection charges inspected entries before work.
The implementation exposes consumed counts and maximum native/expanded depth
in immutable check usage so exact-boundary tests need no timing or source scan.

The checked clause records the selected context object, model identities,
required observations/universes and operation/frame correspondence. Current
invariants require current context; operations require the selected invocation
and frame validation over pre/post, including post self even for a constant
postcondition. All referenced observations remain tied to captured values.
This records what FR-007 must validate; it asserts neither population closure
nor availability before those runtime inputs are supplied.

## Qualification

IT-005 uses a separately authored native-rule-model fixture. FR-025 promotes its
fallible Rust frontend into `model_source::read`: it calls actual IR constructors,
derives native roles and binds every declaration to its original JSON occurrence.
The explicit `native-rule-model/1` source profile returns a ModelDraft; its fallible
`admit` method applies NativeModel::new with independent limits. Source format,
declared metadata, limits and typed refusals are defined in
[FR-025](../spec/functional/FR-025-compile-rule-model-source.md).
The test wrappers retain setup and fixture license checks, with all semantic
lowering in production. Original FS03 fixture bytes remain
unchanged. The producer's concrete bounds/roles are checked independently against
the rule-model hypotheses before TC-025–029 are run. Setup failures fail tests;
they cannot impersonate a requested checker refusal.

TC-040–045 qualify this producer and model/link boundary. TC-025–029 and
TC-046–052 exercise the actual checker, including numeric/observation mutations,
source/anchor identity, all-branch typing, alias sharing, proof budgets and exact
retention. TC-053 adds an independent truth-table oracle for the native presence
fact rules, including mandatory positive and negative controls. First-party
implementation and tests are Rust under existing license
policy. Build/check commands remain local, single-job and low priority.

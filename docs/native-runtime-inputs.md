# Native runtime input and validation contract

LC03 contract reviewed before implementation. FR-018's structural constructors
are qualified; model-aware validation is qualified at 45ed1b4 by SR-097,
with 35 passing public API tests. Native evaluation and combined execution are
implemented under FR-008/023; selected byte intake is defined by FR-024. This
contract consumes the internally adopted state-finite rules at specification
e897f81 and the native model/checker landed in PR10 at bfac17d. It does not
modify the historical profile or fixture bytes. FR-018 owns input construction,
FR-007 owns validation, and NFR-006 owns implementation work ceilings.

## Existing owners and prerequisite boundary

The compiler's NativeModel is the exact admitted nominal/object/operation view;
CheckedPackage supplies authored clause/source bindings, native types and static
definedness. The IR remains the declaration and proof API. Runtime input cannot
change model roles, scalar bounds, operation anchors or frame permissions.

The inspected quire-contract-runtime at 7caefac supplies generated-oracle
operators, observations and campaign counts. Its snapshot-json format persists
campaign counters, not native domain populations. It has no native population
validator or interpreter to reuse. The new runtime remains compiler modules.

B's quire-verification shared-reference reader exists at d577486 in its VP01
worktree and retains the standard's 8ab058b schema/data snapshot. It owns shared
reference interpretation and portable method/plan/result contracts. Native input
artifacts below are domain payloads with local byte provenance, not replacements
for those wrappers. The existing IR BoundPackage/binder remains the backend
boundary. No dependency on uncommitted B work or edits to B/C/Filament/TL are
introduced. The last inspection found B's VP04 specification work in progress.

Runtime library implementation depends on the landed FR-015/016 APIs. Full
LC03 acceptance still depends on the remaining LC02 strict package/projection
handoff and FS03 acceptance. Those issue-level gates are not declared complete
by this intermediate API. Backend qualification and opaque Quire integration
remain required later stages of the original assignment.

## Flat input values

The public Rust input representation uses a flat value arena. ValueId is a
typed u32 index local to one input artifact; it is never an authored identity.
A ValueNode is Boolean, Integer(i64), Text, Enum(type, variant), Record(type,
fields), Absent, Present(child), Sequence(children), Reference(object identity)
or Object(object identity). Record fields pair an exact SymbolName with a
ValueId. All container children refer to earlier nodes in the same arena.
Forward, out-of-range and self references refuse. This bounds traversal and
destruction without a recursively owned caller-input tree. Structural record
cycles remain inadmissible; object-reference cycles are represented by identities.

Each root names a ValueId. Sharing a node does not infer native nominal type,
observation equality or a guard fact. Primitive nodes receive their exact type
from the admitted field/value declaration. Record and enum nodes additionally
name the exact declaration owner/type. Optional absence has an explicit node;
missing fields and unknown observations are not absence. There is no Null, float,
rational, set, bag, helper or callback value variant in this profile.

An ObjectIdentity contains the model RequirementRef, object record SymbolName,
universe SymbolName and exact text key. The role's reference-carrier text maximum
bounds the key; all Unicode scalar strings within that bound, including empty
text, remain distinct valid keys. No normalization, case folding or parsing of a
display label creates identity. Object and Reference nodes stay distinct kinds.
The containing snapshot or invocation role supplies the observation; a runtime
reference cannot select its own foreign snapshot.

## Native input artifacts

Snapshot::new and Invocation::new consume flat draft input plus caller-selected
SourceIdentity labels and ArtifactLimits. They return immutable native artifacts
with exact deterministic bytes, ByteDigest and a role-specific SnapshotRef or
InvocationRef. Reference constructors permit an explicitly expected digest;
resolution compares that digest to the supplied artifact. Identical labels and
content reproduce identical bytes. No timestamp, random ID, path or environment
lookup participates.

ValueIds are artifact-local indices, not generative arena tokens: an integer
copied from another draft has only its local numeric meaning. The constructor
does not claim to detect that caller intent. Cross-artifact object correspondence
uses ObjectIdentity and explicit snapshot references, never a foreign ValueId.

The emitted native-state-input/1 envelope has exactly version, kind, identity
and body. Kind is snapshot or invocation. Body fields are the complete draft
fields specified below; all are present. Snapshot and invocation use separate
Rust types. Construction establishes structure; FR-024 separately defines the
selected-byte reader. Neither operation is a shared-reference decoder or a
substitute for model-aware runtime validation.

`Snapshot::read_verified` and `Invocation::read_verified` bound bytes, verify
the selected digest, decode the closed envelope and select its version/kind
before decoding the body. Serde handles JSON; native records require objects,
all fields are required, and unknown/duplicate fields refuse. Existing IR
constructors validate declaration identifiers. Existing structural constructors
check every decoded artifact before admission. Whitespace and field order are
accepted and retained exactly; `bytes()` and `digest()` describe those external
bytes, while `usage()` describes the actual constructor pass. InputReadError
retains the expected reference, failed stage and original JSON/structural cause.
No reader opens files or starts runtime evaluation.

Encoding uses serde_json's exact integer/string encoding and declared struct
field order, with no extra whitespace or terminal newline. Sequences, flat arena
node order and all explicit input vectors retain their order in these byte
artifacts. Native semantic equality does not follow from artifact ordering or
digest inequality. A bounded writer charges each output byte before extending
its buffer. The digest is SHA-256 of those complete emitted bytes, not a new
semantic canonicalization domain. SourceIdentity labels are native-local
provenance and do not automatically establish a shared ArtifactRef authority.

The encoding's object fields appear in this order. Names are exact snake_case;
all listed fields are present. Optional result is JSON null or an unsigned
ValueId; this envelope null does not add a native Null value.

| Encoded object | Fields in emitted order |
| --- | --- |
| Envelope | version, kind, identity, body |
| SourceIdentity | identity, revision (both exact strings) |
| RequirementRef | package, requirement, revision (existing IR serialization) |
| Model binding | model (RequirementRef), digest (lowercase SHA-256 hex) |
| Qualified name | model (RequirementRef), name (SymbolName string) |
| ObjectIdentity | model, record, universe, key |
| SnapshotRef/InvocationRef | identity (SourceIdentity), digest (lowercase SHA-256 hex) |
| Snapshot body | observation, models, populations, values, arena |
| Population | model, record, universe, complete, objects |
| Object entry | key, fields |
| Field binding | name, value (ValueId) |
| State binding | declaration (qualified name), value (ValueId) |
| Invocation body | models, context (qualified name), operation, anchor, self_object, pre, post, parameters, result, created, deleted, arena |
| Parameter binding | declaration (qualified name), value (ValueId) |

Value nodes are objects beginning with kind. Kinds boolean/integer/text add
value; enum adds declaration (qualified name), variant; record adds declaration,
fields; absent adds nothing; present adds value (ValueId); sequence adds values
(ValueId vector); reference/object add identity (ObjectIdentity). Observations
encode as current/pre/post. Envelope version is native-state-input/1. Kind is
snapshot/invocation. SymbolName and anchor values use their existing exact string
representations; numeric payloads preserve signed i64 values. No unordered map
controls the order of emitted fields or entries. These emitted bytes are a native
domain payload; external-reader acceptance is governed by FR-024.

Artifact construction checks nonempty identity/revision labels, bounded values,
metadata entries, content and arena depth; duplicate fields/objects remain in
their vectors for the model-aware validator to diagnose rather than disappear
into a host map. Artifact construction alone establishes no population/type,
completeness or invocation judgment. All nodes, including unused arena nodes,
receive structural bounds/child-index checks; only required and supplied roots
receive model-dependent validation.

A snapshot body contains observation, model bindings, populations, state values
and its arena. A model binding is the existing exact IR RequirementRef paired
with the NativeModel artifact ByteDigest. A population names the model owner,
object record and universe, explicitly declares complete, and contains object
entries with text keys and field/root ValueIds. Every snapshot model binding
must select exactly one imported native model; duplicates, foreign owners or
stale digests refuse. Each supplied population/type and state declaration must
belong to those bindings. Model bindings needed by selected roots cannot be
inferred from a coincident type spelling.

State value entries pair a qualified model ValueDeclaration name with a root.
They must name State declarations. A named entry cannot override the separate
self context or the selected operation's parameter/result. Extra declared State
entries in a selected snapshot are validated too. An unused declaration need
not have a supplied root; every actual checked read needs its exact observation's
root. The paired-invocation rules below additionally require counterpart roots.

An invocation body contains model bindings, exact operation context/name/anchor,
self ObjectIdentity, selected pre/post SnapshotRefs, ordered parameter bindings,
optional result root, explicit created/deleted ObjectIdentity vectors, and its
arena. It has no caller-authored frame permission field. Parameters and result
correspond exactly to the selected NativeModel operation; parameters are captured
at pre and result at post. A result absent from the model requires no result
root; a declared result requires one. A parameter/result value cannot be supplied
as a snapshot State declaration or addressed through a result-name bypass.

SnapshotRef/InvocationRef contain exact SourceIdentity labels and ByteDigest;
their Rust types prevent swapping roles. No path grants authority. Within the
supplied inventory, one identity/revision may not name conflicting bytes or
artifact kinds. Exact duplicate entries are ambiguous and refuse; they are not
silently coalesced. The checker has no persistent history or external evidence
store: correspondence to artifacts outside the inventory is supplied by explicit
expected refs and the existing outer shared-reference owner.

## Validation API and selection

runtime::validate takes a borrowed CheckedPackage, an owned RuntimeInput inventory,
an ExecutionSelection, ValidationLimits and a caller cancellation poll. The
selection identifies an exact authored RequirementRef/ClauseId pair and either
current SnapshotRef plus self identity, or an InvocationRef. Invariants require
the current form; pre/post clauses require the selected operation invocation.
Foreign clause, operation, context or model bindings refuse. A local clause name
alone never selects an authored requirement. There is no latest-by-time selection.

Success returns a constructor-private ValidatedContext borrowing the checked
package and owning the immutable input inventory and selected validated indexes.
Accessors expose the exact clause/source/model/snapshot/invocation bindings and
consumed validation work. Failure exposes diagnostics and usage without a partial
validated context. No predicate is evaluated during validation.

The inventory is bounded before indexing. Known identity conflicts are checked
across the whole inventory. Model/value validation applies to selected snapshots
and invocation; unselected artifacts remain offered inventory, not validated
populations. An unavailable selected artifact/observation is incomplete. A
present artifact under the selected labels with a different digest is stale
and refuses. Missing static model/operation authority refuses instead of becoming
a runtime incomplete observation.

For invariant validation, self exists in current under its exact object role.
For operation validation, pre and post are both required: this is retrospective
validation of one recorded invocation, including for a precondition judgment.
It is not a prospective precondition API that authorizes a future mutation.
Pre self exists in pre; a postcondition additionally requires self in post.
A precondition can validate a permitted deletion of self. An allowed deletion
does not manufacture a postcondition context. Selected snapshot observation tags must match
their invocation roles; the two refs do not silently retag one artifact.

## Value, population and closure checks

Every required population in CheckedClause::runtime_requirements must be present
and complete. Native reference targets discovered from required/supplied roots
also require their target populations. Missing required populations and complete
false are incomplete; a known wrong type/universe or duplicate identity refuses.
For an incomplete population, available objects/fields still receive known-shape
and bound checks; a missing target there is unavailable evidence, not a proved
dangling reference. A missing target in a complete population is dangling_reference.

Every supplied object and State value in a selected snapshot is validated,
including fields a predicate will skip. Object fields exactly match their native
record declarations: no missing, extra or duplicate fields. Scalar integers and
text satisfy exact authored bounds, enum types/variants match, structural records
have the declared type and fields, and sequences preserve order/duplicates within
their declared maximum. Absent is valid only for Option; Present recursively
validates the child. Object/Reference nodes retain their exact model/type/universe
and resolve in the captured observation. Equal-valued objects are not merged.

Validation first establishes bounded population indexes, then checks references
against those indexes; no partially built population supplies a successful
lookup. It does not follow object references as recursive structural values.
Shared flat nodes are checked under each required type/observation use; caching
may reuse only an exact context and must preserve the documented work accounting.

## Invocation, frame and population differences

Parameters/result and every required State read are validated under their
captured observations before evaluation. A pre reference may resolve an object
deleted at post. Reference payloads cannot be retagged by pre(alias).

For every selected operation population, actual created/deleted identity sets
are the post-minus-pre and pre-minus-post differences. The supplied sets must be
duplicate-free, disjoint and exactly equal to those differences. Unavailable or
incomplete observations cannot justify inferred differences. Created/deleted
types and every changed field of surviving objects must be permitted by the
selected immutable model frame. Reference-valued fields compare identity across
pre/post; observation alone is not a field change.

Frame comparison is storage equality, separate from source-language equality:
it recursively compares optional tags, sequence order and multiplicity, record
fields, exact primitive values and object/reference identity. Thus Option/Seq
remain ineligible for source '=' while still supporting frame validation.

Frame roles grant object fields and created/deleted types, with no global State
root write permission. The union of supplied/required State roots in pre/post
therefore needs corresponding roots in both snapshots and preserved storage
values. Missing counterpart data is incomplete; a changed State root is a frame
violation. Object/Reference root identity may remain equal while its population
fields change under the authored frame. Adding global-root write permissions
would require a versioned native model-role extension rather than runtime input.

## Diagnostics and atomicity

The existing Diagnostic envelope gains optional structured runtime location,
without altering the source span's meaning. The primary source/span remains the
authored native clause or evaluated expression. RuntimeLocation names an exact
input artifact ref, observation and optional population/object/value/field path;
it does not invent a byte offset for a programmatically constructed value.
Related locations identify original model declarations where relevant.

New phases are validate and evaluate. New codes are invalid_runtime_input,
dangling_reference, incomplete_population, unavailable_observation,
population_delta_mismatch, frame_violation, cancelled and runtime_invariant.
Existing wrong_snapshot, stale_dependency, invalid_model_binding and
resource_exhausted retain their distinctions. Cancelled, resource_exhausted,
incomplete_population and unavailable_observation are incomplete conditions.
The separate per-diagnostic classification is retained when defects coexist.

| Condition | Stable code |
| --- | --- |
| Empty construction/ref identity labels | invalid_source_identity |
| Bad child/root index, duplicate inventory identity, duplicate object/field, wrong value shape/bound/type/universe | invalid_runtime_input |
| Foreign or missing static model/clause authority | invalid_model_binding |
| Present selected artifact/model labels with a different digest | stale_dependency |
| Supplied operation anchor/context or snapshot observation role mismatch | wrong_snapshot |
| Missing required artifact, observation or State root | unavailable_observation |
| Required population missing or declared incomplete | incomplete_population |
| Missing object target or required self in a complete population | dangling_reference |
| Duplicate/intersecting or incorrect declared created/deleted identities | population_delta_mismatch |
| Known effect outside the admitted immutable frame | frame_violation |
| Exhausted stage limit | resource_exhausted |
| Caller poll requests stop | cancelled |
| Evaluation contradicts a validated/checked invariant | runtime_invariant |

Runtime paths use ordered typed components for an artifact reference,
observation, model/population, object key, qualified State/parameter/result,
field name and sequence index as applicable. A draft-only error instead uses
arena/entry indices and supplied labels, without claiming an artifact digest.
Missing data is located at the expected reference/path. Model-dependent failures
also relate to the exact original model declaration. Symbol/string components
sort lexicographically and numeric indices numerically; absent optional
components sort before present ones.

Validation collects independently observable defects under its work/diagnostic
budget. Invalid dependent values are not traversed as if they had the expected
type. Missing observations do not suppress known defects in supplied data.
Diagnostics sort by validation stage rank (identity, binding, observation,
population, value, delta, frame), structured runtime location,
native source span and code. With sufficient work budget, permuting inventory,
populations or fields preserves the diagnostic multiset and sorted result.
A stopped traversal reports only its observed prefix, not a claim that all
permutations would discover the same defects before exhaustion.
If work stops, the report retains already observed diagnostics and the stop
reason. Overall refusal takes precedence when a known invalid condition exists;
otherwise unavailable/exhausted/cancelled validation is incomplete. Neither path
has a predicate Boolean. The report has a separate optional terminal reason in
addition to its detail vector. The first defect needing a detail slot beyond
capacity stops with resource_exhausted; capacity zero permits successful valid
input but retains no detailed defects. Classification still remembers whether
the unrecordable defect was invalid or incomplete. No duplicate terminal entry
is inserted in the detail vector.

Before an artifact or authored selection exists, InputError carries the stable
Code, supplied identity labels, structured draft path and construction usage.
It implements the standard Rust error interface without inventing source bytes,
spans or an artifact digest. Validation adds real native clause/declaration loci;
a foreign clause selection is located at the parsed unit with its requested
authored identity retained, rather than assigned another clause's span.

Validated input is immutable and tied to the exact checked package. A later
request with a smaller budget repeats its own work; success cannot leak through
a shared validation cache. Input defects, diagnostic storage and cancellation
remain bounded under NFR-006. Runtime truth and portable result adequacy are
outside the validation judgment.

# Native checked-package artifact and reconstruction

Draft LC02 contract for FR-019/020/021 and NFR-007, based on compiler
789c636bf9fb26812a74a2a2f5310b595262bfee. This is the compiler-owned payload
inside any separately selected transport/reference envelope. B owns shared
references/results; Contract IR owns executable binding. Plan-007 tracks
implementation after all-eight QUOIN review at 69588ad and the source-setup
correction/re-review at 2c6b9b8/1c3aa50. Construction, verified reconstruction
and runtime integration are implemented under Task-016/017/018.

## Purpose and boundaries

A checked in-memory package can be exported to a versioned artifact selected
by exact bytes. This contract exports its complete dependency,
authored-clause, resolution and runtime-obligation manifest. Reading that
manifest reconstructs through real native compilation; serialized claims never
construct a CheckedPackage directly.

Original source bytes and admitted NativeModel objects are explicit dependencies
supplied by the caller. This is not a source archive or general formal-model
decoder. The package contains the complete selected model artifacts, including
their declaration/type/role inventories, but those strings cannot manufacture
admitted models. Resolved expressions remain the original source AST; a second
expression wire/interpreter is not introduced.

The exact package ByteDigest identifies its complete bytes. The separate native
static identity follows the complete named derivation below; independent FS05
consumer adoption remains required. Existing IR canonical and bound identities,
shared-reference versions and JCS remain unchanged. Runtime
population, evaluation results, proof-engine work counters and producer/run
attestations are not static package fields. B's run/producer references retain
the separate execution provenance; a package is not a producer attestation.

## Rust API

The new `package` module owns the following public interface; no new crate is
needed. Names below are part of this contract.

```rust
NativePackage::new(checked: CheckedPackage<'model>, limits: PackageLimits)
    -> Result<NativePackage<'model>, Box<PackageError>>
NativePackage::read_verified(
    bytes: &[u8], expected: NativePackageRef,
    bindings: CheckBindings, models: &'model [NativeModel],
    support: &PackageSupport, limits: PackageReadLimits,
) -> Result<NativePackage<'model>, Box<PackageError>>
NativePackage::checked(&self) -> &CheckedPackage<'model>
NativePackage::bytes(&self) -> &[u8]
NativePackage::digest(&self) -> ByteDigest
NativePackage::canonical_identity(&self) -> NativePackageIdentity
NativePackage::reference(&self) -> NativePackageRef
NativePackage::usage(&self) -> &PackageUsage
NativePackageRef::new(digest: ByteDigest) -> NativePackageRef
NativePackageRef::digest(&self) -> ByteDigest
NativePackageRef::format(&self) -> &'static str
```

NativePackage is constructor-private, owns the checked package and accepted
immutable artifact bytes, and borrows the same admitted models through that
checked package. Reference types cannot be interchanged with SourceIdentity,
SnapshotRef or InvocationRef. A NativePackageRef is a byte selector, not proof
that a package exists or has been checked. It fixes format native-linked-package/1.

PackageReadLimits contains separate `package: PackageLimits`, `syntax: Limits`,
`link: LinkLimits` and `check: CheckLimits`. Each default equals the existing
stage default or the new package defaults. PackageSupport contains a set of
supported feature strings; its default is the complete known set below. An
extra caller string cannot enable an unknown wire/profile/feature in the reader.
No I/O, mutable global state or callback is needed. Existing runtime APIs
consume package.checked() and retain their validation and cancellation contracts.

## Closed outer representation

The format is UTF-8 JSON, with no final newline. Producers emit compact JSON
with the following top-level field order. Every member is required, including
nulls and empty arrays. No extension container or unknown fields are admitted.
The structural schema is
[native-linked-package-1.schema.json](../schemas/native-linked-package-1.schema.json).
Its success does not prove constructor constraints, source correspondence,
feature derivation or compiler reconstruction. The admitted native grammar
requires at least one model import and one clause. A header-only source cannot
produce a CheckedPackage or a successful NativePackage. Empty serialized
inventories remain untrusted claims and cannot bypass actual reconstruction.
The smallest positive fixture uses an actual imported model and constant clause;
this package contract does not change the native source grammar.

| Member | Exact content |
| --- | --- |
| format | String native-linked-package/1 |
| semantics | The closed semantic-selection record below |
| required_features | The exact derived feature set, sorted by UTF-8 bytes |
| source | `{identity, revision, digest, formal}`; identity/revision are original opaque native strings, digest is sha256 plus lowercase hex, formal is `{document, revision}` under existing IR constructors |
| models | One import record per native source import, in original import order |
| clauses | One checked clause record per native source clause, in original clause order |
| canonical_identity | Closed `{domain, version, algorithm, digest}` under the native static identity contract below |

Source/formal labels and all emitted strings retain exact Unicode scalar
spelling without normalization. The source's local display path is excluded;
the reconstructed CheckedPackage retains the caller's actual path for diagnostics.
All integer fields are decoded directly from the original bytes into exact
Rust integers, without fractions/exponents or floating-point conversion of
their semantic value. Opaque native revisions never become IR numeric
revisions implicitly. Positive u64 IR revisions remain exact Rust integers;
an external consumer needing a narrower domain must explicitly refuse it.

The semantic-selection record has fixed member order `language`, `edition`,
`syntax_profile`, `model_profile`, `checking_contract`, `ir_revision`,
`base_definition`, `rules_definition`. Their exact values are:

- language: ix:native; edition: 0-draft; syntax_profile: state-finite/0-draft.
- model_profile: native-state-model/1.
- checking_contract: native-checked-clauses/1, naming the existing FR-016 contract
  as selected by this artifact version; it does not change language meaning.
- ir_revision: 690bde7f2dc58662cf9ff0595c2c0e3b17107c6f.
- Each definition is `{revision, digest}`, where revision is the exact standard
  Git revision e897f810a7356d4ce8fd19026221ebda7b65596f, not an authored IR revision.
- base_definition.digest is sha256:8bc68a3c7e46d26c7191dfbb662d4d63070af9fedc9ce985fb1885f7efe29429,
  for proposals/state-core/profile.md at that revision.
- rules_definition.digest is sha256:d9eb316752ac45d7984b355a054cd279ef9749164a92c7f61fbf621fe280588b,
  for proposals/state-core/state-semantics.md at that revision.

Both artifacts are required: the unchanged base-profile file alone does not
identify the adopted semantic refinement. These pins record the owner's adopted
internal semantics, not a new shared SemanticRef or a public release. Different
definition bytes require an explicit new supported selection; there is no latest
definition lookup or caller-overridden meaning.

## Model, clause and correspondence records

An import record has `alias`, `owner`, `digest`, `artifact` in that order. owner
is `{package, requirement, revision}` using the existing RequirementRef fields
and constructors. digest is the exact selected NativeModel digest. artifact is
the exact native-state-model/1 UTF-8 artifact represented as a JSON string,
matching the already qualified model artifact contract. Its internal canonical
declaration string and role/locus fields retain their existing encoding. It is
compared against the supplied admitted model, not independently interpreted as
an untrusted model constructor. Repeated imports through distinct aliases retain
their source order; unselected offered models are not silently added to closure.
Existing link_native inventory conflict checks still inspect all offered models.

A clause record has these members in fixed order:

| Member | Exact content |
| --- | --- |
| name | Original native clause name |
| owner | Existing RequirementRef shape `{package, requirement, revision}` |
| clause | Existing ClauseId string, supplied by external authored bindings |
| kind | invariant, precondition or postcondition |
| execution_point | Closed existing ExecutionPoint shape: initialization/handler with name, pre/post with operation; tag member kind comes first |
| span | Original native clause span `{start, end}` in zero-based half-open bytes |
| expression | Original root ExprId as an unsigned integer local to this exact source |
| context | DeclarationLocation record described below |
| operation | DeclarationLocation or explicit null |
| occurrences | Complete LinkedClause occurrences in their existing retained order |
| runtime | Complete RuntimeRequirements projection described below |
| projections | Exactly the two dispositions described below, in their declared order |

DeclarationLocation is `{identity, source}`. identity is `{owner, key}`; owner
uses the RequirementRef shape. key is a closed tagged record: `type`/`value`/
`scalar` with name; `field` with record/field; `variant` with enumeration/variant;
or `operation` with context/name. No display-name matching joins namespaces.
source is the existing IR SourceSpan shape `{start, end}`; each endpoint is
`{source, line, column, byte_offset}`, with source `{document, revision}`.
The native producer obtains these values from checked original correspondence;
it does not reconstruct locations by string search.

An occurrence is `{expression, span, target}`. expression is its original ExprId
or null for context/operation occurrences. span is the original native byte
span. target is `{kind: formal, declaration: DeclarationLocation}` or
`{kind: local, binding: native-byte-span}`. Local binding spans and ExprIds are
only handles under this exact source, never global semantic identity.

runtime has `context`, `context_observations`, `universes`, `operation`,
`validate_frame`. context repeats the exact runtime context location.
context_observations uses the order current, pre, post with only required entries.
universes is a duplicate-free list sorted by model owner, object record and
universe, each `{model, record, universe, observations}` with the same observation
order. model is the exact RequirementRef. operation is null or
`{model, context, name}`, selecting the complete immutable OperationRole in that
model artifact, including ordered parameters, result and frame. validate_frame
is Boolean. The complete model is the type/frame authority; the package cannot
enlarge permissions. Any discrepancy from actual RuntimeRequirements refuses.

These closed wire records must enforce closure themselves. Existing IR Rust
Deserialize implementations are not assumed to reject every extra member;
wire adapters check the exact shape and then use existing IR constructors.
This is a representation adapter, not a second declaration/type authority.

## Projection inventory

Each clause has exactly:

1. `{target: native-reference/1, status: available, cost_model: native-ref-cost/1-draft}`.
   This selects the qualified existing native evaluation API after validation;
   it carries no predicate result or promise that runtime input is available.
2. `{target: quire.contract.executable-projection/v1, status: unlowered,
   code: unsupported_construct, span: <original root expression byte span>}`.
   It retains the original source obligation while LC04 lowering remains absent.

No branch supplies a fake IR projection, executable AST, proof-as-code conversion
or omitted clause. A later available IR disposition requires reviewed lowering,
actual BoundPackage construction and separate backend qualification. That change
reopens this closed artifact contract; this version cannot be read as promising
an available backend. The current package API exposes no successful BoundPackage
accessor. The actual backend/full ConfigVersion workflow remains mandatory work
under LC04/IT-002; it is not satisfied by unlowered descriptors.

## Required feature derivation

The known strings are boolean, integer, text, enumeration, structural-record,
object, reference, option, sequence, integer-arithmetic, comparison,
boolean-control, conditional, let, quantification, reachability, pre-observation,
precondition, postcondition. There are no feature aliases or implicit extensions.

Scan all original expressions, including unreachable branches, and every
selected model declaration/role, including unused fields, enum variants,
operation parameters/results and nested wrappers. Each checked expression's
native type also contributes its type features, including inferred literal and
size results. Primitive and named types
contribute their corresponding type feature; object/reference roles contribute
object/reference instead of treating their carriers as ordinary structural
records. Record field types are still scanned. An operation always contributes
object; actual pre/post clause kinds contribute precondition/postcondition.
pre syntax contributes pre-observation. Scalar arithmetic, comparison, Boolean
operators, if, let, forall/exists and reaches contribute their named feature.
Boolean clauses always contribute boolean. Literals contribute their primitive
feature. Grouping, reads and field selection contribute no extra feature beyond
their already represented types. Duplicates are removed only from this derived
set; no source, declaration or sequence inventory is deduplicated implicitly.

The operator mapping is exhaustive: not/and/or/implies contribute boolean-control;
unary minus and add/subtract/multiply/div/rem contribute integer-arithmetic;
all six equality/ordering operators contribute comparison. present/value,
deref and size contribute only their already scanned operand/result type
features; pre additionally contributes pre-observation. No arbitrary feature
string is minted from an enum's Debug spelling. Named type references are
visited by exact model/declaration identity, with a visited set and the existing
model/AST ceilings; object-reference cycles cannot cause recursive expansion.

The reader validates uniqueness before sorting feature strings. Unknown feature
strings and required features outside PackageSupport receive unknown_required_feature.
Omitting a known required feature or inventing a known-but-unused one fails the
later complete derivation comparison with invalid_package. Reordering the unique
set is accepted; that does not make import/clause/occurrence arrays set-like.

## Intake sequence and failures

1. Admit the entire offered byte length under PackageLimits before hashing or
   parsing. Verify the expected raw package digest; mismatch is stale_dependency.
2. Recognize one complete UTF-8 JSON object through a bounded Serde traversal.
   Reject BOM, trailing content, malformed JSON strings/numbers, unpaired
   surrogates and duplicate decoded members at every nesting level as
   invalid_package (invalid_utf8 remains distinct). Retain the top-level format
   string and original input; do not interpret version-specific payload fields.
   Recognition uses Serde's i64/u64/finite-f64 JSON number domain; a number
   outside that domain is invalid_package. This generic syntax pass supplies
   no numeric payload values to typed decoding or canonicalization: the original
   bytes remain authoritative, including integers above binary64 precision.
   No host map may erase duplicates before
   rejection. No handwritten quote/delimiter lexer is used.
3. A missing or non-string format is invalid_package. An unknown format is
   unknown_wire, even when that valid JSON object's other fields do not fit
   version 1. Only for native-linked-package/1, decode the original bytes into
   closed typed records under a separately metered pass, rejecting extra,
   missing, incorrectly typed or constructor-invalid members as invalid_package.
   Select language, edition, syntax/model/checking profiles, IR/definition pins,
   canonical domain/version/algorithm, then required features, in that order.
   Existing unknown_language/unknown_edition/unknown_profile distinctions apply;
   wrong IR/definition selections are unknown_profile. No decoder fallback or
   interpretation of an unsupported version's fields occurs.
4. Verify source identity/revision/digest/formal identity against the externally
   selected bindings. Native byte mismatch is stale_dependency; a foreign
   native/formal identity is invalid_model_binding. Check wire authored clause
   bindings against the complete external CheckBindings before they can be used.
   External bindings may be permuted, but duplicate names or owner/clause pairs,
   a different count, and any missing/extra/changed binding are
   invalid_model_binding. Count bounds precede inventory allocation/traversal.
   This comparison uses exact name/owner/clause/execution-point values, not
   serialized checked status; the actual checker still validates them in step 5.
5. Parse the original supplied source with actual syntax limits, link against
   the complete offered NativeModel inventory with actual LinkLimits, and check
   with the external CheckBindings and actual CheckLimits. Existing missing,
   stale, ambiguous, unsupported, ill-typed and undefined-expression diagnostics
   retain their native code, phase, original span and related loci.
6. Regenerate the complete manifest from the reconstructed checked package,
   under fresh bounded derivation accounting. Compare every member, normalizing
   only JSON object-member representation and the declared feature set. Model
   artifact strings and all ordered arrays remain exact. Any altered or omitted
   derived claim receives invalid_package at its actual package field path.
7. Expose the NativePackage only after complete agreement. Retain the accepted
   original bytes/digest, not rewritten canonical bytes. The reconstructed
   checked package supplies the source/model/runtime correspondence to callers.

Earlier numbered stages take precedence when several defects coexist. Within
recognition or closed decoding, stop at the first defect in input traversal
order; a budget is checked before the operation it charges. Selection uses the
fixed order above, independent of object-member order. External source checking
compares native identity, native revision, native digest, then formal identity;
wire/external authored comparisons follow wire clause order, whose source-order
claim is independently checked during regeneration. Complete manifest
comparison follows producer member/array order and reports the first mismatch.
This defines deterministic diagnostics for each input, without requiring
different malformed representations to report an identical first defect.

PackageStage distinguishes encode, decode, rebind and compare. PackageError
implements std::error::Error through thiserror and carries the stable crate Code,
stage, structured field/index path and actual per-pass usage. Its optional typed
cause is the original serde_json error (including its line/column) or native
Diagnostic. Wire errors have no invented native span. The three new Code values
are InvalidPackage/invalid_package, UnknownWire/unknown_wire and
UnknownRequiredFeature/unknown_required_feature. Existing code spellings remain.
Package error classification follows the retained native cause for upstream
failures; resource_exhausted is incomplete at every stage.

PackageUsage records admitted input bytes once and separate optional pass
records for recognition, decode, derive, canonical, encode and compare. A pass
not entered is absent, not a fabricated zero-work success. Each record holds
output bytes, decoded string bytes, entries and maximum container depth; fields
inapplicable to that pass are zero. NativePackage::new performs derive,
canonical and encode. read_verified performs recognition, decode, rebind,
derive, canonical, encode and compare; its syntax/link/check usage is retained
only where the existing APIs expose it, separately from package passes. Selected
frontend limits and the original Diagnostic remain available; unavailable
upstream counters are absent rather than invented. Rebind is a PackageStage, not a second JSON
pass. Canonical work belongs to stage encode during construction and compare
during reconstruction. Comparison does not skip excluded projection metadata.

## Native static identity

FR-021 selects domain quire.native.bound-package, version v1, algorithm sha256.
The emitted digest is bare lowercase 64-hex. NativePackageIdentity is a distinct
immutable Rust type with Display yielding that bare digest; it is not an IR
CanonicalDigest or ByteDigest. Its constructor is internal to checked derivation.

Canonical content is the complete manifest in this contract, excluding the
top-level canonical_identity member and each clause's projections member.
Every other member participates, including exact source digest/identities,
semantic definition pins, complete selected model artifact strings, authored
clause identities, source locations, resolved targets and runtime obligations.
Import/clause/occurrence arrays remain ordered. Required features and runtime
universe/observation sets use their expressly defined producer order. Object
members use the fixed field orders specified here; no global key or array sort
is applied. Changing source spelling can change this source-bound identity even
when an independently typed IR expression retains its source-free digest.

Encode canonical content as compact UTF-8 JSON with exact decimal integers,
no newline, no normalization and no BOM. String escaping uses double quotes,
backslash for quote/backslash, the short b/f/n/r/t escapes for their controls,
and lowercase `\u00xx` for the other U+0000–001F controls. Other Unicode scalars
remain their UTF-8 bytes, including supplementary characters; slash is not
escaped. This fixed typed-record encoding is not RFC 8785 and never converts
u64 revisions through binary64. The already qualified model artifact strings
retain their complete bytes inside this outer escaping.

The hash preimage is UTF-8 `quire-spec-language`, NUL,
UTF-8 `quire.native.bound-package/v1`, NUL, UTF-8 `linked-package`, NUL,
then the canonical content bytes. The textual output domain labels are not
substitutes for those exact preimage bytes. There is no newline or extra prefix.
The canonical_identity field is added only after this derivation, avoiding a
self-referential digest. Its closed selection is domain
quire.native.bound-package, version v1, algorithm sha256, digest as defined above.

Canonical encoding is a separately bounded/reported pass under NFR-007 before
the final artifact encoding pass. Both successful and refused work retain their
own actual counters. The reader selects the canonical domain/version/algorithm
with unknown_profile for an unsupported selection, then recomputes the digest
from its reconstructed checked package. A malformed/substituted or mismatching
digest is invalid_package. Native canonical equality never skips full manifest
comparison: forged excluded projections still refuse.

Projection availability is deliberately outside static identity because it is
a capability/derivation claim, not native clause meaning. It remains inside
raw artifact bytes and is verified against the actual producer's dispositions.
This separation does not qualify an unavailable backend or grant evidence reuse.
The new native domain is not added to B's closed shared-reference schemas by
this payload contract; A/B must qualify that explicit registration under FS05.

## Qualification and remaining acceptance

IT-007 must build the source-derived rule model, actual checked clauses and this
package, read it through real reconstruction, then execute healthy/violating,
refused/incomplete and operation cases through the unchanged runtime APIs.
Adverse wire/derived mutations recompute the expected raw digest where necessary
so a digest failure cannot impersonate strict decoding or semantic reconstruction.
Independent assertions enumerate every exported field/obligation and retained
projection; round-trip equality alone is insufficient qualification.

The Rust pipeline and its pure reader require no network or external producer.
JSON grammar belongs to Serde; private typed/budget adapters own only this
payload's fields and limits. Existing shared-reference consumption and the IR
strict executable binder were inspected and remain under B/C ownership.

The existing fixture-audit JSON visitor has a different numeric, node/depth and
error contract; copying it would not establish these limits. The package's
recognition adapter remains private and contains no application interpretation.
Typed selectors stay strings until explicit selection, so a Deserialize enum
cannot turn unknown_wire or unknown_profile into an accidental shape error.
The existing serde_json pin may enable unbounded_depth solely so this adapter's
own checked 128-container bound governs traversal; other decoders retain their
existing recursion configuration. No arbitrary-precision number feature or new
production dependency is required. Qualification may use the already locked
MIT jsonschema 0.17.1 as a direct development dependency with default features
disabled and draft202012 enabled. Schema tests use local references only and
do not perform network/file resolution. Dependency inventory and the complete
existing audit/IR regression remain required when selecting these features.
Shared canonical-domain registration, independent consumer adoption, compiled ConfigVersion
backend parity and Quire extraction integration remain explicit full-assignment
acceptance work. No passing package test can close those gates.

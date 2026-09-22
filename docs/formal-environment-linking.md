# Native formal-environment linking

The `native-formal-environment/1` binding profile makes FR-005 concrete using
Contract IR's existing public Rust API at 690bde7f2dc58662cf9ff0595c2c0e3b17107c6f.
It introduces no Filament reader, generic model service or replacement binder.
It is an explicit A-owned mapping for native compilation over already validated
formal declarations. Additional native object/reference semantics remain required
for the full ConfigVersion/parent-graph workflow.

## Public API and ownership

`link(unit: ParsedUnit, environments: &[DeclarationEnvironment], limits: LinkLimits)`
returns `Result<LinkedPackage<'_>, Box<LinkingError>>`. LinkedPackage owns the exact
parsed source and borrows immutable environments for its lifetime. It exposes
read-only source/unit, selected models, clauses and resolved occurrence records.
No borrowed input is mutated or cloned into a second type authority.

Each formal declaration identity consists of its complete RequirementRef and a
closed declaration key: type, record field, value or enum variant. A field key
includes its record and field symbols; a variant includes enum and variant.
Each occurrence carries the original native Span and the declaration's original
IR SourceSpan. Local references instead identify the exact native binding-name
span in the same ParsedUnit. A local span is not relabeled as an IR declaration.
Linking retains the native source's opaque labels and bytes without converting
them to numeric IR revisions. Later expression lowering requires a separately
explicit source-revision/anchor binding; this API does not fabricate it.

## Exact import binding

For each validated environment, the existing
`canonical_declaration_with_limit(CanonicalProfile::V1, maximum_bytes)` returns
the selected artifact bytes. The native model import uses:

- package: exact `environment.owner().package().as_str()`;
- version: exact decimal `environment.owner().revision().get()` spelling;
- digest: ByteDigest::of(output.bytes().as_slice()), rendered as sha256 plus
  the lowercase hexadecimal hash, under the existing ByteDigest Display contract.

The actual RequirementRef, including its requirement identifier, remains in
the canonical bytes and resolved declaration identity. CanonicalDigest hashes a
different preimage and cannot be substituted. This selected artifact excludes
IR source provenance according to its existing profile: semantic declaration
identity and declaration source correspondence remain separate output facts.
The native source bytes and their digest are always retained independently.
No compiled Filament artifact, manifest, lock or historical fixture is silently
renamed into this new binding profile.

An environment is closed over its declared type dependencies by the existing IR
constructor validation. Altering any included declaration changes this import's
artifact bytes; no external package closure is guessed. This is the stale-closure
control for this profile, not a claim to have decoded another model's lock format.

Resolve all imports in source order. A package absent from the supplied inventory
returns missing_import. A known package with no exact revision/digest returns
stale_dependency. A malformed digest returns invalid_model_binding at its token.
Multiple exact candidates refuse as ambiguous_declaration; no input order wins.
Distinct imports may share an alias. Context/enum lookup considers all exact
imports under that alias, chooses one matching export or reports missing_declaration
or ambiguous_declaration at the authored name with every conflicting formal locus.
Duplicate clause names refuse invalid_model_binding.

## Native name resolution

A current invariant context selects one RecordDeclaration. Its environment must
declare a State value named self of that same record type; that is the explicit
formal context binding, not a generated Rust-layout inference. Name expressions
resolve the nearest lexical let/quantifier binding, then an environment value.
An initializer/domain is outside its own new binding scope. The body/predicate
is inside it; sibling scopes do not leak. self and result keep their reserved
syntax meaning. Result availability is a later check; an absent explicit value
is a missing declaration.

Field resolution follows record shape in the selected owner environment. Option
value selection and collection element binding propagate existing ValueType
shape, including through grouping/let/conditional expressions when unambiguous.
No native integer type, unit or range proof is inferred by name linkage. Unknown
shape needed for a field selection refuses rather than inventing a declaration.
Enum literals preserve their selected model owner, enum and variant identities.
Scalar operators and Boolean roots remain untyped until FR-006 runs.

The initial profile has no formal object reference constructor or operation
declaration mapping. deref, reaches and pre/post operation clauses therefore
refuse unsupported_construct with their native span. Resolving syntax such as
pre(expr) does not prove that an observation exists; FR-006 must check it before
evaluation or IR lowering. This phase does not produce a reference-evaluable or
executable package and cannot emit a logical verdict.

## Bounds, ordering and diagnostics

Hard inclusive ceilings are 64 supplied environments, 64 imports, 256 clauses,
10,000 syntax expression nodes, 64 traversal depth, 1 MiB emitted canonical
bytes per environment and 8 MiB emitted canonical bytes across the inventory.
LinkLimits can lower each ceiling, including to zero; larger values clamp to
the hard ceiling. Check counts before iteration and canonical byte totals before
accepting another artifact. The byte budgets count emitted canonical content,
not allocator capacity; IR's constructor/input limits bound its internal work.
Traversal checks the next node/depth before visiting it. A refused request owns
no externally observable partially constructed LinkedPackage and caches no
request state for later calls.

The existing Diagnostic gains a link phase and codes missing_import,
stale_dependency, ambiguous_declaration, missing_declaration and
invalid_model_binding. Structured related formal declaration locations and a
canonicalization upstream diagnostic, when it fails, are retained as typed
sibling fields on `LinkingError` (ADR-011 §6.1: the foundation-layer
`Diagnostic` does not import `crate::linking` or IR types), not on
`Diagnostic` itself. Legacy diagnostics initialize related locations empty.
Resource exhaustion maps to native resource_exhausted, not a syntax error or Boolean.
CLI parse/format behavior remains its existing contract; no new CLI reader is
introduced by this library milestone.

## Qualification and policy

The public integration tests construct actual IR environments. Two distinct
RequirementRef owners exporting BoundedCounter under one repeated native alias
exercise real ambiguity, with both declaration sources retained. Exact source,
foreign digest, changed unused dependency, lexical scope, missing names,
unsupported mappings, boundary limits and prior-call/permutation controls are
independent expected cases. TC-020–024 qualify resolution; TC-025–029 remain
the separately planned static judgments. FR-013 adds the concrete API controls.

Consume the pinned IR dependency under its existing MIT OR Apache-2.0 grant.
Its exact serde 1.0.228 requirement is selected consistently for this crate;
existing Rust fixture audits and parser gates must pass after the dependency
change. Pin the native toolchain/local workflow recipe to Rust 1.98.1, with
Cargo rust-version 1.98. Hosted workflows remain workflow_dispatch only and are
not dispatched. New native implementation remains AGPL-3.0-or-later.

# Compiled protocol package wire v1

Normative data contract owned by [FR-042](../spec/functional/FR-042-publish-compiled-protocol-artifacts.md).
This is generated data, not another editable formal language. The source grammar,
selected definitions and producer contracts own the meanings represented here.

## Encoding and external selection

| Identity | Exact value |
| --- | --- |
| Payload wire | `quire.compiled-protocol/1` |
| Media type | `application/vnd.quire.compiled-protocol+json;version=1` |
| Schema/type | `quire.compiled-protocol.schema/1` / `CompiledProtocolPackage` |
| Byte encoding | `quire.protocol.compact-json/1` |
| Native numbers | `quire.protocol.numeric/1`, the closed [FR-038](../spec/functional/FR-038-encode-exact-protocol-numbers.md) objects |

The encoding is UTF-8 JSON without BOM, whitespace between tokens or a final
newline. Object members occur in the order specified below. Strings use the
existing `serde_json` 1.0.151 `CompactFormatter` spelling: quote/backslash and
backspace/tab/newline/form-feed/carriage-return use their short escapes; other
U+0000–001F scalars use lowercase `\u00xx`; other Unicode scalars, including `/`,
remain literal UTF-8. There is no Unicode normalization. No float conversion or
JSON-number semantic value participates. A reader re-encodes admitted fields
through the bounded Serde writer and compares every byte with the offered bytes.
Alternate whitespace, escaping, member order and numeric spelling refuse.

The existing `ix.artifact-ref/3-draft` reference is **external** to these bytes.
Its `kind` is `linked-package`, `wire` is `{identity:"quire.compiled-protocol",version:"1"}`,
and `digest` is `sha256:` plus lowercase SHA-256 of the complete canonical payload.
The payload contains no digest of itself. This is its one artifact-content binding;
there is no additional payload hash, semantic hash or result-JCS hash. The outer
composed-package `SemanticRef` retains its selected package definition, exactly
the declaration-family features below and `canonicalIdentity:null` under
[standard FR-130](ix://agent-ix/quire-specification/FR-130).

Reader input includes that independently selected expected artifact reference,
the accepted contract/baseline and producer selections, and exact supplied
dependency bytes/admitted producer views. The artifact cannot designate its own
acceptance, trusted seal or replacement inventory. Recomputing a modified
payload's seal does not change the expected reference. Authorizing a different
external reference is a new selection; digest integrity alone does not prove
authenticity or equivalence to native source. That correspondence comes from the
real compiler's admitted input/output path, not parser-free reconstruction.

## Closed records and ordering

The notation below specifies JSON data shapes, not executable helper code.
`{field:Type,...}` lists **all required members in canonical order**; no ellipsis
is an actual field allowance. `T?` means `T` or explicit JSON `null`, never omission.
`[T]` is an array. A tagged alternative `{kind:"tag",...}` is one closed object.
Unknown, duplicate or missing fields/tags and positional-array substitutes refuse.

`String` is a Unicode scalar string, possibly empty; `Name` is nonempty.
Names/identity components are at most 4,096 UTF-8 bytes; text/source content is
bounded by the reader's content limits. `U` is a bare JSON integer from zero
through 1,048,576 **inclusive**,
with no sign, exponent, fraction or leading zero. It is used only for structure,
source coordinates and array indices. Native values and authored bounds use
`Number`; `Integer` is its integer alternative. Counts such as repeat maxima,
delivery bounds, type maxima and temporal intervals are authored bounds, not `U`.
Compensation `maximum_attempts` is in `1..=9223372036854775807`. Compilation
and reading retain that finite signed-64 bound without expanding attempts;
consumer execution budgets limit actual work separately from the authored bound.
Revisions remain names in their explicit revision namespace, including exact
canonical decimal strings where the selected producer requires that spelling.

```text
Integer = {kind:"integer",decimal:String}
Number = Integer | {kind:"rational",numerator:String,denominator:String}
Revision = {namespace:Name,value:Name}
Wire = {identity:Name,version:Name}
Ref = {refVersion:"ix.artifact-ref/3-draft",kind:ArtifactKind,authority:Name,
       identity:Name,revision:Revision,digest:ByteDigest,wire:Wire}
Formal = {document:Name,revision:Revision}
Span = {start:U,end:U}
Locus = {source:U,span:Span}
ForeignLocus = {source:Ref,formal:Formal,span:Span}
Handle = {declaration:U,index:U}
SelectedDigest = {domain:Name,version:Name,algorithm:Name,value:Name}
Producer = {implementation:Name,revision:Revision,binary:Ref}
Dependency = {artifact:Ref,requires:[U]}
Definition = {identity:Name,revision:Revision,artifact:U,rules:[U],requires:[U]}
ProducerObject = {interface:U,kind:Name,authority:Name,identity:Name,
                  revision:Revision,digest:SelectedDigest}
Correspondence = {producer:ProducerObject,native:U,relation:U,exports:[U]}
Export = {kind:ExportKind,path:[Name],locus:ForeignLocus}
Model = {artifact:U,profile:Name,exports:[Export],correspondence:Correspondence?}
Source = {artifact:Ref,native:{identity:Name,revision:Name},path:Name,formal:Formal,text:String}
Features = {declarations:[Name],required:[Name],optional:[Name]}
CompiledProtocolPackage = {
 wire:"quire.compiled-protocol/1",media:"application/vnd.quire.compiled-protocol+json;version=1",
 schema:"quire.compiled-protocol.schema/1",type:"CompiledProtocolPackage",
 encoding:"quire.protocol.compact-json/1",numeric:"quire.protocol.numeric/1",
 contract:Ref,producer:Producer,baseline:Ref,language:{identity:"ix:native",edition:Name},
 package_definition:U,features:Features,sources:[Source],dependencies:[Dependency],
 definitions:[Definition],models:[Model],types:[Type],declarations:[Declaration]
}
```

`ByteDigest` and `ArtifactKind` use the exact shared-reference schema; no new
artifact kind is added. The external package reference is not copied inside the
payload. `contract`, `baseline`, producer `binary` and each dependency must match
their individually expected role/reference, including wire and revision namespace.
Definition artifact/rule indices select source-kind dependency bytes; model
artifacts select model-package-kind dependencies. An export kind is exactly
`scalar|enum|variant|record|field|object|reference|operation|relationship|population|component|endpoint`.
Paths and source loci must identify that kind in the admitted producer view;
unsupported authoritative exports prevent emission/admission of their dependents.
The native adapter derives a `population` export from an admitted `ObjectRole`,
using path `[record, universe]` and that role's original source locus. Its
`object` export retains path `[record]`; its `reference` export retains path
`[reference]`. These paths are relative to the exact selected model. Two object
roles sharing a universe label still have distinct population exports. This
binds a nominal population requirement; it supplies no population members or
completeness evidence. Within each declaration, the population requirement key
is the exact selected model, object record and original observation anchor;
binding names are opaque labels. Native admission derives the set from original
source owners; the parser-free reader checks the exact set justified by the
decoded input binders and anchored values against admitted models, refusing
missing or surplus pairs as `Invalid::Binding`. Derived binders retain initializer origins,
and selected values preserve their contributing origins. Nested record/reference
traversal visits each key once, including cycles, under the declared work limits.
Absent relationship/component/endpoint exports are not
supplied by declaring their tags. A producer correspondence, when required
by its interface, is mandatory and verified against its selected relation
artifact; `null` is allowed only for a directly admitted native model without
such a producer-domain mapping.

`package_definition` and every `profile` index `definitions`; `Definition.requires`
indexes definitions, while its `artifact`/`rules` and `Dependency.requires` index
dependencies. `Model.artifact`, `ProducerObject.interface`, `Correspondence.native`
and `Correspondence.relation` also index dependencies; correspondence `native`
equals that model's artifact index and its `exports` index that model's exports.
Interface/relation bytes must admit that exact producer object, native model and
export correspondence in their own domains; absent verification is unsupported.
Dependency and definition closure is acyclic and complete under the selected
package contract. Source/dependency revision or digest values are copied exactly
from the selected producer reference, never regenerated in the artifact domain.

Native source labels, external source-artifact revision and the explicitly
authored formal document/revision remain separate. Source bytes must hash to
the source artifact digest; spans use those original decoded UTF-8 bytes and
must be scalar-boundary aligned. Declaration loci are disjoint in each source;
all local loci lie inside their owning declaration. Foreign model loci are
validated against their separately supplied exact source bytes. No display
path, occurrence ordinal or equal name establishes authority.

Dependencies sort by `(kind,authority,identity,revision.namespace,revision.value,wire.identity,wire.version)`;
sources sort by their artifact key; definitions by `(identity,revision.namespace,revision.value)`;
models by dependency index; exports by `(kind,path)`; declarations by
`(source index,span.start,span.end)`. Comparisons use lexicographic UTF-8 byte
order. Equal immutable keys with different content refuse, as do repeated
entries even when equal. Types share an index only when their resolved forms
are exactly equal. Visit declarations in the order above, and within each
declaration visit binder types, non-null binding-requirement types, then value
types in their respective table order. Next visit the predicate result type,
or, for a protocol, channel message types followed by compensation attempt
types in table order. On the first visit to a type, assign its index before
recursively visiting its option value or sequence element type. Later visits
reuse that index; unreferenced types refuse. Declaration
local tables follow original source occurrence order, using original arena
index only to break an equal-span tie. Every index is rewritten consistently
after ordering. Semantic arrays preserve authored order: arguments, sequence
children, captures, query results and path segments. Set-valued features,
definition/rule/dependency references and joins sort uniquely by their typed key.
Sorting a set never inserts causality or reorders an executable sequence.

`features.declarations` is exactly the applicable set
`declaration.predicate`, `family.state`, `family.temporal`, `family.protocol`.
`features.required` always contains `quire.protocol.numeric/1` and
`quire.protocol.bindings/1`, adds `quire.protocol.values/1` for value nodes,
`quire.protocol.temporal/1` for temporal nodes and `quire.protocol.control/1`
for protocol control. These are closed wire capability names, separate from
selected semantic definitions and backend assessment capabilities. This version
has no optional feature: `optional` is `[]`; unknown entries refuse as unsupported
features. The package includes at least one protocol and the complete declarations
of its explicit source inventory; a filtered or partially admitted inventory
cannot use its full-package interpretation.

## Types, owners and pure values

`D` is a declaration index. `V/B/A/S/T/C/R/H/K` are `Handle` values indexing that
declaration's values/binders/anchors/scopes/temporal nodes/controls/roles/channels/
compensations respectively. Their wire shapes coincide; their expected kinds and
declaration owners do not. `X={model:U,export:U}` selects a model export;
`Q` is an index in the global types table. Values, lexical locals and controls
cannot point into another declaration. Cross-declaration calls/requirements use
`D` and explicit ordered arguments; no foreign `ExprId` becomes a local handle.

```text
Type = {kind:"boolean"}
 | {kind:"scalar",export:X,unit:Name?,representation:Representation}
 | {kind:"enum",export:X} | {kind:"record",export:X}
 | {kind:"object",export:X} | {kind:"reference",export:X,object:X,universe:X}
 | {kind:"option",value:Q} | {kind:"sequence",element:Q,maximum:Integer}
Representation = {kind:"integer",minimum:Integer,maximum:Integer}
 | {kind:"rational",numerator_minimum:Integer,numerator_maximum:Integer,maximum_denominator:Integer}
 | {kind:"text",maximum_scalars:Integer}
Scope = {parent:S?,locus:Locus}
Anchor = {kind:AnchorKind,owner:Handle?,binding:U?,locus:Locus}
Binder = {name:Name,kind:BinderKind,type:Q,scope:S,anchor:A,initializer:V?,locus:Locus}
Origin = {kind:"independent"} | {kind:"anchor",anchor:A} | {kind:"selected",value:V}
Value = {original_expression:U,locus:Locus,operator_locus:Locus?,type:Q,
         profile:U,scope:S,anchor:A,origin:Origin,operation:ValueOperation}
ValueOperation = {kind:"boolean",value:Boolean} | {kind:"number",value:Number}
 | {kind:"text",value:String} | {kind:"enum",variant:X} | {kind:"read",binder:B}
 | {kind:"group",value:V} | {kind:"field",base:V,field:X}
 | {kind:"unary",operator:Unary,value:V}
 | {kind:"binary",operator:Binary,left:V,right:V}
 | {kind:"if",condition:V,then_value:V,else_value:V}
 | {kind:"let",binder:B,initializer:V,body:V}
 | {kind:"pre",value:V,anchor:A}
 | {kind:"call",predicate:D,arguments:[V]}
 | {kind:"size",collection:V,result:Q} | {kind:"contains",collection:V,member:V}
 | {kind:"query",operator:Query,binder:B,collection:V,body:V,result:Q}
 | {kind:"parent",reference:V,edge:X,universe:X}
 | {kind:"reaches",start:V,target:V,edge:X,universe:X}
```

`Boolean` is JSON true/false. `Unary` is `not|negate|present|value|deref`;
`Binary` is `implies|or|and|equal|not_equal|less|less_equal|greater|greater_equal|add|subtract|multiply|rational_divide`;
`Query` is `forall|exists|filter|map|count|sum`. Integer division/remainder/modulo
have no admitted variant. Their recognized source syntax still receives its
source-profile refusal. A type's representation and exact unit must equal the
selected export's actual IR/native representation; equal bounds never merge
nominal types. Integer bounds are signed-64 and ordered; rational denominator
maximum is 1..i64::MAX; sequence maximum retains its admitted producer bound.
Each number also satisfies its node/field-specific type domain.

The wire `parent` alternative has no adopted composed source operator or
distinct result contract; the current native adapter refuses it as
`Unsupported::Feature`. Authored field access/dereference and positive-length
`reaches` retain their existing selected graph meanings.

A `Type.reference` triple must select the reference, object and population
exports of one actual admitted object role in one model. Matching carrier types,
equal names or equal universe labels in another role/model cannot join that
triple. Native emission retains separate population and closure requirements for
the exact object/universe and original reference evaluation anchor, including
pre-state and captured origins. The closure requirement depends on that
population requirement. These are required consumer inputs; static compilation
neither reads future observations nor establishes membership, closure or truth.

Each declaration has one root lexical scope; every other scope reaches that
root through an acyclic parent chain. Binder initializers, value children and
non-self origin references form an acyclic graph. `Origin.Selected` may name its
own value: the native checker uses this marker for mixed immutable provenance,
not recursive evaluation. It retains the same owner/range checks; an actual
self operand or a cycle through different origin nodes still refuses.
Reads select a visible binder using that value's admitted `scope` handle and
the scope parent chain to the selected binder's `scope`; source-locus containment
does not grant lexical availability. Scope loci retain original source provenance
and may include a query binder token and collection even though the binder is
available only in its body. Reads retain the initializer's immutable anchor.
Wrapper-type indices are acyclic; nominal recursion remains an explicit producer export,
not a cyclic wrapper table. All handles must be in range and match their field's
kind and declaration owner, including unused entries.

`AnchorKind` is `predicate|current|invocation_input|invocation_pre|invocation_post|activation|temporal_instant|protocol_instant|fifo|registration|compensation_activation|retry|recovery|control|finish`.
Owner is null only for declaration-level anchors; otherwise it points to the
owning channel, compensation or control as required by that kind. Binding names
the applicable runtime requirement, not a concrete observation. `BinderKind` is
`parameter|input|trigger|capture|let|query|self|result|invocation_parameter|event|finish|fifo|forward_effect|compensation_trigger|earlier_attempt|later_attempt|recovery`.
Scopes, initialization order, visibility and immutable origins must agree with
actual binding. `let` initializes once; `if` and Boolean operators retain lazy
evaluation. `pre` changes eligible reads inside its operand without replaying an
outer initializer. Queries preserve ordered duplicates, scoped binders and
sum's prefix obligations. This value graph is a faithful checked native
evaluation graph; it is neither serialized `TypeReport` nor the prover's
symbolic Boolean witnesses/abstracted object inputs.

## Declarations, protocol control and recovery

```text
Declaration = {name:Name,locus:Locus,requirement:{package:Name,identity:Name,revision:Revision},
 clause:Name,execution:Execution,profile:U,requires:[D],scopes:[Scope],anchors:[Anchor],
 binders:[Binder],values:[Value],temporal:[Temporal],bindings:[BindingRequirement],body:Body}
Execution = {kind:"initialization",name:Name} | {kind:"handler",name:Name}
 | {kind:"pre",operation:X} | {kind:"post",operation:X}
Body = {kind:"predicate",parameters:[B],result:Q,root:V}
 | {kind:"state",clause_kind:"invariant"|"pre"|"post",context:X,operation:X?,root:V}
 | {kind:"temporal",input:B,clock:U,activation:Activation,captures:[B],root:T}
 | {kind:"protocol",input:B,activation:Activation,captures:[B],roles:[Role],
    relationships:[Relationship],channels:[Channel],compensations:[Compensation],
    temporal_requirements:[D],controls:[Control],causal_edges:[CausalEdge],run:C,finish:Finish}
Activation = {kind:"origin",anchor:A} | {kind:"each",trigger:B,guard:V?,anchor:A}
Interval = {lower:Integer,upper:Integer}
Temporal = {original_node:U,locus:Locus,operation:TemporalOperation}
TemporalOperation = {kind:"constant",value:Boolean} | {kind:"holds",value:V}
 | {kind:"group",value:T} | {kind:"unary",operator:TemporalUnary,interval:Interval?,value:T}
 | {kind:"binary",operator:TemporalBinary,interval:Interval?,left:T,right:T}
Role = {name:Name,model:X,instance:U,locus:Locus}
Relationship = {name:Name,model:X,binding:U,locus:Locus}
Channel = {name:Name,from:R,to:R,message_type:Q,ordering:Ordering,delivery:Interval,
           message:U,send:U,receive:U,delivery_instance:U,locus:Locus}
Ordering = {kind:"unordered"} | {kind:"fifo",binder:B,key:V,anchor:A}
Control = {name:Name,original_node:U,locus:Locus,operation:ControlOperation}
ControlOperation = {kind:"sequence",children:[C]}
 | {kind:"choice",owner:R,visible:[V],cases:[{label:Name,guard:V,body:C,locus:Locus}]}
 | {kind:"parallel",branches:[{label:Name,body:C,locus:Locus}],join:[U]}
 | {kind:"repeat",owner:R,visible:[V],maximum:Integer,guard:V,body:C,exhausted:C}
 | {kind:"await",after:AwaitAnchor,profile:U,clock:U,within:Interval,event:C,then_body:C,timeout:C}
 | {kind:"event",event:Event,binder:B,related:[Related],constraint:V}
 | {kind:"check",profile:U,value:V}
 | {kind:"commit",owner:R,binder:B,constraint:V,instance:U}
Event = {kind:"send",channel:H} | {kind:"receive",channel:H,send:C}
 | {kind:"attempt",owner:R,operation:X,contracts:[D],instance:U}
 | {kind:"effect",attempt:C,instance:U}
 | {kind:"event",owner:R,compensation:K?,instance:U}
Related = {relationship:U,from:V,to:V,locus:Locus}
AwaitAnchor = {kind:"event",node:C} | {kind:"compensation",compensation:K}
CausalEdge = {kind:EdgeKind,owner:C,from:{node:C,port:Port},to:{node:C,port:Port},maximum:Integer?}
Finish = {name:Name,binder:B,constraint:V,closure:U,locus:Locus}
Compensation = {name:Name,forward_effect:C,forward:B,owner:R,operation:X,profile:U,
 clock:U,registration_anchor:A,registration_instance:U,registration_captures:[B],
 trigger:B,guard:V,activation_anchor:A,activation_captures:[B],within:Interval,
 maximum_attempts:Integer,attempt_type:Q,earlier:B,later:B,retry:V,attempt_instance:U,
 effect_instance:U,commit:C?,recovery:B,recover:V,recovery_bindings:[U],locus:Locus}
BindingRequirement = {name:Name,kind:BindingKind,type:Q?,authority:Ref,contract:U,
 model:X?,subject:Subject,anchor:A,scope:S,relation:U?,requires:[U],locus:Locus}
Subject = {kind:"declaration",declaration:D} | {kind:"role",role:R}
 | {kind:"channel",channel:H} | {kind:"control",control:C} | {kind:"compensation",compensation:K}
```

`TemporalUnary` is `not|eventually|always|once|historically`;
`TemporalBinary` is `implies|or|and|until|release|since|triggered`.
Boolean temporal operators have null intervals; bounded operators require
ordered nonnegative intervals under the selected clock/range contract.
Profile fields index exact definitions and preserve nested check/await/recovery
selections. Operation references select actual operation exports and their
authored execution anchors; attempt contracts select only matching pre/post
declarations. Predicate roots and every guard/constraint/retry/recovery root
are Boolean and total; required calls retain the callee's own profile/owner.

`Declaration.requires` is exactly its direct predicate, operation-contract and
temporal declaration dependencies, including unused syntax. Its transitive
semantic dependency graph is acyclic; bounded control progress is not an edge
in that graph. Calls preserve exact arity, ordered argument types and the
callee's independently admitted profile. `Related.relationship` indexes the
owning protocol's relationships; parallel `join` indexes its authored branches.

`Port` is `enter|exit`; `EdgeKind` is `sequence|branch|join|repeat_progress|await_success|await_timeout`.
Edges must equal the relation derived from the closed control structures, not an
arbitrary graph supplied by a producer. `maximum` is non-null only on
`repeat_progress`, equals its owner repeat's nonnegative maximum and advances that
repeat's iteration counter once before re-entry. Removing those explicitly
bounded progress edges leaves an acyclic graph; finite unfolding is acyclic.
Nesting is finite and structural children have one owner. Parallel ordering is
per branch, with `join` indexing exactly the authored branch completion set;
array order, timestamps and FIFO on another channel add no edges. Await targets,
receive/send pairs, effects/attempts and compensation/commit references are
kind-checked. Choice ownership, branch labels, non-overlap and decision-visible
facts are compiler family-admission obligations under
[FR-042](../spec/functional/FR-042-publish-compiled-protocol-artifacts.md),
consuming the accepted standard's FR-050–059. They are not assumed from graph
shape or a reader-admitted package. The constructor-private producer admission
must establish them before emission; the reader verifies the derived data and
its independently selected producer, not the source proof.
Choice guards are also collectively exhaustive over admitted decision inputs.

The native observed-Boolean choice fragment uses atoms from actual Boolean fields
of received or same-owner attempt/domain-event record/object binders. An atom
retains the exact event binder, selected model field/export and original
observation anchor;
repeated reads or immutable aliases reuse that identity, while distinct
observations remain distinct. The receive's actual channel `to` role, attempt's
role or domain event's role must resolve to the choice owner, and the existing
admitted source scopes must establish guaranteed causal availability at the
decision. A preceding sequence receive, or a guaranteed receive after an
all-branch join, can qualify. A sibling receive before that join, a branch-only
receive after its choice, or an await-success receive outside its success
path cannot acquire availability from textual order or timestamps.
The same availability rule applies to attempt results, including all-joined
attempts and unavailable parallel siblings. Equal role model types cannot replace
exact role identity. Admitting an attempt-result Boolean field establishes no
operation success or business effect; operation/contract admission remains
required.
A compensation-qualified domain event keeps its exact registration association
as an independent binding prerequisite. Its Boolean atom establishes no
registration, activation, operation success, effect, send or commit observation.
Ordinary and qualified domain events share the same exact-role and causal
availability rule; all existing family/binding checks remain required.

`visible(...)` declares the information obligation. Each entry in this fragment
is a direct eligible received or same-owner attempt/domain-event Boolean field,
transparent grouping/immutable alias to that exact atom, or a closed Boolean
constant; every guard atom must occur in that validated atom set. A composite
visible expression such as `a and b` does
not disclose its individual atoms and remains `Unsupported::FamilyProof`.
Listing an arbitrary input or another role's observed value grants no visibility.
Supported guard formulas are Boolean literals and fields, grouping, `not`, `and`,
`or`, `implies`, Boolean `=`/`!=`, Boolean conditionals and immutable `let` aliases.
Alias/capture tracing uses the existing source-owned initializer and observation
provenance, with the same availability checks; it cannot manufacture a prior
observation. All original operands are checked, including unused initializers
and conditional branches.
Numeric comparisons, queries, arbitrary input atoms and callee-body expansion
remain outside this proof fragment. A bounded repeat's guard may use this same
fragment over observed Booleans; its atoms carry the repeat's own anchor, must be
owned by the role named by `by`, and must appear in that repeat's visible set. A
guard atom established only inside the repeat body has no availability at the
decision instant and stays outside the fragment.

Family admission proves that exactly one case guard holds for every valuation
of a conservative abstraction in which distinct observation atoms are independent.
This establishes coverage and non-overlap even when actual inputs are correlated.
A failing abstract valuation does not establish an admissible runtime input:
failure to prove the partition returns `Unsupported::FamilyProof`, without a
fabricated concrete overlap/hole. Fully closed decisions retain `Invalid::Control`
for an evaluated overlap or hole. Missing supported visibility/provenance also
leaves family proof unestablished; earlier source/type/scope refusals retain their
own causes. Closed classification covers every original operand and initializer;
short-circuit truth and unselected branches cannot erase a prerequisite.
Existing IR `check_expression` establishes definedness, not truth of
the partition. This private Boolean proof changes neither the emitted original
guards/AST handles nor the wire schema and supplies no runtime observations.
When such a choice supplies a continuing repeat body's progress proof, every
case feasible in the abstraction must guarantee observable progress; one
progressing case is insufficient. If the continuing body's progress remains
unproved, admission returns `Unsupported::FamilyProof`; other guaranteed progress
in the body retains its ordinary sequence/parallel meaning.

At each repeat decision, false exits normally; true below the maximum enters
the body, and true at the maximum enters the exhausted child once. Maximum zero
therefore admits only the normal/exhausted paths, without entering the body.
Every continuing body path needs observable progress; checks or empty groups
alone do not establish it. All parallel branches appear in `join` exactly once.
An await anchor selects an exact preceding event or a compensation activation,
never a group or guessed latest occurrence. Its matched child is receive, effect
or domain event only. The upper endpoint is inclusive; timeout requires the
selected progress/completeness authority, not a false constraint or absent datum.
Its static association is a `Progress` or `Closure` binding requirement with
`subject: Control(await)` and `requires` containing that await's clock **binding
index**. Its `contract` and `authority` retain the independently selected static
definition bytes; neither is a runtime observation. The reader refuses a missing
association, foreign subject or absent clock dependency as `Invalid::Binding`.
The compiler establishes the selected contract's meaning during family admission;
the consumer supplies and validates actual progress/completeness inputs later.
The finish constraint is reached after the run root exits and the selected
join/completion requirements close; a source declaration position is no substitute
for that termination dependency.

The exact structural edge expansion below uses `in/out` for enter/exit ports;
the edge owner is the expanded control. Edges are alternatives selected by that
owner's admitted guard/deadline rule, except parallel joins which require all
branches. They do not assert that mutually exclusive branches both occurred.

| Control | Required edges (kind) |
| --- | --- |
| Event/check/commit; empty sequence | in → out (`sequence`) |
| Nonempty sequence | in → first.in; each child.out → successor.in; last.out → out (`sequence`) |
| Choice | in → each case.in (`branch`); each case.out → out (`join`) |
| Parallel | in → each branch.in (`branch`); every branch.out → out (`join`) |
| Repeat | in → out, body.in or exhausted.in (`branch`); body.out → in (`repeat_progress`, maximum N); exhausted.out → out (`join`) |
| Await | in → match.in and match.out → then.in (`await_success`); in → timeout.in (`await_timeout`); then.out and timeout.out → out (`join`) |

An event await anchor additionally contributes anchor.out → await.in (`sequence`);
a compensation anchor retains its activation-binding dependency instead. A
receive contributes send.out → receive.in and an effect contributes attempt.out
→ effect.in (`sequence`, owner the dependent event). Other registration/commit
dependencies remain in their typed compensation/binding records. The edge array
sorts uniquely by `(owner,kind,from.node,from.port,to.node,to.port)`; no other edge
is admitted. A repeat's progress edge is unreachable when N=0. The reader checks
these finite edges without unrolling runtime iteration identities.

`BindingKind` is `workflow_instance|role_instance|participant|component|endpoint|message|send|receive|delivery|attempt|effect|compensation_registration|compensation_attempt|compensation_effect|commit|snapshot|invocation|population|relationship|clock|observation|progress|closure|capture`.
Each is a typed **requirement for a later binding**, not a runtime instance or
observation embedded in a static package. `U` fields named instance, clock,
closure, binding or recovery_bindings index the owning declaration's binding
requirements. Dependency/contract/relation indices select the appropriate
immutable external contract bytes; relationship indices select the owning
protocol's relationship table. Requirements retain exact producer authority,
role, type, anchor, scope and closure/completeness dependencies. B/D/F's admitted
request/producer interfaces supply actual instance identities later. Two
workflows may bind one provider while their role/message/attempt/effect subjects
remain distinct. A delivery never becomes an effect, and a component/endpoint
never becomes a role instance through equal spelling or hash bits.

`BindingRequirement.requires` selects bindings of that same declaration, and
its `contract`/non-null `relation` select dependencies. Binding requirements are
acyclic. `type` and `model` may be null only when the selected binding contract
defines a non-value/non-model role, such as a clock or closure authority; null
cannot erase a required instance type or authoritative model export.

One explicit exception represents a compensation effect's instance identity
without inventing a payload: `kind:"compensation_effect"` may have `type:null`
only with non-null `model` selecting that compensation's exact `operation`
export. It uses the owning compensation subject and retry anchor, and requires
exactly that compensation's `attempt_instance`. This is neither an untyped
observation nor a substitution of the attempt, operation result or recovery view
for effect evidence. F must later admit the distinct actual effect subject and
its typed signal/value mapping under the unchanged selected observation-binding
contract; the null type does not waive that requirement. Every compensation
effect passes the exact operation/obligation identity checks. The current native
profile supplies no authoritative effect-payload selector or correspondence:
adding a non-null `type` to an otherwise valid effect instance returns
`Unsupported::Export`. A foreign operation remains `Invalid::Binding`, even
with a type present. A future typed effect view needs that explicit producer
interface; an attempt, operation-result or recovery type cannot supply it.
Ordinary `effect` bindings still require both type and model.

The registration, activation observation, attempt and effect bindings select the
registered observation-binding definition and retain the same compensation
subject. Registration uses the registration anchor and requires exactly the
paired forward effect and owner role-instance bindings. The activation
observation uses the compensation-activation anchor, retains the declared trigger
type/model and requires exactly the registration binding. The attempt uses the
retry anchor, declared `attempt_type` and exact operation export, and requires
exactly the activation observation and owner role-instance bindings. Registration,
activation and retry anchors belong to this compensation and their `binding`
members select those respective records. The effect shares that retry anchor
and requires exactly the attempt binding. These prerequisites preserve the
obligation/activation/attempt/effect distinctions without supplying observations.
Missing, foreign, cross-wired or differently contracted links refuse.

The compensation clock belongs to the same subject at its activation anchor,
requires the activation observation and selects the exact temporal profile's
contract and authority. Its recovery snapshot belongs to that compensation's
recovery anchor, has the recovery binder's exact type/model, requires the
activation observation and selects the observation-binding contract. The
recovery anchor names that snapshot. Recovery progress and closure each select
the progress contract and require exactly the clock, effect instance and
snapshot. `recovery_bindings` contains exactly that snapshot, progress, closure,
and the declaration-owned population/closure pairs for the recovery view and
its contributing captured origins. Other obligations' clocks or recovery
records, missing prerequisite edges and unrelated population pairs refuse.

Recovery origins follow one shared dependency rule in native emission and reading.
Start with the owning recovery anchor and traverse from `recover` through every
original value-operand edge, read-binder initializer edge and non-self
`Origin.Selected` edge. Each reachable `Origin.Anchor` adds its original anchor;
a self-selected origin is only a provenance marker and adds no edge. Reachability
uses these handles, never source-span containment or proof simplification of
original operands. Calls contribute their argument values, without traversing
callee-owned graphs or inventing captures. Select exactly the declaration-owned
population/closure pairs at the resulting anchors; an origin naming another
compensation's phase anchor refuses. Existing model validation
still checks their nominal object/universe identities and pair integrity. This
rule derives static requirements without future observations or closure claims.

At runtime each authored compensation obligation registers only after its paired
successful effect, once for that obligation and effect identity. Distinct
compensation definitions may name the same forward control: the standard's
FR-056 registers exactly the obligations paired with an effect, not a single
global compensator. Registration and activation captures have separate anchors; retries
have distinct attempt identities under the same obligation. Null commit means
the authored `never` boundary, not unknown commit input. A non-null commit
forbids the selected subsequent runtime registration/recovery under standard
FR-057. Its presence does not forbid declaring a recovery relation. The static
reader checks the commit's kind and owner; the consumer checks actual ordering
against bound observations. Recovery retains the
actual predicate, target captures and required population/relationship authorities;
operation success alone is not restoration. Full and partial recovery therefore
remain different authored relations. Missing future observations are not static
admission failures; missing required static binding contracts are.

## Bounded read and emission

The current native adapter admits `ix:native` edition `1-draft` and registered
definitions with actual `NativeModel` views. A different edition cannot satisfy
that definition selection (`Invalid::Definition`). Producer/native
correspondence and relationship, component and endpoint exports remain explicit
`Unsupported` prerequisites until their authoritative producer adapters exist;
raw dependency bytes or object-universe names cannot substitute for those
adapters. Reference/population exports use the actual admitted native object
role as specified above. These remaining refusals keep full FR-042
emission and handoff acceptance open.

The Rust reader accepts bytes plus an explicit expected selection. It hashes
the bounded offered bytes against the external seal, decodes the closed shape,
checks header/accepted identities and dependency bytes before interpreting their
dependent records, validates numbers and table/reference/scope/type/control
invariants, then performs bounded canonical byte comparison. Source text is
hashed/indexed for original loci; it is never parsed. External producer digests
are verified only by their selected domain adapter. Profile/rule closure, source
inventory and every retained dependency must match the expected inputs exactly.

The public Rust refusal vocabulary is part of this boundary; callers inspect
variants and fields, never `Display` text. It is not a serialized error protocol.

| Result class | Meaning and typed detail |
| --- | --- |
| `Error::Allocation` | A bounded reservation failed. |
| `Error::Json { line, column }` | Closed JSON shape or syntax failed at the original byte-oriented position. |
| `Error::Numeric(NumberError)` | `NonCanonicalDecimal` or `ComponentOutOfRange` identifies `Decimal`, `Numerator` or `Denominator`; `NonPositiveDenominator` and `UnreducedRational` retain the exact numeric refusal. |
| `Error::Invalid(Invalid)` | Recognized data violates the selected contract; discriminants below identify the violated invariant. |
| `Error::Unsupported(Unsupported)` | `Wire`, `Feature`, `Definition` or `Profile` lacks an interpretation; `ProducerCorrespondence` or `Export` lacks the required authoritative adapter; `FamilyProof` means the native family prerequisites are not established. |
| `Error::Incomplete(Exhaustion)` | `dimension`, successful prior `used`, next `requested`, effective `limit` and available original `locus` identify the unaffordable operation. |

`Invalid` distinguishes `Selection` (independent identity mismatch), `Seal`
(exact-byte mismatch), `Name`, `StructuralInteger`, `WrongNumericKind`,
`NumericDomain`, `Inventory` (missing/extra entries), `Order`, `Duplicate`,
`Dependency`, `Definition`, `Model`, `ForeignLocus`, `Locus`, `Reference`
(dangling handle), `Owner`, `Scope`, `Type`, `Profile`, `Call`, `Binding`,
`Control`, `Cycle`, `Feature` (inconsistent recognized capability inventory),
`Canonical` (byte spelling/order) and `Encoding` (writer failure).
Resource dimensions are `PayloadBytes`, `OutputBytes`, `SourceBytes`,
`ContentBytes`, `Sources`, `Dependencies`, `Definitions`, `Models`,
`Declarations`, `Entries`, `References`, `ByteWork` and `Depth`.

The first failed check in the stated pass order wins. In particular, after the
seal and closed-shape checks, an unknown wire version refuses as
`Unsupported::Wire` before dependency inventory checks, even if the latter would
also fail. A failed prerequisite prevents interpretation of dependent records.
Package-wide canonical output has no native source locus; it must not inherit
the last visited value's region. Source-owned validation retains its actual locus.

The following independent dimensions each use the listed default and hard maximum:
8 MiB payload/output bytes;
1 MiB per source; 8 MiB decoded source/string content; 10,000 sources/dependencies/
definitions/models/declarations each; 100,000 aggregate table entries; 1,000,000
reference/edge/type visits; 64 MiB total copied, hashed, indexed or compared byte
work; depth 64. Each logical wire array entry/object member and each compiler-owned
index entry charges an entry before allocation; each subsequent reference/edge/type
visit charges again. These are content/work limits, not measurements of allocator
capacity or Serde's temporary buffers. The decode census bounds logical entries,
decoded content and depth before typed deserialization; temporary Serde storage
remains bounded by that input. Content decoding,
hashing, indexing, reference validation and canonical writing charge before work.
Bounded-repeat graphs are not eagerly unfolded. Lowered limits, including zero,
are honored; overflow or the next unaffordable step yields typed incomplete with
dimension, prior usage, requested work and available locus, never an admitted
partial package. Each report retains effective limits and successful usage in
the versioned `quire.protocol.artifact-work/1` accounting contract. Input,
dependency, decode, link and canonical passes each use fresh traversal state;
their successful work accumulates in this invocation's counters, and retry
starts a new invocation. Retained diagnostic-list entries are charged before
allocation. The reader's single inline terminal refusal needs no entry allocation
and cannot be replaced by exhaustion of a diagnostic budget.

Native Boolean choice proof uses these same counters: created atom, formula and
work records charge `Entries`; each provenance/reference/operand and case or
valuation inspection charges `References`; byte traversal and explicit traversal
depth retain their existing dimensions. Reservations and visits charge before
work. Exhaustion remains `Incomplete` with the original decision/expression locus,
never a proved partition or an abstract counterexample. No public proof platform
or separate unbounded truth-table allocation is introduced.
Private read indexes and lowering records are rebuilt per dynamic choice and
charged each time, including transient records discarded after that choice.
`Entries` is cumulative creation work, not peak live memory. A sequence of
individually admissible choices may exhaust the shared invocation budget and
returns `Incomplete` without partially admitted output.

Production emission consumes the actual fully admitted family graph, original
source/formal correspondence and exact definitions/models/runtime requirements.
It does not accept freely constructed wire records, type reports or proof graphs
as that authority. The emitter independently checks references and exact model/
type/profile correspondence while deriving the records, then uses the existing
Rust numeric codec, bounded Serde writer and SHA-256 machinery. Failure leaves
the partial compilation report intact but returns no accepted full-package
artifact or external seal.

The Rust path is `checking::composed::proofs::discharge` →
`protocol_artifact::native::admit` → `native::emit`. Admission borrows the actual
proof report and independent `native::Selections`: original formal sources,
exact source/dependency references and bytes, admitted models, contract, baseline
and producer. Each selected source retains its opaque external artifact revision
separately from the native/formal revision. The caller explicitly selects
namespaces for formal-source, requirement and registered-definition revisions;
a registered definition's semantic revision comes from its actual registered
bytes, not the source artifact's revision label. Successful admission owns a
private `FamilyAdmission`; `emit` takes that type and returns `EmittedPackage`
bytes and their raw-byte digest under a fresh invocation's limits. An arbitrary
`wire::Package`, reader result or proof witness cannot be passed as this authority.

The implemented path preserves original expression/control arenas, declaration
owners, lexical binder identities and evaluation anchors across predicate,
state, temporal and protocol families. It requires completed native typing and
definedness before family checks. Supported constant decisions establish their
coverage and non-overlap directly; continuing bounded loops must establish
observable progress. The native path lowers `size`, `contains`, `forall`,
`exists`, `filter`, `map`, `count` and `sum` from their original AST occurrences
after supported query definedness completes. It retains collection/body order,
query binder identity, result domain and provenance. The admitted per-value
`scope` handle and its parent chain govern binder availability: the query body
uses the binder's scope, while its collection retains the surrounding scope.
The scope locus is source provenance, not an availability oracle. Proof-side
symbolic or empty-domain simplification does not replace the emitted query with
a witness, constant or unrolled graph.
Sum discharge supports integer and denominator-one rational prefix domains;
broader rational sum-domain transfer remains explicitly unsupported unless the
sum is statically empty. Static compensation registration/activation, retries,
commit/never and authored full/partial recovery requirements now emit with their
original identities and dependencies. General symbolic choice/visibility proofs
and absent authoritative producer exports still refuse explicitly. Runtime
recovery remains a consumer judgment. These are remaining implementation obligations,
not a reduced language specification. The caller retains the upstream compilation
reports when family admission refuses; no partial package is emitted. A successful
native emission followed by independent `read` is compiler-to-reader evidence;
the actual `quire-protocol` consumer handoff remains separately required.

Channel delivery bounds are per-send cardinalities, not elapsed time. The native
adapter retains separate message, send, receive and delivery requirements and
channel-local FIFO key expressions. Event binders remain records as specified by
the native surface. Its direct payload path requires the same exact model type
for the channel message and send/receive record. A different record/payload pair
needs the authoritative observation correspondence and currently refuses as
`Unsupported::Export`; no record field is guessed as the payload. This boundary
does not forbid scalar channel declarations or grant an observation identity from
record contents. Concrete workflow/message/delivery correspondence remains a
consumer input requirement.

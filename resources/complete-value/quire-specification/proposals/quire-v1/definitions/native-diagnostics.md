# Native diagnostic interpretation candidate

Interpretation identity: `quire.native.diagnostics/v1`; revision: `1-draft.3`.
**The interpretation role is accepted by FS01; this semantic revision remains
proposed and is not adopted or fully implemented.** This supplies the versioned catalog
required by FR-047. It interprets native producer diagnostics in their existing
result envelopes; it is not a clause profile, payload wire or evidence store.

## Selection and compatibility

The producer response selects this catalog's exact identity, revision and byte
digest separately from its language/profile, payload wire and backend selections.
Changing diagnostic wording, display paths or this response catalog does not
change a checked clause's static meaning. A different diagnostic interpretation
does change the response's interpretation/provenance. No clause `using` alias
can select this catalog or acquire source permissions from it.

The following immutable compiler sources establish the retained code spellings
and broad meanings. Only `Code::as_str()`'s spellings and the code/meaning table
are selected; their implementation, envelope layouts and status-aggregation
policy are not normative dependencies of this catalog. The typed refinements
below supply this candidate's additional requirements.

| Selected inventory | Exact source at quire-spec-language revision `f444d03c06539a6cd0ada6be4ae099b54466d9d9` | SHA-256 of file bytes |
| --- | --- | --- |
| Native Code spellings | [src/diagnostic.rs](https://github.com/agent-ix/quire-spec-language/blob/f444d03c06539a6cd0ada6be4ae099b54466d9d9/src/diagnostic.rs) | `c73eaf5a543c67d2187b976846c7253db9af4cc4f9bd23dc535921e91fbf136d` |
| Broad code meanings | [docs/native-error-codes.md](https://github.com/agent-ix/quire-spec-language/blob/f444d03c06539a6cd0ada6be4ae099b54466d9d9/docs/native-error-codes.md) | `c447024767a3ebac8085383658547130d04741894aefae6a0afcb2e3683905a2` |

Hyphenated and underscored spellings stay exact; they are not normalized aliases.
A current compiler enum or its `non_exhaustive` annotation cannot silently extend
this immutable selection. A new required code or cause variant needs a new
catalog revision and explicit reader support. An unknown selection, code or
required variant refuses interpretation of that diagnostic while retaining its
original bytes and available provenance. It cannot be recast as a known failure,
an empty diagnostic list or a successful assessment.

Historical responses retain their original contracts. A legacy code plus message
does not establish the new typed cause by substring matching. Where the old
response lacks distinguishing data, its interpretation remains legacy/incomplete
for this mapping; preserve it without fabricating a stronger new-catalog claim.

## Common structured context

Every diagnostic retains its observing producer stage and its catalogued code.
A stage binds the exact selected producer-interface identity, revision and digest
plus one exact label from that interface's closed stage vocabulary. A wrapper
retains the originating pair and cause instead of replacing either with its own
display stage. This catalog does not rename historical stage labels or create a
competing global pipeline enum.

Required context is the affected request/subject, exact selections that were
available, original source region and typed input location where known. An
authored source span, a JSON field/index path and a runtime object/value path
remain different location kinds. Unavailable context is explicitly unavailable;
byte zero, an empty path or a name-search match cannot masquerade as a located
failure. Related declarations and upstream diagnostics retain their own exact
source/catalog identities rather than being flattened into a string.

The producer determines code and cause from typed variants at the failing
operation. A cause is a closed discriminated value with the payload required by
its selected variant, not a free-form string map. Serialization emits its stable
tag; formatting/localization reads it and does not select it. Expected/actual
values are typed selections, kinds, domains or locations, not English summaries.

## Required distinguishing causes

These are mandatory refinements when the named failure occurs. A producer cannot
choose a broader listed alternative to discard a distinction that it knows.
The selected producer interface may carry additional nonsemantic explanatory
context, but that context cannot replace a required cause field or variant.

| Code or code family | Closed cause distinction and required payload |
| --- | --- |
| `invalid_syntax` | `unexpected-token`, `unexpected-end`, `invalid-token` or `invalid-escape`; retain the actual token/source span when available and the expected syntactic category. An ungrammatical index/slice is not a recognized profile extension. |
| `unsupported_construct` | `declaration-form` or `expression-form`; retain the recognized form, owning declaration and exact selected profile that prohibits it. Presence in unreachable syntax does not erase the refusal. |
| `unknown_language`, `unknown_edition`, `unknown_profile` | `unsupported-selection` or `wrong-selection-role`; retain the supplied selection and required role. A known package/diagnostic interpretation used as a clause profile is the latter. |
| `unknown_wire` | `unsupported-wire`; retain the actual selected wire and expected artifact role, even if its outer reference envelope is well formed. |
| `unknown_required_feature` | `unknown-feature` or `unsupported-feature`; retain the feature and selected consumer/profile scope. A recognized name is not implementation support. |
| `missing_import`, `missing_declaration` | `missing-selection` or `missing-name`; retain the requested typed dependency/name, owning scope and referring locus. |
| `stale_dependency`, `source_digest_mismatch` | `revision-mismatch`, `byte-digest-mismatch` or `digest-domain-mismatch`; retain expected and actual typed selections/digest domains where available. A missing artifact cannot supply an invented actual digest. |
| `ambiguous_declaration` | `ambiguous-name` or `conflicting-authority`; retain the visible name/clause and the distinct conflicting declaration/authority selections and loci. |
| `invalid_package` | `malformed-json`, `missing-member`, `duplicate-member`, `unknown-member`, `wrong-value-kind`, `invalid-value`, `definition-cycle`, `conflicting-definition`, `incompatible-definition` or `feature-set-mismatch`; retain the actual field/index path or dependency edges/feature sets appropriate to that variant. Under `quire.model.complete/v1`, `definition-cycle` also covers a call-graph cycle through a dispatch edge (FR-151) and retains the cycle's call edges in the FR-151 order. |
| `invalid_model_binding` | `wrong-artifact-role`, `wrong-model-selection`, `wrong-export`, `wrong-context`, `conflicting-binding`, `unpreserved-model-meaning`, `specialization-cycle`, `redefinition-target`, `derivation-conflict`, `unsupplied-producer-record`, `port-direction`, `unclosed-subsettings` or `unclosed-redefinitions`; retain the required/actual declaration or contract, referring location and affected meaning. Missing versus explicit null may not be collapsed into a generic absent value. `wrong-export` for a systems-model kind retains the required kind, the actual kind (`none` when the record maps to no kind) and both producer identities. `specialization-cycle` retains the cycle's generalization record keys in path order. `redefinition-target` retains every redefinition record key and the zero or several resolved targets. `derivation-conflict` retains both derivation paths and exposes no chosen effective member. `unsupplied-producer-record` retains `{capability, required, supplied, item}`: the capability name, required and supplied producer interface versions and the item's producer key. `port-direction` retains the connection identity, its FCD direction and both ports' directions. `unclosed-subsettings` and `unclosed-redefinitions` retain the ModelSelection and its `subsettingClosure` or `redefinitionClosure` value. |
| `ill_typed` | `type-mismatch`, `unit-mismatch`, `operator-ineligible`, `ambiguous-literal`, `non-boolean-root`, `variance-parameter`, `variance-result`, `multiplicity-narrowing`, `effect-escape` or `subsetting-type`; retain the operator/input locus and expected/actual named type, unit or kind. The five conformance causes retain the redefining and redefined effective member identities, the axis (parameter index for `variance-parameter`), the expected and actual effective type, typed multiplicity or effect entry, and every contributing record key in effective-identity order. |
| `undefined_expression` | `unproved-range`, `unproved-nonzero`, `unproved-presence`, `unproved-decrease` or `unproved-refinement`; retain the potentially evaluated operation, exact operand/read identity and applicable guard/anchor context, and for `unproved-decrease` the recursive component cycle, call edge and failed measure obligation. `unproved-refinement` retains the redefining field's effective identity, the writing operation's effective identity and the obligation `field-domain`, `field-presence` or `no-proof-form`. Failure to prove definedness is not logical violation. |
| `ambiguous_dispatch` | `multiple-undominated` or `no-applicable`; retain the called effective operation, the closed subtype for which selection fails, every candidate effective method identity and, for `multiple-undominated`, every dominance pair among the candidates. It is a link-time refusal; no source or registration order resolves it. |
| `cardinality_out_of_bound` | `below-minimum` or `above-maximum`; retain the collection type, its inclusive bound, the formed occurrence or member count and the constructing or converting expression locus. It is a runtime refusal of a known out-of-bound collection value, not a type error or resource exhaustion. |
| `wrong_snapshot` | `wrong-observation`, `wrong-invocation`, `wrong-anchor` or `forbidden-pre-read`; retain required and supplied observation/invocation/anchor selections, or the exact prohibited read. |
| `invalid_runtime_input` | `malformed-json`, `missing-member`, `duplicate-member`, `unknown-member`, `wrong-value-kind`, `invalid-value`, `wrong-role-mapping`, `conflicting-role-mapping`, `absent-key`, `subsetting-violation` or `conflicting-identity`; retain the actual input path, owning required role and supplied mapping when relevant. `absent-key` retains the population binding, the requested key and the query locus. `subsetting-violation` retains the subsetting record key, the object and the values outside the subsetted feature. `conflicting-identity` retains both member records sharing one universe and object identity. A missing observation under a valid mapping is a different code. |
| `unavailable_observation` | `missing-required-artifact`, `missing-required-valuation` or `incomplete-scope`; retain the required role/scope, selected input and exact missing premise. |
| `incomplete_population` | `missing-required-artifact`, `missing-required-valuation`, `incomplete-scope`, `unclosed-subtypes` or `unclosed-method-set`; retain the required role/scope, selected input and exact missing premise. `unclosed-subtypes` and `unclosed-method-set` retain the ModelSelection and its `generalizationClosure` value, and the queried type or called operation. Unknown membership is not an empty or complete population. |
| `foreign_reference` | `foreign-universe`, `foreign-model-selection` or `foreign-type`; retain the required and supplied object universe, ModelSelection key or effective type identity, respectively, and the operation locus. No cause is decided from an authority, because a reference key carries none. |
| `dangling_reference` | `absent-target-in-complete-population`; retain the reference identity, exact required universe and admitted completeness selection. Without that complete population, use the incomplete-population outcome instead. |
| `population_delta_mismatch`, `frame_violation` | `delta-disagreement` or `unauthorized-change`, respectively; retain the invocation, pre/post selections, affected object/field or State root, and the declared delta/frame permission. |
| `resource_exhausted`, `cancelled` | `insufficient-next-charge` or `caller-cancelled`, respectively; retain the actual stage, accounting contract, effective limit or cancellation source, prior usage and the work item that could not proceed. A semantic maximum is not a caller work budget. |
| `unsupported_projection` | `unsupported-requested-capability`; retain the exact admitted subject, requested claim/target and selected backend. It cannot reclassify a valid source as malformed. |
| `projection_binding`, `invalid_projection_correspondence` | `upstream-refusal` or `correspondence-loss`, respectively; retain the originating structured IR/mapping cause and exact source/derived identities. |
| `extraction-requires-run`, `extraction-package-conflict`, `extraction-clause-count` | Distinct `wrong-command-mode`, `conflicting-compilation-inputs` and `binding-cardinality` variants, respectively; retain the requested mode, conflicting selectors or actual count. These three refusals cannot share only an English distinction. |
| `runtime_invariant` | `established-invariant-broken`; retain the named violated evaluator/input invariant and originating stage/context. It is a failed evaluation, not a false predicate. |

`ambiguous_dispatch` and `foreign_reference` are catalog-owned spellings of
this revision. Their spelling and causes are selected by this table even where
the pinned compiler inventory above does not enumerate them; a producer claiming
revision `1-draft.3` emits them with these closed causes, and a re-pinned
compiler inventory must agree with them.

Other retained host/source codes keep their selected broad meaning and original
structured producer cause: I/O error, malformed command, invalid digest/identifier,
output serialization failure, invalid Quire context, invalid source identity and
invalid source map. This catalog does not interpret their OS or upstream codes as
native subvariants. Readers retain that upstream selection explicitly; unknown
upstream meaning cannot establish a native semantic result.

## Undefined reasons

An `undefined` evaluation outcome of a package selecting `quire.model.complete/v1`
that names a reason carries exactly one of these closed reasons. A reason is not
a refusal code or cause and does not make the outcome refused.

| Reason | Required payload |
| --- | --- |
| `absent-key` | the population binding, the requested reference key and the `lookup` locus, for `lookup<T>(p, r) absent undefined` with no member of that key (FR-153) |
| `precondition-false` | the called effective operation, the selected method's effective identity, the receiver reference and the call locus, for a dispatched call whose selected effective precondition is false (FR-151) |

## Outcomes and delivery boundary

The catalog classifies diagnostics, not truth or adequacy. A false evaluated
predicate is a logical result under the selected family contract, not
`undefined_expression` or `runtime_invariant`. Failed prerequisites prevent only
their dependent requests from executing. Resource exhaustion, cancellation and
required missing observations retain incompleteness; known invalidity remains
refused. Retain both facts when they coexist, even when the detail list is capped.
An error-list length cannot decide success. B's result contract still owns
assessment execution, truth, activation, decision support and subject closure.

This definition adds no wire fields to historical payloads and no new production
implementation. The existing compiler/package/output interfaces must specify how
they carry the catalog selection and typed causes under a supported response
wire before claiming this interpretation. FR-047 and TC-047 own the shared
obligation; existing compiler L2/diagnostic work supplies the Rust implementation.

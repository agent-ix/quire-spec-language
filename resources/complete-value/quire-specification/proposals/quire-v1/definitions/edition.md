# Native Quire composed edition definition

Definition identity: `ix:native`; language edition: `1-draft`;
definition revision: `1-draft.2`. **The identity role is accepted by FS01; this
semantic revision remains proposed and is not adopted or fully implemented.**

## Selection and scope

The compiler inventory binds this exact definition artifact; source headers
select `language "ix:native" edition "1-draft";`. The artifact byte digest,
definition revision and language/edition remain distinct. This draft includes
the corrected static/runtime prerequisites and the domain package import.
Historical `0-draft` sources retain their separate identity.

This is a working definition in one coherent draft. Required definitions and
rule links resolve within this checkout; acceptance freezes their exact selections.
Earlier draft text remains in Git history. Runtime references still require the
selected immutable identities, revisions and digest domains.


This definition owns the complete lexical and syntactic grammar, closed source
inventory, common name resolution, source correspondence, static package
components, profile dependency resolution and typed diagnostic stages. Parsing
a temporal/protocol form does not admit its semantics without that family's
exact selected definition.

A profile definition's admitted-form catalog determines which recognized
constructs it permits. Its selected rule files supply their semantics and all
applicable type, definedness, source and binding constraints. Rules describing
another facet do not grant that facet's forms. A child profile may add explicitly
listed forms; it cannot weaken inherited numeric, presence, identity, purity,
anchor or resource rules. Imported predicates retain their own defining profile.

Required definitions use exact identity, revision and byte digest. Unknown or
conflicting definitions, cycles and incompatible inherited rules refuse dependent
admission. No definition is a runtime script, plugin or implicit network fetch.
A compiler reports support for the selected contract separately from knowing its
bytes. Markdown is the normative document encoding, not another clause parser.

Only the rule files explicitly selected below and transitive required definitions
are normative dependencies. Their external/background links do not silently add
unversioned rules. Source-model and observation contracts remain explicit
parameters of the linked package; neither this definition nor a profile supplies
a missing domain package or observation contract.

## Selected rule files

The following current rule files define this draft's grammar and admission
meaning. Background links do not add implicit semantic dependencies.

| Rule file |
| --- |
| [shared-grammar.md](../shared-grammar.md) |
| [package-contract.md](../package-contract.md) |
| [FR-030-bind-composed-definitions.md](../../../spec/functional/foundation/FR-030-bind-composed-definitions.md) |
| [FR-031-report-requested-capabilities.md](../../../spec/functional/foundation/FR-031-report-requested-capabilities.md) |
| [FR-032-preserve-language-evolution.md](../../../spec/functional/foundation/FR-032-preserve-language-evolution.md) |
| [FR-035-bind-ecosystem-subjects.md](../../../spec/functional/foundation/FR-035-bind-ecosystem-subjects.md) |
| [FR-036-retain-lexical-source-locations.md](../../../spec/functional/foundation/FR-036-retain-lexical-source-locations.md) |
| [FR-037-parse-shared-native-expressions.md](../../../spec/functional/foundation/FR-037-parse-shared-native-expressions.md) |
| [FR-038-resolve-predicate-scopes.md](../../../spec/functional/foundation/FR-038-resolve-predicate-scopes.md) |
| [FR-040-admit-explicit-state-extensions.md](../../../spec/functional/foundation/FR-040-admit-explicit-state-extensions.md) |
| [FR-047-emit-typed-located-causes.md](../../../spec/functional/foundation/FR-047-emit-typed-located-causes.md) |
| [NFR-010-bound-composed-processing.md](../../../spec/non-functional/NFR-010-bound-composed-processing.md) |

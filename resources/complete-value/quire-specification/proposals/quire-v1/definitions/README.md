# Native Quire v1 definition set

FS01 accepts and reserves the identity hierarchy and dependency direction in this
set. Each unsuffixed file contains the latest corrected semantic text and
links to its current requirements and dependencies. Identity-role acceptance is
not semantic-definition adoption, compiler/backend implementation or
qualification. Those statuses remain explicit in each artifact and owning
FS02–FS06/family ticket. Superseded draft copies live in Git history. Families
normally select a coherent reviewed repository revision; the complete-value
family additionally publishes an exact rule-closure manifest because its
consumer wire contract transports each rule as an immutable DefinitionRef.
Its [definition lock](complete-value-lock.json) separates the byte-verified
qualification catalog (always-selected, conditional, alternative and rule
artifacts) from per-package selection: the Unicode text profile is selected only
by `text_bearing` packages, the IEEE profile only by `ieee_operation` packages,
and exactly one integer-division profile only by `integer_div_rem` packages.
Listing a profile in the catalog does not select it.

FS03 accepts the exact `state-core → state-queries → state-graph` semantic
closure at the revisions below. The `1-draft` labels remain unpublished selection
identities, and acceptance does not imply complete implementation or qualification.
Other definition families remain editable candidates under their owning tickets.
The PR revision identifies the coherent reviewed checkout; editing a draft does
not require a checksum refresh or another successor file.

| Definition | Draft revision | Current file |
| --- | --- | --- |
| `ix:native` / edition `1-draft` | `1-draft.2` | [Edition and shared grammar](edition.md) |
| `quire.state.core/v1` | `1-draft.3` | [State core](state-core.md) |
| `quire.state.queries/v1` | `1-draft.3` | [Predicates and queries](state-queries.md) |
| `quire.state.graph/v1` | `1-draft.3` | [Finite graphs](state-graph.md) |
| `quire.temporal.bounded-facet/v1` | `1-draft.3` (accepted by FS04) | [Common temporal meaning](temporal-common.md) |
| Event-position temporal root | `1-draft.3` (accepted by FS04) | [Event positions](temporal-event-position.md) |
| Fixed-sample temporal root | `1-draft.3` (accepted by FS04) | [Fixed samples](temporal-fixed-sample.md) |
| Timestamped-window temporal root | `1-draft.3` (accepted by FS04) | [Timestamped windows](temporal-timestamped-window.md) |
| `quire.protocol.finite-global/v1` | `1-draft.4` | [Protocol and choreography](protocol-finite.md) |
| `quire.observation.binding/v1` | `1-draft.3` (accepted by FS04) | [Observation binding](observation-binding.md) |
| `quire.observation.progress/v1` | `1-draft.3` (accepted by FS04) | [Observation progress](observation-progress.md) |
| `quire.observation.range/v1` | `1-draft.1` (accepted by FS04) | [All-clock range correspondence](observation-range.md) |
| `quire.package.composed/v1` | `1-draft.2` (accepted by FS05) | [Linked-package interpretation](package-reference.md) |
| `quire.native.diagnostics/v1` | `1-draft.3` | [Diagnostic interpretation](native-diagnostics.md) |
| `quire.value.complete/v1` | `1-draft.1` | [Complete value root](value-complete.md) |
| `quire.value.complete.rules/v1` | `1-draft.1` | [Exact complete-value rule closure](value-complete-rules.json) |
| `quire.value.text.unicode-17.0.0/v1` | `1-draft.1` | [Unicode text profiles](value-text-unicode-17.md) |
| `quire.value.integer-division.truncating/v1` | `1-draft.1` | [Truncating integer division](value-integer-division-truncating.md) |
| `quire.value.integer-division.floor/v1` | `1-draft.1` | [Floor integer division](value-integer-division-floor.md) |
| `quire.value.integer-division.euclidean/v1` | `1-draft.1` | [Euclidean integer division](value-integer-division-euclidean.md) |
| `quire.value.ieee754-2019-default/v1` | `1-draft.1` | [IEEE binary profile](value-ieee754-2019-default.md) |
| `quire.value.accounting/v1` | `1-draft.1` | [Scalar accounting schedule](value-accounting.md) |
| `quire.value.compound-unit/v1` | `1-draft.1` | [Evaluator-owned compound units](value-compound-unit.md) |
| `quire.model.complete/v1` | `1-draft.1` | [Complete model root](model-complete.md) |
| `quire.model.complete.rules/v1` | `1-draft.1` | [Exact complete-model rule closure](model-complete-rules.json) |
| `quire.model.effective-declaration.schema/v1` | `1-draft.1` | [Effective declaration schema](../../checked-package-v2/model-effective-declaration.schema.json) |
| `quire.model.effective-declaration.vectors/v1` | `1-draft.1` | [Effective declaration vectors](../../checked-package-v2/model-effective-declaration-vectors.json) |
| `quire.model.definition-lock/v1` | `1-draft.1` | [Complete model lock](complete-model-lock.json) |

The [package contract](../package-contract.md), [state contract](../state-contract.md),
[protocol contract](../protocol-contract.md) and [observation contract](../observation-contract.md)
are current semantic inputs to these definitions. External producer and compiler
inventory selections remain explicitly identified at their consuming boundary;
they are not silently replaced with whatever another repository currently has.

FS03 accepts the ordered state closure, FS04 accepts the temporal/observation
closure, and FS05 accepts the linked-package
interpretation plus its selected strict reference/canonical/migration contracts.
Other listed family definitions retain their own acceptance tickets. At each
owning semantic acceptance, freeze the relevant coherent definition
closure and its immutable revision/digest. Runtime identity, source provenance,
digest-domain separation and refusal of mismatched inputs remain required by
FR-030/032/035. This authoring workflow does not weaken those product semantics
or rewrite previously accepted profiles.

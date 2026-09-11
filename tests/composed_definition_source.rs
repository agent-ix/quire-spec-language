// SPDX-License-Identifier: AGPL-3.0-only
//! TC-114: fixed native definition selections and their exact normative resources.
//! Supplied-inventory refusal and declaration admission are tested separately.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use ix_trace_rs::trace;
use quire_spec_language::linking::composed::definition_source::RegisteredDefinition;
use quire_spec_language::ByteDigest;

use RegisteredDefinition::{
    Diagnostics, Edition, EventPosition, FixedSample, ObservationBinding, Package, Progress,
    Protocol, Range, StateCore, StateGraph, StateQueries, TemporalFacet, TimestampedWindow,
};

struct Expected {
    definition: RegisteredDefinition,
    identity: &'static str,
    revision: &'static str,
    file: &'static str,
    requirements: &'static [RegisteredDefinition],
}

// These expectations transcribe the selected documents, independently of the
// implementation's registry table. Edition and definition revision differ.
const EXPECTED: &[Expected] = &[
    Expected {
        definition: Edition,
        identity: "ix:native",
        revision: "1-draft.2",
        file: "edition.md",
        requirements: &[],
    },
    Expected {
        definition: StateCore,
        identity: "quire.state.core/v1",
        revision: "1-draft.3",
        file: "state-core.md",
        requirements: &[Edition],
    },
    Expected {
        definition: StateQueries,
        identity: "quire.state.queries/v1",
        revision: "1-draft.3",
        file: "state-queries.md",
        requirements: &[StateCore],
    },
    Expected {
        definition: StateGraph,
        identity: "quire.state.graph/v1",
        revision: "1-draft.3",
        file: "state-graph.md",
        requirements: &[StateQueries],
    },
    Expected {
        definition: TemporalFacet,
        identity: "quire.temporal.bounded-facet/v1",
        revision: "1-draft.3",
        file: "temporal-common.md",
        requirements: &[StateGraph],
    },
    Expected {
        definition: EventPosition,
        identity: "quire.temporal.event-position.false-extension/v1",
        revision: "1-draft.3",
        file: "temporal-event-position.md",
        requirements: &[TemporalFacet],
    },
    Expected {
        definition: FixedSample,
        identity: "quire.temporal.fixed-sample.false-extension/v1",
        revision: "1-draft.3",
        file: "temporal-fixed-sample.md",
        requirements: &[TemporalFacet],
    },
    Expected {
        definition: TimestampedWindow,
        identity: "quire.temporal.timestamped-event.finite-window/v1",
        revision: "1-draft.3",
        file: "temporal-timestamped-window.md",
        requirements: &[TemporalFacet],
    },
    Expected {
        definition: Protocol,
        identity: "quire.protocol.finite-global/v1",
        revision: "1-draft.4",
        file: "protocol-finite.md",
        requirements: &[StateGraph, TemporalFacet],
    },
    Expected {
        definition: ObservationBinding,
        identity: "quire.observation.binding/v1",
        revision: "1-draft.3",
        file: "observation-binding.md",
        requirements: &[Package, Range, TemporalFacet, Protocol],
    },
    Expected {
        definition: Progress,
        identity: "quire.observation.progress/v1",
        revision: "1-draft.3",
        file: "observation-progress.md",
        requirements: &[ObservationBinding, Range, TemporalFacet],
    },
    Expected {
        definition: Range,
        identity: "quire.observation.range/v1",
        revision: "1-draft.1",
        file: "observation-range.md",
        requirements: &[],
    },
    Expected {
        definition: Package,
        identity: "quire.package.composed/v1",
        revision: "1-draft.2",
        file: "package-reference.md",
        requirements: &[Edition],
    },
    Expected {
        definition: Diagnostics,
        identity: "quire.native.diagnostics/v1",
        revision: "1-draft.1",
        file: "native-diagnostics.md",
        requirements: &[],
    },
];

fn resource(path: &str) -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("resources/native-v1")
            .join(path),
    )
    .unwrap()
}

#[test]
#[trace("TC-114", "FR-036-AC-1")]
fn exact_definition_bytes_and_labels_select_distinct_registered_meanings() {
    assert_eq!(
        RegisteredDefinition::all(),
        EXPECTED
            .iter()
            .map(|row| row.definition)
            .collect::<Vec<_>>()
    );
    let mut identities = BTreeSet::new();
    let mut digests = BTreeSet::new();
    for row in EXPECTED {
        let path = format!("proposals/quire-v1/definitions/{}", row.file);
        let original = resource(&path);
        let definition = row.definition;
        let selection = definition.selection();
        assert_eq!(definition.path(), path);
        assert_eq!(definition.bytes(), original);
        assert_eq!(definition.identity(), row.identity);
        assert_eq!(definition.revision(), row.revision);
        assert_eq!(selection.identity, row.identity);
        assert_eq!(selection.revision, row.revision);
        assert_eq!(selection.digest, ByteDigest::of(&original));
        assert_eq!(definition.requirements(), row.requirements);
        assert!(identities.insert(row.identity));
        assert!(digests.insert(selection.digest.to_string()));
    }
}

#[test]
#[trace("TC-114", "FR-036-AC-1")]
fn direct_rules_retain_the_selected_file_inventory_and_original_bytes() {
    let expected: &[(RegisteredDefinition, &[&str])] = &[
        (Edition, &[
            "proposals/quire-v1/shared-grammar.md",
            "proposals/quire-v1/package-contract.md",
            "spec/functional/FR-030-bind-composed-definitions.md",
            "spec/functional/FR-031-report-requested-capabilities.md",
            "spec/functional/FR-032-preserve-language-evolution.md",
            "spec/functional/FR-035-bind-ecosystem-subjects.md",
            "spec/functional/FR-036-retain-lexical-source-locations.md",
            "spec/functional/FR-037-parse-shared-native-expressions.md",
            "spec/functional/FR-038-resolve-predicate-scopes.md",
            "spec/functional/FR-040-admit-explicit-state-extensions.md",
            "spec/functional/FR-047-emit-typed-located-causes.md",
            "spec/non-functional/NFR-010-bound-composed-processing.md",
        ]),
        (StateCore, &[
            "proposals/quire-v1/state-contract.md",
            "spec/functional/FR-039-normalize-exact-rational-literals.md",
            "spec/functional/FR-044-check-composed-numeric-definedness.md",
            "spec/functional/FR-045-preserve-control-and-presence-facts.md",
            "spec/functional/FR-046-validate-state-invocation-inputs.md",
        ]),
        (StateQueries, &[
            "spec/functional/FR-033-admit-reusable-predicates.md",
            "spec/functional/FR-034-bind-cross-family-predicates.md",
            "spec/functional/FR-041-evaluate-ordered-queries.md",
            "spec/functional/FR-042-select-pre-state-reads.md",
        ]),
        (StateGraph, &["spec/functional/FR-043-evaluate-finite-graph-relations.md"]),
        (TemporalFacet, &[
            "spec/functional/FR-048-bind-native-temporal-syntax.md",
            "spec/functional/FR-090-select-temporal-profile-and-clock.md",
            "spec/functional/FR-091-evaluate-bounded-future.md",
            "spec/functional/FR-092-evaluate-bounded-past.md",
            "spec/functional/FR-093-bind-temporal-activation-and-captures.md",
            "spec/functional/FR-094-interpret-progress-history-and-closure.md",
            "spec/functional/FR-095-preserve-native-tl-correspondence.md",
            "spec/non-functional/NFR-040-bound-temporal-state.md",
            "spec/functional/FR-061-report-orthogonal-results.md",
        ]),
        (EventPosition, &[]),
        (FixedSample, &[]),
        (TimestampedWindow, &[]),
        (Protocol, &[
            "spec/functional/FR-049-bind-native-choreography-syntax.md",
            "spec/functional/FR-050-bind-protocol-instances.md",
            "spec/functional/FR-051-preserve-communication-identities.md",
            "spec/functional/FR-052-represent-bounded-control.md",
            "spec/functional/FR-053-enforce-choice-visibility.md",
            "spec/functional/FR-054-bind-channel-premises.md",
            "spec/functional/FR-055-activate-protocol-obligations.md",
            "spec/functional/FR-056-register-compensation.md",
            "spec/functional/FR-057-enforce-commit-recovery.md",
            "spec/functional/FR-058-preserve-retry-and-partial-recovery.md",
            "spec/functional/FR-059-assess-finite-global-conformance.md",
            "spec/functional/FR-060-separate-protocol-claims.md",
            "spec/functional/FR-061-report-orthogonal-results.md",
            "spec/non-functional/NFR-020-bound-protocol-processing.md",
            "spec/non-functional/NFR-021-reproduce-protocol-results.md",
            "proposals/quire-v1/choreography-surface.md",
            "proposals/quire-v1/protocol-contract.md",
        ]),
        (ObservationBinding, &[
            "proposals/quire-v1/observation-contract.md",
            "spec/functional/FR-061-report-orthogonal-results.md",
            "spec/functional/FR-115-report-activation-participation-and-adequacy.md",
            "spec/functional/FR-116-map-observation-results-to-consumers.md",
            "proposals/quire-v1/observation-output-mapping-contract.md",
        ]),
        (Progress, &[
            "proposals/quire-v1/observation-contract.md",
            "spec/functional/FR-061-report-orthogonal-results.md",
        ]),
        (Range, &[
            "proposals/quire-v1/observation-contract.md",
            "spec/functional/FR-090-select-temporal-profile-and-clock.md",
        ]),
        (Package, &[
            "proposals/shared-reference-2-draft/schema.json",
            "proposals/shared-reference-2-draft/README.md",
        ]),
        (Diagnostics, &[
            "https://github.com/agent-ix/quire-spec-language/blob/f444d03c06539a6cd0ada6be4ae099b54466d9d9/src/diagnostic.rs",
            "https://github.com/agent-ix/quire-spec-language/blob/f444d03c06539a6cd0ada6be4ae099b54466d9d9/docs/native-error-codes.md",
        ]),
    ];
    assert_eq!(
        expected.iter().map(|row| row.0).collect::<Vec<_>>(),
        RegisteredDefinition::all()
    );
    let mut shared = BTreeMap::new();
    for (definition, paths) in expected {
        let rules = definition.rules();
        assert_eq!(
            rules.iter().map(|rule| rule.path).collect::<Vec<_>>(),
            *paths
        );
        let unique: BTreeSet<_> = rules.iter().map(|rule| rule.path).collect();
        assert_eq!(unique.len(), rules.len());
        for rule in rules {
            let resource_path = match rule.path {
                "https://github.com/agent-ix/quire-spec-language/blob/f444d03c06539a6cd0ada6be4ae099b54466d9d9/src/diagnostic.rs" => "external/quire-spec-language/src/diagnostic.rs",
                "https://github.com/agent-ix/quire-spec-language/blob/f444d03c06539a6cd0ada6be4ae099b54466d9d9/docs/native-error-codes.md" => "external/quire-spec-language/docs/native-error-codes.md",
                path => path,
            };
            assert_eq!(rule.bytes, resource(resource_path), "{}", rule.path);
            shared
                .entry(rule.path)
                .or_insert_with(Vec::new)
                .push(rule.bytes);
        }
    }
    assert_eq!(shared.len(), 55);
    // FR-061 is one shared interface across four direct selections; a different
    // copy under one consumer must not redefine that same rule identity.
    let result_rules = &shared["spec/functional/FR-061-report-orthogonal-results.md"];
    assert_eq!(result_rules.len(), 4);
    let result_bytes = resource("spec/functional/FR-061-report-orthogonal-results.md");
    for supplied in result_rules {
        assert_eq!(*supplied, result_bytes);
    }
}

#[test]
#[trace("TC-114", "FR-036-AC-1")]
fn protocol_and_observation_closures_do_not_select_a_concrete_runtime_clock() {
    let expected: &[(RegisteredDefinition, &[RegisteredDefinition])] = &[
        (
            Protocol,
            &[
                Edition,
                StateCore,
                StateQueries,
                StateGraph,
                TemporalFacet,
                Protocol,
            ],
        ),
        (
            EventPosition,
            &[
                Edition,
                StateCore,
                StateQueries,
                StateGraph,
                TemporalFacet,
                EventPosition,
            ],
        ),
        (
            FixedSample,
            &[
                Edition,
                StateCore,
                StateQueries,
                StateGraph,
                TemporalFacet,
                FixedSample,
            ],
        ),
        (
            TimestampedWindow,
            &[
                Edition,
                StateCore,
                StateQueries,
                StateGraph,
                TemporalFacet,
                TimestampedWindow,
            ],
        ),
        (
            Progress,
            &[
                Edition,
                StateCore,
                StateQueries,
                StateGraph,
                TemporalFacet,
                Protocol,
                ObservationBinding,
                Progress,
                Range,
                Package,
            ],
        ),
        (Range, &[Range]),
        (Diagnostics, &[Diagnostics]),
    ];
    for (root, expected_closure) in expected {
        let mut found = BTreeSet::new();
        let mut pending = vec![*root];
        while let Some(current) = pending.pop() {
            if found.insert(current) {
                pending.extend(current.requirements());
            }
        }
        assert_eq!(
            found,
            expected_closure.iter().copied().collect(),
            "{root:?}"
        );
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-411, FR-940, FR-124: a unit that selects two models, each with its own
//! `terminal` declaration, classifies terminal states and replays an
//! undefined `terminal when` as one declaration on the source route and on
//! the package route, which agree.

use filament_exact::Cancel;
use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_analyze::analyze::{
    deadlock_model_item, AnalyzeLimits, PackageDeadlockSubject, RunInput, SourceProvision,
    SubjectBinding, TemporalItem,
};
use fsl_eval::simulation::model::{
    ExpansionLimits, ModelSimulationRequest, ModelSystem, PackageModelInput, PopulationUniverse,
    TerminalClass,
};
use fsl_eval::simulation::TransitionSystem;
use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_package::{BarePackage, BareSupply, CheckedPackage, EmitLimits, V2ReadLimits};
use fsl_route::request::ItemKind;
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::intake::package_input;
use fsl_semantics::model::observation::{DocumentProvision, DocumentRef, ObservationLimits};
use ix_trace_rs::trace;

use crate::counter::UNLIMITED;
use crate::hyper_world::{
    declarations, document, key, other_document, other_key, snapshot, unit_text_with_functions,
    Vault,
};

/// Each vault steps once to `l = i` while `l = 0`, so it is terminal at `l = 1`.
const CONTRACTS: &str = "pre Fresh using v on V::Vault::step { self.l = 0 }\n\
     post Step using v on V::Vault::step { self.l = i }\n\
     pre FreshN using v on N::Vault::step { self.l = 0 }\n\
     post StepN using v on N::Vault::step { self.l = i }\n";

/// `q` divides 6 by its argument, so it is undefined at zero (Kernel).
const FUNCTIONS: &str = "function q using v(x: Integer): Integer pure { 6 mod x }\n\
     function b using v(x: Integer): Int[0, 3] pure { x + 3 }\n";

/// The declaration on `test/vault` (alias `V`) that is undefined at the
/// terminal state `l = 1`.
const UNDEFINED_WHEN: &str = "terminal Done using v on V when { q(1 - self.l) = 0 };\n";
/// The declaration on `test/vault-other` (alias `N`).
const OTHER_ANY: &str = "terminal Halt using v on N any;\n";
/// The declaration on `test/vault` that intends every terminal state.
const ANY: &str = "terminal Halt using v on V any;\n";
/// The declaration on `test/vault-other` that never intends a state.
const OTHER_FALSE: &str = "terminal Stop using v on N when { false };\n";
/// The declaration on `test/vault-other` that is undefined at `l = 1`.
const OTHER_UNDEFINED_WHEN: &str = "terminal Done using v on N when { q(1 - self.l) = 0 };\n";

const ONE_IDENTITY_CONTRACTS: &str = "pre Fresh using v on M::Vault::step { self.l = 0 }\n\
     post Step using v on M::Vault::step { self.l = i }\n";
const ONE_IDENTITY_DISABLED_STEP: &str = "pre Disabled using v on M::Vault::step { false }\n";

/// A two-model unit, its package and its initial snapshot: each vault at
/// `(h, l) = (0, 0)`; a `step` sets `l`, and a vault is terminal once `l` is 1.
struct Two {
    text: String,
    source: SourceProvision,
    package: CheckedPackage,
    packages: fsl_semantics::model::intake::PackageInput,
    snapshots: DocumentProvision,
    initial: Vec<DocumentRef>,
    /// Whether the subject's universes also cover `test/vault-other`.
    reads_other: bool,
}

fn two_models(terminals: &str) -> Two {
    let first = document(false, false);
    let second = other_document(&first);
    let documents: [&[u8]; 2] = [&first, &second];
    let text = unit_text_with_functions(
        Some(&second),
        Vault::Custom(CONTRACTS),
        false,
        FUNCTIONS,
        terminals,
    );
    let graph = declarations(&documents, &text)
        .check(
            CheckMode::Kernel,
            CheckingLimits::default(),
            &fsl_eval::value::S6aRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("{text}: the unit checks: {refusals:?}"));
    let source = SourceProvision::single(graph.source(), text.as_bytes());
    let mut snapshots = DocumentProvision::new();
    let start = snapshot(
        &mut snapshots,
        "s0",
        &[("v", (0, 0))],
        None,
        Some(&[("v", (0, 0))]),
    );
    Two {
        text,
        source,
        package: CheckedPackage::link(graph),
        packages: package_input(documents.iter().copied()).unwrap(),
        snapshots,
        initial: vec![start],
        reads_other: true,
    }
}

impl Two {
    /// The same unit read by a subject whose universes cover `test/vault`
    /// alone.
    fn reading_vault_only(mut self) -> Self {
        self.reads_other = false;
        self.snapshots = DocumentProvision::new();
        let start = snapshot(&mut self.snapshots, "s0", &[("v", (0, 0))], None, None);
        self.initial = vec![start];
        self
    }

    fn universes(&self) -> Vec<PopulationUniverse> {
        let other = self.reads_other.then(|| other_key("vaults"));
        std::iter::once(key("vaults"))
            .chain(other)
            .map(|population| PopulationUniverse {
                population,
                keys: vec!["v".to_owned()],
            })
            .collect()
    }

    fn binding(&self) -> SubjectBinding<'_> {
        SubjectBinding {
            packages: &self.packages,
            snapshots: &self.snapshots,
            initial: self.initial.clone(),
            universes: self.universes(),
        }
    }

    fn simulation<'a>(
        &'a self,
        input: fsl_eval::simulation::model::ModelCheckInput<'a>,
    ) -> ModelSimulationRequest<'a> {
        ModelSimulationRequest {
            emit: EmitLimits::fixture(),
            input,
            packages: &self.packages,
            normalization: ModelNormalizationLimits::default(),
            snapshots: &self.snapshots,
            initial: self.initial.clone(),
            universes: self.universes(),
            observation: ObservationLimits::default(),
            expansion: ExpansionLimits {
                max_candidates: 1_000_000,
            },
            meter: UNLIMITED,
        }
    }
}

/// One selected vault identity under two aliases, used by FR-124-AC-11.
struct OneIdentity {
    text: String,
    source: SourceProvision,
    package: CheckedPackage,
    packages: fsl_semantics::model::intake::PackageInput,
    snapshots: DocumentProvision,
    initial: Vec<DocumentRef>,
    vault_keys: Vec<String>,
}

fn one_identity(terminals: &str) -> OneIdentity {
    one_identity_with(terminals, ONE_IDENTITY_CONTRACTS, &[("v", (0, 0))], "")
}

fn one_identity_with(
    terminals: &str,
    contracts: &'static str,
    vaults: &[(&str, (i64, i64))],
    bounds: &str,
) -> OneIdentity {
    let document = document(false, false);
    let text = unit_text_with_functions(
        None,
        Vault::Custom(contracts),
        false,
        FUNCTIONS,
        &format!("{bounds}{terminals}"),
    )
    .replace(
        "model V = \"test/vault\";",
        "model M = \"test/vault\";\nmodel N = \"test/vault\";",
    );
    let documents: [&[u8]; 1] = [&document];
    let graph = declarations(&documents, &text)
        .check(
            CheckMode::Kernel,
            CheckingLimits::default(),
            &fsl_eval::value::S6aRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("{text}: the unit checks: {refusals:?}"));
    let source = SourceProvision::single(graph.source(), text.as_bytes());
    let mut snapshots = DocumentProvision::new();
    let start = snapshot(&mut snapshots, "one-identity-start", vaults, None, None);
    OneIdentity {
        text,
        source,
        package: CheckedPackage::link(graph),
        packages: package_input(documents.iter().copied()).unwrap(),
        snapshots,
        initial: vec![start],
        vault_keys: vaults.iter().map(|(key, _)| (*key).to_owned()).collect(),
    }
}

impl OneIdentity {
    fn binding(&self) -> SubjectBinding<'_> {
        SubjectBinding {
            packages: &self.packages,
            snapshots: &self.snapshots,
            initial: self.initial.clone(),
            universes: vec![PopulationUniverse {
                population: key("vaults"),
                keys: self.vault_keys.clone(),
            }],
        }
    }

    fn simulation<'a>(
        &'a self,
        input: fsl_eval::simulation::model::ModelCheckInput<'a>,
    ) -> ModelSimulationRequest<'a> {
        ModelSimulationRequest {
            emit: EmitLimits::fixture(),
            input,
            packages: &self.packages,
            normalization: ModelNormalizationLimits::default(),
            snapshots: &self.snapshots,
            initial: self.initial.clone(),
            universes: vec![PopulationUniverse {
                population: key("vaults"),
                keys: self.vault_keys.clone(),
            }],
            observation: ObservationLimits::default(),
            expansion: ExpansionLimits {
                max_candidates: 1_000_000,
            },
            meter: UNLIMITED,
        }
    }
}

/// The package written to its handoff bytes, with the rows of its supply.
struct Handoff {
    bytes: Vec<u8>,
    admitted: fsl_package::CheckedPackageV2,
    identities: fsl_package::ReadIdentityRows,
    equality: fsl_package::ReadEqualityRows,
    identity_bytes: Vec<u8>,
    equality_bytes: Vec<u8>,
}

fn handoff(world: &Two) -> Handoff {
    handoff_package(&world.package, &world.packages)
}

fn handoff_package(
    package: &CheckedPackage,
    packages: &fsl_semantics::model::intake::PackageInput,
) -> Handoff {
    let emission = fsl_package::emit_checked(package, EmitLimits::fixture()).expect("emits");
    let bytes = emission.package().bytes().to_vec();
    let admitted = fsl_package::admit_checked_package(&bytes, packages, V2ReadLimits::fixture())
        .expect("the strict reader admits the bytes");
    let mut budget = V2ReadLimits::fixture().native_join_budget();
    let identities = emission
        .identity_rows(&admitted, &mut budget)
        .expect("the identity rows");
    let equality = emission
        .equality_witness(&admitted, &mut budget)
        .expect("the equality rows");
    Handoff {
        identity_bytes: identities.to_bytes(&admitted).expect("identities.json"),
        equality_bytes: equality.to_bytes(&admitted).expect("equality.json"),
        bytes,
        admitted,
        identities,
        equality,
    }
}

impl Handoff {
    fn bare(&self) -> BarePackage<'_> {
        BarePackage::new(&self.admitted)
            .with_supply(
                BareSupply {
                    identities: &self.identities,
                    equality: Some(&self.equality),
                },
                &mut V2ReadLimits::fixture().native_join_budget(),
            )
            .expect("the supply joins")
    }

    fn supply(&self) -> fsl_replay::HandoffSupply<'_> {
        fsl_replay::HandoffSupply {
            dependencies: &[],
            identities: &self.identity_bytes,
            equality: Some(&self.equality_bytes),
        }
    }
}

/// The class of every state `system` reaches, breadth first.
fn classes(system: &ModelSystem<'_>) -> Vec<(Vec<u8>, TerminalClass)> {
    let mut queue: std::collections::VecDeque<_> = system.initial().into_iter().collect();
    let mut seen: Vec<Vec<u8>> = Vec::new();
    let mut classes = Vec::new();
    while let Some(state) = queue.pop_front() {
        let key = system.state_key(&state).expect("a state key").0;
        if seen.contains(&key) {
            continue;
        }
        seen.push(key.clone());
        let expansion = system.successors(&state).expect("expands");
        let class = system
            .terminal_class(&state, !expansion.successors.is_empty())
            .expect("the class reads");
        classes.push((key, class));
        queue.extend(expansion.successors.into_iter().map(|(_, next)| next));
    }
    classes
}

/// FR-411, FR-940-AC-6: over a unit that selects two model identities, the
/// source route and the package route classify every state alike, and their
/// terminal declarations join by selection identity. FR-124 reads one model's
/// declarations for a subject and FSpec FR-368-AC-22 gives a package with
/// several model roots no derived deadlock-freedom item, so what a joint
/// state classifies as is not asserted here, only that both routes agree.
#[trace("TC-2958", "FR-940-AC-6")]
#[test]
fn two_models_read_their_terminals_in_selection_identity_order_on_both_routes() {
    for terminals in [
        String::new(),
        "terminal Done using v on V when { self.l = 1 };\n\
         terminal Fin using v on N when { self.l = 1 };\n"
            .to_owned(),
        format!("{ANY}{OTHER_UNDEFINED_WHEN}"),
        format!("{UNDEFINED_WHEN}{OTHER_ANY}"),
        format!("{UNDEFINED_WHEN}terminal Fin using v on N when {{ self.l = 1 }};\n"),
        format!("terminal Done using v on V when {{ b(self.l) = 0 }};\n{OTHER_UNDEFINED_WHEN}"),
        UNDEFINED_WHEN.to_owned(),
        format!("{ANY}terminal Done using v on N when {{ b(self.l) = 0 }};\n"),
        format!("terminal Done using v on V when {{ b(self.l) = 0 }};\n{OTHER_ANY}"),
    ] {
        let world = two_models(&terminals);
        let handoff = handoff(&world);
        let bare = handoff.bare();
        let input =
            PackageModelInput::of(&bare, &world.packages, ModelNormalizationLimits::default())
                .expect("the package route reads both terminals");
        let over_source = ModelSystem::new(world.simulation(
            fsl_eval::simulation::model::ModelCheckInput::source(&world.package),
        ))
        .expect("the source admits");
        let over_bytes =
            ModelSystem::new(world.simulation(input.input())).expect("the package admits");
        let (from_source, from_bytes) = (classes(&over_source), classes(&over_bytes));
        assert_eq!(from_bytes, from_source, "{terminals}");
        assert!(
            from_source
                .iter()
                .any(|(_, class)| *class != TerminalClass::NotTerminal),
            "{terminals}: a terminal state is reached"
        );
        assert_eq!(
            world_keys(&world),
            input.input().terminal_keys(),
            "{terminals}: the declarations join by selection identity"
        );
    }
}

fn world_keys(world: &Two) -> fsl_semantics::check::TerminalKeys {
    fsl_eval::simulation::model::ModelCheckInput::source(&world.package).terminal_keys()
}

/// FR-411, FR-940, FR-128-AC-6: the deadlock-freedom item over the two-model
/// unit whose first declaration is an undefined `when` is refuted at the
/// terminal predicate, and the package route's run and its replay from the
/// package bytes give the source route's outcome and settlement.
#[trace("TC-2958", "FR-940-AC-6")]
#[trace("TC-539", "FR-128-AC-6")]
#[test]
fn an_undefined_terminal_of_two_models_runs_and_replays_as_on_the_source_route() {
    let world = two_models(&format!("{UNDEFINED_WHEN}{OTHER_FALSE}"));
    let selection = world
        .package
        .graph()
        .model_selections()
        .first()
        .expect("a selection")
        .clone();
    let claim = OccurrenceKey::generated(
        fsl_semantics::check::model_root_key(&selection).expect("the model root"),
    );
    let form = TemporalItem {
        item_kind: ItemKind::DeadlockFreedom,
        ..TemporalItem::default()
    };
    let run = RunInput::source(&world.package);
    let source_item = || {
        deadlock_model_item(
            &run,
            &world.binding(),
            AnalyzeLimits::unbounded(),
            &world.source,
            &claim,
            &form,
        )
        .expect("the derived target is formed")
    };
    let handoff = handoff(&world);
    let bare = handoff.bare();
    let input = PackageModelInput::of(&bare, &world.packages, ModelNormalizationLimits::default())
        .expect("the package route reads both terminals");
    let package_item = || {
        PackageDeadlockSubject::new(
            &input,
            &handoff.bytes,
            handoff.supply(),
            world.binding(),
            AnalyzeLimits::unbounded(),
            &claim,
            &form,
        )
        .model_item()
    };
    let check = |item: fsl_analyze::analyze::ModelItem<'_>| {
        fsl_analyze::model_check::check_model(item.request, &Cancel::new())
    };
    let from_source = check(source_item()).expect("the source checks");
    let from_package = check(package_item()).expect("the package checks");
    assert_eq!(format!("{from_package:?}"), format!("{from_source:?}"));
    let fsl_replay::ModelCheckOutcome::Violated { counterexample, .. } = from_source else {
        panic!("the undefined terminal refutes the item: {from_source:?}");
    };
    assert!(
        matches!(
            counterexample.kind,
            fsl_replay::CounterexampleKind::UndefinedEvaluation(_)
        ),
        "{counterexample:?}"
    );
    let source_replay = (source_item().replay)(&counterexample).expect("the source replays");
    let package_replay = (package_item().replay)(&counterexample).expect("the bytes replay");
    assert!(
        format!("{:?}", source_replay.settlement).contains("Reproduced"),
        "{:?}",
        source_replay.settlement
    );
    assert_eq!(
        format!("{:?}", package_replay.settlement),
        format!("{:?}", source_replay.settlement)
    );
    assert_eq!(
        source_replay.terminals.mismatch(&package_replay.terminals),
        None
    );
    assert!(world.text.contains("model N"));
}

/// The Counter unit selecting `test/counter` as `M` and as `N`, with `body`.
fn two_aliases(body: &str) -> (CheckedPackage, fsl_semantics::model::intake::PackageInput) {
    use crate::counter::{document, unit_text_in, PACKAGE};
    let doc = document(false);
    let text = unit_text_in(PACKAGE, body).replace(
        "model M = \"test/counter\";",
        "model M = \"test/counter\";\nmodel N = \"test/counter\";",
    );
    let documents: [&[u8]; 1] = [&doc];
    let graph = declarations(&documents, &text)
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &fsl_eval::value::S6aRewardEvaluator,
        )
        .unwrap_or_else(|refusals| panic!("{text}: the unit checks: {refusals:?}"));
    (
        CheckedPackage::link(graph),
        package_input(documents.iter().copied()).unwrap(),
    )
}

fn bare_terminal_keys(
    package: &CheckedPackage,
    packages: &fsl_semantics::model::intake::PackageInput,
) -> fsl_semantics::check::TerminalKeys {
    let emission = fsl_package::emit_checked(package, EmitLimits::fixture()).expect("emits");
    let admitted = fsl_package::admit_checked_package(
        emission.package().bytes(),
        packages,
        V2ReadLimits::fixture(),
    )
    .expect("the strict reader admits the package");
    let mut budget = V2ReadLimits::fixture().native_join_budget();
    let identities = emission
        .identity_rows(&admitted, &mut budget)
        .expect("the identity rows");
    let equality = emission
        .equality_witness(&admitted, &mut budget)
        .expect("the equality rows");
    let mut budget = V2ReadLimits::fixture().native_join_budget();
    let bare = BarePackage::new(&admitted)
        .with_supply(
            BareSupply {
                identities: &identities,
                equality: Some(&equality),
            },
            &mut budget,
        )
        .expect("the package witnesses join");
    fsl_semantics::check::TerminalKeys::of(
        bare.terminal_declarations()
            .expect("the bare terminal rows read")
            .into_iter()
            .map(|terminal| ((terminal.model, terminal.ordinal), terminal.node)),
    )
}

/// FR-411 "Deadlock-freedom verdict join", FR-124: two aliases that select one
/// package identity each declare a terminal. Both routes key the declarations
/// by (identity, ordinal in terminal node key order), so neither replaces the
/// other when source placement changes.
#[trace("TC-1142", "FR-411-AC-5")]
#[trace("TC-519", "FR-124-AC-11")]
#[test]
fn two_aliases_of_one_identity_keep_their_terminals_apart() {
    let when = "terminal Done using v on M when { self.value = 3 };\n";
    let any = "terminal Halt using v on N any;\n";
    let (both, packages) = two_aliases(&format!("{when}{any}"));
    let (only_first, _) = two_aliases(when);
    let (only_second, _) = two_aliases(any);

    let keys = both.graph().terminal_keys();
    let first = only_first.graph().terminal_keys();
    let second = only_second.graph().terminal_keys();
    assert_ne!(keys, first, "the second alias's declaration is not dropped");
    assert_ne!(keys, second, "the first alias's declaration is not dropped");
    let resolved = fsl_package::resolve_terminals(&both).expect("the source nodes resolve");
    let mut by_node: Vec<_> = resolved.iter().collect();
    by_node.sort_by(|left, right| left.node.cmp(&right.node));
    for (ordinal, terminal) in by_node.into_iter().enumerate() {
        assert_eq!(
            terminal.ordinal, ordinal,
            "{} takes its ordinal in terminal node key order",
            terminal.name
        );
    }
    // The one declaration each single-declaration unit keeps sits at ordinal
    // 0, so the mismatch names the first ordinal whose node differs: 0 when
    // the kept node sorts second, 1 (the node that is missing) when first.
    let by_name = |name: &str| {
        resolved
            .iter()
            .find(|terminal| terminal.name == name)
            .expect("the terminal resolves")
    };
    let sorted: Vec<_> = {
        let mut sorted: Vec<_> = resolved.iter().map(|terminal| &terminal.node).collect();
        sorted.sort();
        sorted
    };
    for (kept, alone) in [("Done", &first), ("Halt", &second)] {
        let kept = by_name(kept);
        let mismatch = keys.mismatch(alone).expect("the dropped declaration shows");
        if kept.node == *sorted[0] {
            assert_eq!(mismatch.model, "test/counter#1");
            assert_eq!(mismatch.requested, Some(sorted[1].clone()));
            assert_eq!(mismatch.recompiled, None);
        } else {
            assert_eq!(mismatch.model, "test/counter");
            assert_eq!(mismatch.requested, Some(sorted[0].clone()));
            assert_eq!(mismatch.recompiled, Some(kept.node.clone()));
        }
    }
    assert_eq!(
        fsl_eval::simulation::model::ModelCheckInput::source(&both).terminal_keys(),
        keys,
        "the run's input keys its declarations as the graph does"
    );

    let emission = fsl_package::emit_checked(&both, EmitLimits::fixture()).expect("emits");
    let admitted = fsl_package::admit_checked_package(
        emission.package().bytes(),
        &packages,
        V2ReadLimits::fixture(),
    );
    let admitted = admitted.expect("a package carries two terminal nodes on one model node");
    let rows = fsl_package::read_terminals(&admitted).expect("the terminal rows read");
    let [first_row, second_row] = &rows[..] else {
        panic!("two terminal rows: {rows:?}");
    };
    assert_eq!(first_row.model, second_row.model);
    assert_eq!((first_row.ordinal, second_row.ordinal), (0, 1));
    assert!(first_row.node < second_row.node);
    assert_ne!(first_row.node, second_row.node);
    assert_eq!(
        fsl_package::read_terminal(&admitted, &second_row.model, 1),
        Ok(second_row.declaration.clone())
    );
    assert_ne!(first_row.declaration, second_row.declaration);
}

/// FR-124-AC-11: swapping the source placement of two aliases leaves the
/// source route, package resolver and bare package reader on the same
/// (identity, terminal node key order) ordinal assignment.
#[trace("TC-519", "FR-124-AC-11")]
#[test]
fn terminal_ordinals_are_stable_when_source_placements_swap() {
    let when = "terminal Done using v on M when { self.value = 3 };\n";
    let any = "terminal Halt using v on N any;\n";
    let (first, packages) = two_aliases(&format!("{when}{any}"));
    let (swapped, swapped_packages) = two_aliases(&format!("{any}{when}"));

    let keys = |package: &CheckedPackage, packages: &fsl_semantics::model::intake::PackageInput| {
        let source = package.graph().terminal_keys();
        let resolved = fsl_package::resolve_terminals(package)
            .expect("the source terminal nodes resolve")
            .into_iter()
            .map(|terminal| {
                (
                    (terminal.package.to_owned(), terminal.ordinal),
                    terminal.node,
                )
            });
        assert_eq!(source, fsl_semantics::check::TerminalKeys::of(resolved));
        assert_eq!(source, bare_terminal_keys(package, packages));
        source
    };

    assert_eq!(keys(&first, &packages), keys(&swapped, &swapped_packages));
}

/// FR-124-AC-11: the same identity selected by two aliases keeps the
/// undefined declaration's identity+ordinal key while the source label
/// follows the alias that owns `Done`; a false sibling does not make it
/// intended.
#[trace("TC-519", "FR-124-AC-11")]
#[test]
fn one_identity_undefined_terminal_keeps_key_and_source_label_across_routes() {
    for (done_alias, stop_alias) in [("M", "N"), ("N", "M")] {
        let terminals = format!(
            "terminal Done using v on {done_alias} when {{ q(1 - self.l) = 0 }};\n\
             terminal Stop using v on {stop_alias} when {{ false }};\n"
        );
        let world = one_identity(&terminals);
        let selection = world
            .package
            .graph()
            .model_selections()
            .first()
            .expect("the shared model identity is selected")
            .clone();
        let claim = OccurrenceKey::generated(
            fsl_semantics::check::model_root_key(&selection).expect("the model root"),
        );
        let form = TemporalItem {
            item_kind: ItemKind::DeadlockFreedom,
            ..TemporalItem::default()
        };
        let run = RunInput::source(&world.package);
        let source_item = || {
            deadlock_model_item(
                &run,
                &world.binding(),
                AnalyzeLimits::unbounded(),
                &world.source,
                &claim,
                &form,
            )
            .expect("the derived target is formed")
        };
        let handoff = handoff_package(&world.package, &world.packages);
        let bare = handoff.bare();
        let input =
            PackageModelInput::of(&bare, &world.packages, ModelNormalizationLimits::default())
                .expect("the package route reads both terminal nodes");
        let package_item = || {
            PackageDeadlockSubject::new(
                &input,
                &handoff.bytes,
                handoff.supply(),
                world.binding(),
                AnalyzeLimits::unbounded(),
                &claim,
                &form,
            )
            .model_item()
        };
        let check = |item: fsl_analyze::analyze::ModelItem<'_>| {
            fsl_analyze::model_check::check_model(item.request, &Cancel::new())
        };
        let from_source = check(source_item()).expect("the source checks");
        let from_package = check(package_item()).expect("the package checks");
        assert_eq!(format!("{from_package:?}"), format!("{from_source:?}"));
        let fsl_replay::ModelCheckOutcome::Violated { counterexample, .. } = from_source else {
            panic!("the undefined terminal refutes the item: {from_source:?}");
        };
        let fsl_replay::CounterexampleKind::UndefinedEvaluation(evaluation) = &counterexample.kind
        else {
            panic!("the counterexample names undefined evaluation: {counterexample:?}");
        };
        let fsl_replay::UndefinedLocus::Terminal(model) = &evaluation.site.locus else {
            panic!("the undefined site names a terminal declaration: {evaluation:?}");
        };
        let done = fsl_package::resolve_terminals(&world.package)
            .expect("the source terminal nodes resolve")
            .into_iter()
            .find(|terminal| terminal.name == "Done")
            .expect("Done resolves");
        assert_eq!(model.identity(), "test/vault");
        assert_eq!(model.ordinal(), done.ordinal);
        assert_eq!(model.label(), done_alias);
        assert!(world.text.contains(&format!("model {done_alias}")));
    }
}

/// FR-124-AC-11: a refused `Done` predicate paired with false `Stop` is
/// undecided, retaining Done's identity, ordinal and source alias in the
/// source model class, and settles inconclusive on both routes.
#[trace("TC-519", "FR-124-AC-11")]
#[test]
fn one_identity_refused_terminal_is_inconclusive_and_names_done() {
    let world = one_identity_with(
        "terminal Done using v on M when { exists(p in M::vaults : true) };\n\
         terminal Stop using v on N when { false };\n",
        ONE_IDENTITY_DISABLED_STEP,
        &[("a", (0, 0)), ("b", (0, 0))],
        "bound OnePerSet on M::vaults [1]\n",
    );
    let selection = world
        .package
        .graph()
        .model_selections()
        .first()
        .expect("the shared model identity is selected")
        .clone();
    let claim = OccurrenceKey::generated(
        fsl_semantics::check::model_root_key(&selection).expect("the model root"),
    );
    let form = TemporalItem {
        item_kind: ItemKind::DeadlockFreedom,
        ..TemporalItem::default()
    };
    let run = RunInput::source(&world.package);
    let source_item = || {
        deadlock_model_item(
            &run,
            &world.binding(),
            AnalyzeLimits::unbounded(),
            &world.source,
            &claim,
            &form,
        )
        .expect("the derived target is formed")
    };
    let handoff = handoff_package(&world.package, &world.packages);
    let bare = handoff.bare();
    let input = PackageModelInput::of(&bare, &world.packages, ModelNormalizationLimits::default())
        .expect("the package route reads both terminal nodes");
    let package_item = || {
        PackageDeadlockSubject::new(
            &input,
            &handoff.bytes,
            handoff.supply(),
            world.binding(),
            AnalyzeLimits::unbounded(),
            &claim,
            &form,
        )
        .model_item()
    };
    let check = |item: fsl_analyze::analyze::ModelItem<'_>| {
        fsl_analyze::model_check::check_model(item.request, &Cancel::new())
    };
    let from_source = check(source_item()).expect("the source check completes");
    let from_package = check(package_item()).expect("the package check completes");
    assert_eq!(format!("{from_package:?}"), format!("{from_source:?}"));
    assert!(matches!(
        from_source,
        fsl_replay::ModelCheckOutcome::Undecided {
            cause: fsl_replay::UndecidedCause::UndecidedSuccessor,
            ..
        }
    ));

    let source_system = ModelSystem::new(world.simulation(
        fsl_eval::simulation::model::ModelCheckInput::source(&world.package),
    ))
    .expect("the source admits");
    let package_system =
        ModelSystem::new(world.simulation(input.input())).expect("the package admits");
    let classify = |system: &ModelSystem<'_>| {
        let state = system.initial().into_iter().next().expect("initial state");
        let expansion = system.successors(&state).expect("expands");
        system
            .terminal_class(&state, !expansion.successors.is_empty())
            .expect("the terminal predicates read")
    };
    let fsl_eval::simulation::model::TerminalClass::Undecided {
        model: source_model,
    } = classify(&source_system)
    else {
        panic!("the source class is undecided");
    };
    let fsl_eval::simulation::model::TerminalClass::Undecided {
        model: package_model,
    } = classify(&package_system)
    else {
        panic!("the package class is undecided");
    };
    let done = fsl_package::resolve_terminals(&world.package)
        .expect("the source terminal nodes resolve")
        .into_iter()
        .find(|terminal| terminal.name == "Done")
        .expect("Done resolves");
    assert_eq!(source_model.identity(), "test/vault");
    assert_eq!(source_model.ordinal(), done.ordinal);
    assert_eq!(source_model.label(), "M");
    assert_eq!(package_model.identity(), source_model.identity());
    assert_eq!(package_model.ordinal(), source_model.ordinal());
    assert_eq!(source_model, package_model);
}

/// The deadlock-freedom item of the model with selection identity `identity`
/// settled on the source route and on the package route, which agree.
fn deadlock_outcome<'w>(
    package: &CheckedPackage,
    source: &SourceProvision,
    packages: &fsl_semantics::model::intake::PackageInput,
    binding: &dyn Fn() -> SubjectBinding<'w>,
    identity: &str,
) -> fsl_replay::ModelCheckOutcome {
    let selection = package
        .graph()
        .model_selections()
        .iter()
        .find(|selection| selection.identity == identity)
        .expect("the identity is selected")
        .clone();
    let claim = OccurrenceKey::generated(
        fsl_semantics::check::model_root_key(&selection).expect("the model root"),
    );
    let form = TemporalItem {
        item_kind: ItemKind::DeadlockFreedom,
        ..TemporalItem::default()
    };
    let run = RunInput::source(package);
    let source_item = deadlock_model_item(
        &run,
        &binding(),
        AnalyzeLimits::unbounded(),
        source,
        &claim,
        &form,
    )
    .expect("the derived target is formed");
    let handoff = handoff_package(package, packages);
    let bare = handoff.bare();
    let input = PackageModelInput::of(&bare, packages, ModelNormalizationLimits::default())
        .expect("the package route reads the terminal nodes");
    let package_item = PackageDeadlockSubject::new(
        &input,
        &handoff.bytes,
        handoff.supply(),
        binding(),
        AnalyzeLimits::unbounded(),
        &claim,
        &form,
    )
    .model_item();
    let check = |item: fsl_analyze::analyze::ModelItem<'_>| {
        fsl_analyze::model_check::check_model(item.request, &Cancel::new())
            .expect("the item completes")
    };
    let from_source = check(source_item);
    let from_package = check(package_item);
    assert_eq!(format!("{from_package:?}"), format!("{from_source:?}"));
    from_source
}

/// The terminal key and label an `UndefinedEvaluation` refutation names.
fn undefined_terminal(outcome: &fsl_replay::ModelCheckOutcome) -> (String, usize, String) {
    let fsl_replay::ModelCheckOutcome::Violated { counterexample, .. } = outcome else {
        panic!("the item is refuted: {outcome:?}");
    };
    let fsl_replay::CounterexampleKind::UndefinedEvaluation(evaluation) = &counterexample.kind
    else {
        panic!("the counterexample names undefined evaluation: {counterexample:?}");
    };
    let fsl_replay::UndefinedLocus::Terminal(model) = &evaluation.site.locus else {
        panic!("the undefined site names a terminal declaration: {evaluation:?}");
    };
    (
        model.identity().to_owned(),
        model.ordinal(),
        model.label().to_owned(),
    )
}

/// The class of the one terminal state of `world` read by a subject whose
/// universes are `world`'s, over the source route.
fn terminal_class_of(world: &Two) -> TerminalClass {
    let system = ModelSystem::new(world.simulation(
        fsl_eval::simulation::model::ModelCheckInput::source(&world.package),
    ))
    .expect("the source admits");
    classes(&system)
        .into_iter()
        .map(|(_, class)| class)
        .find(|class| *class != TerminalClass::NotTerminal)
        .expect("a terminal state is reached")
}

/// FR-124-AC-12: a subject that reads `test/vault` only consults that
/// identity's declarations. A declaration of `test/vault-other`, its
/// `terminal any` included, never intends the subject's terminal state, and
/// an undefined `when` of it is never read.
#[trace("TC-519", "FR-124-AC-12")]
#[test]
fn a_subject_reads_only_the_terminals_of_its_own_identities() {
    for terminals in [
        format!("terminal Stop using v on V when {{ false }};\n{OTHER_ANY}"),
        OTHER_ANY.to_owned(),
        format!("terminal Stop using v on V when {{ false }};\n{OTHER_UNDEFINED_WHEN}"),
    ] {
        let world = two_models(&terminals).reading_vault_only();
        assert_eq!(
            terminal_class_of(&world),
            TerminalClass::Deadlocked,
            "{terminals}"
        );
        let outcome = deadlock_outcome(
            &world.package,
            &world.source,
            &world.packages,
            &|| world.binding(),
            "test/vault",
        );
        assert!(is_deadlock(&outcome), "{terminals}: {outcome:?}");
        let reading =
            fsl_route::deadlock::SubjectReading::read(world.package.graph(), |identity| {
                identity == "test/vault"
            });
        assert!(!reading.opted_out(), "{terminals}");
        assert!(reading.anchor().is_ok(), "{terminals}");
    }
}

/// FR-124-AC-12: the subject's own `terminal any` still intends its terminal
/// state, whichever alias of the identity writes it.
#[trace("TC-519", "FR-124-AC-12")]
#[test]
fn a_subjects_own_terminal_any_intends_its_terminal_state() {
    let world = two_models(&format!("{ANY}{OTHER_FALSE}")).reading_vault_only();
    assert_eq!(terminal_class_of(&world), TerminalClass::Intended);
}

/// FR-124-AC-12: a subject that reads both identities reads both identities'
/// declarations, so `Done` on `V` is read beside `Stop` on `N`.
#[trace("TC-519", "FR-124-AC-12")]
#[test]
fn a_subject_reading_both_identities_reads_both_declarations() {
    let world = two_models(&format!("{UNDEFINED_WHEN}{OTHER_FALSE}"));
    let outcome = deadlock_outcome(
        &world.package,
        &world.source,
        &world.packages,
        &|| world.binding(),
        "test/vault",
    );
    assert_eq!(
        undefined_terminal(&outcome),
        ("test/vault".to_owned(), 0, "V".to_owned())
    );
}

/// FR-124-AC-11: one identity under two aliases, `Stop` written `when
/// { true }`, settles `proved` with `Done` on either alias, and a `terminal
/// any` on the other alias leaves no deadlocked state.
#[trace("TC-519", "FR-124-AC-11")]
#[test]
fn one_identity_with_a_true_sibling_or_any_is_never_deadlocked() {
    for terminals in [
        "terminal Done using v on M when { q(1 - self.l) = 0 };\n\
         terminal Stop using v on N when { true };\n",
        "terminal Done using v on N when { q(1 - self.l) = 0 };\n\
         terminal Stop using v on M when { true };\n",
        "terminal Done using v on M when { q(1 - self.l) = 0 };\n\
         terminal Halt using v on N any;\n",
    ] {
        let world = one_identity(terminals);
        let outcome = deadlock_outcome(
            &world.package,
            &world.source,
            &world.packages,
            &|| world.binding(),
            "test/vault",
        );
        assert!(
            matches!(outcome, fsl_replay::ModelCheckOutcome::Holds { .. }),
            "{terminals}: {outcome:?}"
        );
    }
}

/// FR-124-AC-11: swapping the two declarations leaves the terminal keys and the report equal.
#[trace("TC-519", "FR-124-AC-11")]
#[test]
fn swapping_declarations_leaves_the_report_equal() {
    let done = "terminal Done using v on M when { q(1 - self.l) = 0 };\n";
    let stop = "terminal Stop using v on N when { false };\n";
    let render = |terminals: String| {
        let world = one_identity(&terminals);
        let outcome = deadlock_outcome(
            &world.package,
            &world.source,
            &world.packages,
            &|| world.binding(),
            "test/vault",
        );
        (
            world.package.graph().terminal_keys(),
            undefined_terminal(&outcome),
        )
    };
    let first = render(format!("{done}{stop}"));
    assert_eq!(first, render(format!("{stop}{done}")));
    let world = one_identity(&format!("{done}{stop}"));
    let ordinal = fsl_package::resolve_terminals(&world.package)
        .expect("the source terminal nodes resolve")
        .into_iter()
        .find(|terminal| terminal.name == "Done")
        .expect("Done resolves")
        .ordinal;
    assert_eq!(first.1, ("test/vault".to_owned(), ordinal, "M".to_owned()));
}

/// Whether `outcome` is the deadlock refutation of FR-124.
fn is_deadlock(outcome: &fsl_replay::ModelCheckOutcome) -> bool {
    matches!(
        outcome,
        fsl_replay::ModelCheckOutcome::Violated { counterexample, .. }
            if matches!(counterexample.kind, fsl_replay::CounterexampleKind::Deadlock)
    )
}

/// FR-124-AC-12: a `terminal any` on the subject's own identity opts the
/// subject out of the deadlock-freedom item; one on another identity does
/// not, and a subject that reads both identities has no anchor for the item.
#[trace("TC-519", "FR-124-AC-12")]
#[test]
fn the_deadlock_item_is_written_for_the_subjects_own_identities() {
    let vault = |identity: &str| identity == "test/vault";
    let own = two_models(ANY);
    let reading = fsl_route::deadlock::SubjectReading::read(own.package.graph(), vault);
    assert!(reading.opted_out(), "Halt any on V writes no item");
    let other = two_models(&format!("{UNDEFINED_WHEN}{OTHER_ANY}"));
    let reading = fsl_route::deadlock::SubjectReading::read(other.package.graph(), vault);
    assert!(!reading.opted_out(), "Halt any on N still writes the item");
    assert!(reading.anchor().is_ok());
    let both = fsl_route::deadlock::SubjectReading::read(other.package.graph(), |_| true);
    assert!(
        matches!(
            both.anchor(),
            Err(fsl_route::deadlock::NoAnchor::SeveralSelections { rows: 2 })
        ),
        "a subject reading both identities has no item"
    );
}

/// The derived deadlock-freedom item of the model with selection identity
/// `test/vault`, written over its subject and settled through the whole
/// request: its run, certificate check and counterexample replay
/// (`derived_model_item` and `analyze_items`), on the source route and on the
/// package route.
fn settled_records(world: &Two) -> [fsl_analyze::analyze::AnalyzeRecord; 2] {
    use fsl_analyze::analyze::{analyze_items, derived_model_item, Engines};
    use fsl_route::deadlock::ModelStateGraphItem;

    let selection = world
        .package
        .graph()
        .model_selections()
        .iter()
        .find(|selection| selection.identity == "test/vault")
        .expect("the identity is selected")
        .clone();
    let claim = OccurrenceKey::generated(
        fsl_semantics::check::model_root_key(&selection).expect("the model root"),
    );
    let form = TemporalItem {
        item_kind: ItemKind::DeadlockFreedom,
        ..TemporalItem::default()
    };
    let handoff = handoff_package(&world.package, &world.packages);
    let bare = handoff.bare();
    let input = PackageModelInput::of(&bare, &world.packages, ModelNormalizationLimits::default())
        .expect("the package route reads the terminal nodes");
    let items = [
        deadlock_model_item(
            &RunInput::source(&world.package),
            &world.binding(),
            AnalyzeLimits::unbounded(),
            &world.source,
            &claim,
            &form,
        )
        .expect("the derived target is formed"),
        PackageDeadlockSubject::new(
            &input,
            &handoff.bytes,
            handoff.supply(),
            world.binding(),
            AnalyzeLimits::unbounded(),
            &claim,
            &form,
        )
        .model_item(),
    ];
    items.map(|item| {
        let identity = fsl_replay::SubjectIdentity::of(&item.request.subject);
        let derived = crate::derived_request::derived_items(
            &[],
            &[],
            &[ModelStateGraphItem {
                subject: &identity,
                package: world.package.graph(),
            }],
            &["test/vault"],
        );
        let (request, obligation) = derived
            .iter()
            .find(|(request, _)| request.item_kind() == Some(ItemKind::DeadlockFreedom))
            .expect("a subject that reads one identity has its deadlock item");
        let run = derived_model_item(request, obligation, item).expect("the deadlock-freedom run");
        analyze_items(
            &Engines::fsl().expect("the layer-A manifests register"),
            vec![run],
            &Cancel::new(),
        )
        .remove(0)
    })
}

/// FR-124-AC-12: the derived deadlock item of a subject that reads
/// `test/vault` alone settles `refuted` through the whole request, its
/// counterexample replayed, one step from `l = 0` to the terminal state
/// `l = 1`, on the source and on the package route, whatever the other
/// identity's declarations say.
#[trace("TC-519", "FR-124-AC-12")]
#[test]
fn a_vault_only_deadlock_item_settles_refuted_and_replays_on_both_routes() {
    let terminals = format!("terminal Stop using v on V when {{ false }};\n{OTHER_ANY}");
    let world = two_models(&terminals).reading_vault_only();
    for record in settled_records(&world) {
        let (_, value) = crate::derived_request::settled(&record);
        assert_eq!(value, fsl_replay::TerminalValue::Refuted, "{record:?}");
        let found = record.model_check.expect("EN-1's record");
        let counterexample = found
            .counterexample
            .expect("the refutation's counterexample");
        assert!(
            matches!(
                counterexample.kind,
                fsl_replay::CounterexampleKind::Deadlock
            ),
            "{:?}",
            counterexample.kind
        );
        assert_eq!(counterexample.prefix.len(), 1, "l = 0 steps to l = 1");
    }
}

/// FR-124-AC-12: the vault-only deadlock item `proved` with its certificate
/// admitted, on both routes.
#[trace("TC-519", "FR-124-AC-12")]
#[test]
fn a_vault_only_deadlock_item_proves_with_an_admitted_certificate_on_both_routes() {
    let terminals = format!("terminal Done using v on V when {{ self.l = 1 }};\n{OTHER_FALSE}");
    let world = two_models(&terminals).reading_vault_only();
    for record in settled_records(&world) {
        let (_, value) = crate::derived_request::settled(&record);
        assert!(
            matches!(
                value,
                fsl_replay::TerminalValue::Proved {
                    certification: fsl_replay::Certification::Certified,
                    ..
                }
            ),
            "{value:?}"
        );
    }
}

/// FR-124-AC-12 reads only the derived deadlock item over the identities a
/// subject covers. Every other system is built over every selection, so over
/// a unit whose universes cover `test/vault` alone, an authored invariant of
/// `test/vault-other` is refused for want of a bound exactly as the unit's
/// own subject covering both would be, never settled `proved` over an empty
/// context; and a snapshot that lists the uncovered identity's population is
/// admitted.
#[trace("TC-519", "FR-124-AC-12")]
#[test]
fn an_uncovered_identitys_invariant_is_refused_and_its_snapshot_admitted() {
    use fsl_analyze::model_check::{check_model, ModelCheckRefusal, ModelCheckRequest};
    use fsl_eval::model_subject::ModelSubject;
    use fsl_replay::{InvariantClause, ModelCheckItem, ModelCheckLimits};

    let terminals = "invariant OtherSmall using v on N::Vault at current { self.l <= 0 }\n";
    // The snapshot lists both populations; the universes cover the vault.
    let world = two_models(terminals);
    let mut covered = world.simulation(fsl_eval::simulation::model::ModelCheckInput::source(
        &world.package,
    ));
    covered.universes = vec![PopulationUniverse {
        population: key("vaults"),
        keys: vec!["v".to_owned()],
    }];
    assert!(
        ModelSystem::new(covered.clone()).is_ok(),
        "a snapshot listing the uncovered identity's population is admitted"
    );
    let clause = world
        .package
        .graph()
        .state_clauses()
        .iter()
        .find(|clause| clause.name() == "OtherSmall")
        .expect("the invariant is checked");
    let outcome = check_model(
        ModelCheckRequest {
            temporal_clock_selections: &[],
            subject: ModelSubject {
                simulation: covered,
                proof_bounds: Vec::new(),
            },
            item: ModelCheckItem::Invariant {
                clause: InvariantClause::of(clause),
            },
            max_depth: None,
            limits: ModelCheckLimits::default(),
            reductions: Default::default(),
        },
        &Cancel::new(),
    );
    assert!(
        matches!(outcome, Err(ModelCheckRefusal::RequiresBound(_))),
        "{outcome:?}"
    );
}

/// The items the document entry settles for `world` over the request's
/// `items`, or over FR-620's default items when there are none.
fn settled_items(world: &Two, items: Option<serde_json::Value>) -> Vec<serde_json::Value> {
    use fsl_analyze::analyze::document::{
        resolve_items, settle_run, AnalyzeRun, ItemSelection, SubjectSelection,
    };
    let engines = fsl_analyze::analyze::Engines::fsl().expect("the manifests register");
    let mut registry = fsl_route::Registry::new();
    for descriptor in engines.registrations() {
        registry
            .register(descriptor.clone())
            .expect("a registered backend");
    }
    let selections: Option<Vec<ItemSelection>> =
        items.map(|items| serde_json::from_value(items).expect("the items are selections"));
    let binding = world.binding();
    let units = [world.package.graph().source().clone()];
    let resolved = resolve_items(
        &world.package,
        &SubjectSelection::default(),
        selections.as_deref(),
        &registry,
        &units,
        &binding,
        &AnalyzeLimits::unbounded(),
    );
    settle_run(
        AnalyzeRun {
            package: &world.package,
            binding,
            provision: SourceProvision::single(
                world.package.graph().source(),
                world.text.as_bytes(),
            ),
            items: resolved,
            engines: &engines,
            limits: AnalyzeLimits::unbounded(),
        },
        &crate::analyze_cli_support::Fr290,
        &Cancel::new(),
    )
    .map(|settled| settled.items)
    .expect("a settled run")
}

/// The request item that names the deadlock-freedom item of the model `V`.
fn deadlock_of_vault() -> serde_json::Value {
    serde_json::json!([{"claim": "V", "temporal": {"item_kind": "deadlock-freedom"}}])
}

/// FR-124-AC-12, TC-519 step 11: a request whose subject reads one identity
/// of a unit that selects two models derives the deadlock-freedom item for
/// that identity, so the item settles `refuted` with the one-step deadlock,
/// by the item the request names and by FR-620's default items alike.
#[trace("TC-519", "FR-124-AC-12")]
#[test]
fn a_subject_reading_one_identity_derives_the_deadlock_item_of_that_identity() {
    let world = two_models("").reading_vault_only();
    for items in [Some(deadlock_of_vault()), None] {
        let settled = settled_items(&world, items.clone());
        let [item] = settled.as_slice() else {
            panic!("{items:?}: one derived item: {settled:?}");
        };
        assert_eq!(item["record"]["result"], "refuted", "{item}");
        assert_eq!(item["replay"]["reproduced"], true, "{item}");
        assert_eq!(
            item["counterexample"]["prefix"].as_array().map(Vec::len),
            Some(1),
            "the vault deadlocks after one step: {item}"
        );
    }
}

/// FR-124-AC-12: another selection's `terminal any` is no opt-out of the
/// item of the identity the subject reads, so the item is still written and
/// still refuted.
#[trace("TC-519", "FR-124-AC-12")]
#[test]
fn another_identitys_terminal_any_does_not_opt_the_read_identity_out() {
    let world = two_models(OTHER_ANY).reading_vault_only();
    for items in [Some(deadlock_of_vault()), None] {
        let settled = settled_items(&world, items.clone());
        let [item] = settled.as_slice() else {
            panic!("{items:?}: one derived item: {settled:?}");
        };
        assert_eq!(item["record"]["result"], "refuted", "{item}");
    }
}

/// FSpec FR-368-AC-22: a subject that reads both identities has no anchor for
/// a derived item, so the item the request names settles `unsupported` and
/// FR-620's default items write none.
#[trace("TC-431", "FR-368-AC-22")]
#[test]
fn a_subject_reading_both_identities_has_no_derived_item() {
    let world = two_models("");
    let settled = settled_items(&world, Some(deadlock_of_vault()));
    let [item] = settled.as_slice() else {
        panic!("one item: {settled:?}");
    };
    assert_eq!(item["record"]["result"], "unsupported", "{item}");
    assert_eq!(item["record"]["settled_by"], "disposition", "{item}");
    assert!(
        settled_items(&world, None).is_empty(),
        "the default items write none"
    );
}

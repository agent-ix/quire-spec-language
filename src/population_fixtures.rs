// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-771 (FR-093-AC-19, TC-416 step 11): the `Population<T>[N]` Contract IR
//! v2 packages the admission corpus emits, as files.
//!
//! `with_population_package` builds one package over the TC-442 spine model
//! whose function `pop` takes a `Population<M::Gadget>[N]` parameter for each
//! of its bounds, and [`fixture`] emits it through the real v2 emitter and reads
//! it back through IR's checked-package v2 reader. [`write()`] writes those
//! bytes, for [`CASES`], into a new directory so IR's conformance gate can
//! read them without depending on QSL.
//! The `population_v2_fixtures` example is a thin caller of [`write()`].
//!
//! Beside each case the directory holds the domain package document the
//! reader needs as evidence, under [`DOMAIN_PACKAGE_FILE`].
//!
//! The [`VARIANTS`] are the packages Contract IR's TC-453 (FR-053) reads
//! beyond those cases, written to the same directory: two same-type bindings
//! beside a binding of a second object type, the same package regenerated
//! with one changed maximum, and that package with one parameter's
//! `semantic_type` retargeted to the other same-type population node. The
//! first two come from the real emitter. The third is that emitter's package
//! edited as typed IR values, re-keyed with IR's own public identity
//! derivations and re-encoded with IR's public encoder, then read by IR's
//! public reader; nothing of IR is copied.

use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
};

use qsl_forms::{BuiltinType, Expression, FunctionDeclaration, TypeForm, TypeFormHead};
use qsl_foundation::{source::provenance::SourceRegion, Span};
use qsl_package::{read_import_view, AdmittedPackages, CheckedPackage, V2ReadLimits};
use qsl_semantics::{
    check::PackageDeclarations,
    library::{LibraryName, PackageId},
};
use quire_contract_model::{
    derive_application_node_id, derive_structural_candidates, encode_checked_package_v2,
    ApplicationNodeDraft, BoundedDomainForm, CheckedNodeId, CheckedNodeKind,
    CheckedPackageEvidence, CheckedPackageReadLimits, CheckedPackageV2, CheckedPackageV2ReadResult,
    CheckedSemanticGraphV2, CheckedSemanticId, CheckedSemanticNodeV2, CheckedSourceMapEntry,
};
use quire_semantic_value::checking::CheckingLimits;

/// The domain package document `spine-model.native`'s `model M` selects.
const DOMAIN_PACKAGE: &[u8] = include_bytes!("../tests/fixtures/spine-model.semantic-ir.json");

/// The TC-442 spine-model unit every case is assembled against.
const SPINE_MODEL_UNIT: &str = include_str!("../tests/fixtures/spine-model.native");

/// The file the domain package document is written to.
pub const DOMAIN_PACKAGE_FILE: &str = "spine-model.semantic-ir.json";

/// The bounds of each case: N = 0, N = 1, N = 3, and two populations with
/// distinct bounds.
pub const CASES: [&[u64]; 4] = [&[0], &[1], &[3], &[2, 5]];

/// A TC-453 variant: a package beyond [`CASES`] that Contract IR's
/// population-binding selection reads.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Variant {
    /// Contract IR TC-453 step 1: `Population<M::Gadget>[0]`, `Population<M::Gadget>[5]`
    /// and `Population<M::Widget>[3]`, one parameter each.
    SameTypeAndOtherType,
    /// Contract IR TC-453 step 3: [`Self::SameTypeAndOtherType`] regenerated with the
    /// `Gadget[5]` maximum changed to 6 and nothing else.
    ChangedMax,
    /// Contract IR TC-453 step 5: [`Self::SameTypeAndOtherType`] with the `Gadget[0]`
    /// parameter's `semantic_type` retargeted to the `Gadget[5]` population
    /// node, identities and `package_id` rederived, source rows kept.
    RetargetedParameter,
}

/// Every [`Variant`], in the order [`write()`] emits them.
pub const VARIANTS: [Variant; 3] = [
    Variant::SameTypeAndOtherType,
    Variant::ChangedMax,
    Variant::RetargetedParameter,
];

impl Variant {
    /// The file this variant's package is written to.
    pub const fn file_name(self) -> &'static str {
        match self {
            Self::SameTypeAndOtherType => "population-variant-same-type.json",
            Self::ChangedMax => "population-variant-changed-max.json",
            Self::RetargetedParameter => "population-variant-retargeted.json",
        }
    }

    const fn bindings(self) -> &'static [Binding] {
        match self {
            Self::SameTypeAndOtherType | Self::RetargetedParameter => &SAME_TYPE_BINDINGS,
            Self::ChangedMax => &CHANGED_MAX_BINDINGS,
        }
    }
}

/// One `Population<object>[bound]` parameter.
#[derive(Clone, Copy)]
struct Binding {
    object: &'static str,
    bound: u64,
}

const fn binding(object: &'static str, bound: u64) -> Binding {
    Binding { object, bound }
}

/// [`Variant::SameTypeAndOtherType`]'s parameters, `p0`, `p1`, `p2`.
const SAME_TYPE_BINDINGS: [Binding; 3] = [
    binding("M::Gadget", 0),
    binding("M::Gadget", 5),
    binding("M::Widget", 3),
];

/// [`Variant::ChangedMax`]'s parameters: `p1`'s maximum differs.
const CHANGED_MAX_BINDINGS: [Binding; 3] = [
    binding("M::Gadget", 0),
    binding("M::Gadget", 6),
    binding("M::Widget", 3),
];

/// Why [`write()`] or [`fixture`] produced no complete fixture set.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The output path already exists, as a directory, file or link.
    #[error("output path {path} already exists; the output directory must be new")]
    OutputExists {
        /// The path the caller named.
        path: PathBuf,
    },
    /// A filesystem operation on `path` failed.
    #[error("cannot access {path}: {source}")]
    Io {
        /// The path the operation named.
        path: PathBuf,
        /// The underlying I/O failure.
        #[source]
        source: io::Error,
    },
    /// The spine-model unit did not parse, build, select its model or assemble.
    #[error("the population fixture unit was refused at {stage}: {detail}")]
    Assemble {
        /// The stage that refused.
        stage: &'static str,
        /// The stage's refusal, rendered.
        detail: String,
    },
    /// The assembled package did not check.
    #[error("the population fixture package did not check: {0}")]
    Check(String),
    /// The v2 emitter refused the checked package.
    #[error("the v2 emitter refused the population fixture package: {0}")]
    Emit(String),
    /// IR's reader refused the emitted bytes.
    #[error("IR's checked-package v2 reader refused the population fixture package: {0}")]
    Admission(String),
    /// The retargeted package could not be built from the emitted one.
    #[error("the retargeted population package could not be built: {0}")]
    Retarget(RetargetRefusal),
    /// The emitter omitted nodes, so the bytes are not the whole package.
    #[error("the v2 emitter omitted {omitted} nodes of the population fixture package")]
    Omitted {
        /// How many nodes the emission left out.
        omitted: usize,
    },
}

/// Why the emitted package could not become the retargeted one.
#[derive(Debug, thiserror::Error)]
pub enum RetargetRefusal {
    /// The package has no single population node with this maximum.
    #[error("{found} population nodes have max {max}, expected one")]
    PopulationNodes {
        /// The maximum looked for.
        max: u64,
        /// How many population nodes carry it.
        found: usize,
    },
    /// The package has no single parameter typed by the first population.
    #[error("{found} parameters are typed by the first population node, expected one")]
    ParameterNodes {
        /// How many parameters name it.
        found: usize,
    },
    /// IR refused to derive a node identity.
    #[error("IR's identity derivation refused: {0}")]
    Derivation(String),
    /// Re-keying renamed nodes without settling.
    #[error("re-keying did not settle within {renames} renames")]
    Unsettled {
        /// The rename budget that ran out.
        renames: usize,
    },
    /// The canonical encoder refused.
    #[error("canonical encoding refused: {0}")]
    Encode(String),
}

/// The file `bounds`' package is written to: `population-2-5.json` for
/// `[2, 5]`.
pub fn file_name(bounds: &[u64]) -> String {
    let bounds: Vec<String> = bounds.iter().map(u64::to_string).collect();
    format!("population-{}.json", bounds.join("-"))
}

/// A package over the TC-442 spine model whose one function `pop` takes a
/// `Population<object>[N]` parameter for each of `bindings`, built from
/// source forms (source text spells no `Population` type), handed to `with`
/// (the stage types are minted only by their named constructors, so the
/// package is lent, not returned).
fn with_population_package<T>(
    bindings: &[Binding],
    with: impl FnOnce(&CheckedPackage) -> Result<T, Error>,
) -> Result<T, Error> {
    let span = Span { start: 0, end: 0 };
    let parameters = bindings
        .iter()
        .enumerate()
        .map(|(at, binding)| {
            let population = TypeForm::new(TypeFormHead::Population, span)
                .with_arguments(vec![TypeForm::name(binding.object, span)])
                .with_bounds(vec![binding.bound.to_string()]);
            (format!("p{at}"), population)
        })
        .collect();
    let pop = FunctionDeclaration::new(
        "pop",
        parameters,
        TypeForm::builtin(BuiltinType::Boolean, span),
        None,
        Expression::boolean(true),
    );
    let packages = qsl_semantics::model::intake::package_input([DOMAIN_PACKAGE]);
    let mut declarations = assemble(&packages)?;
    declarations.functions.extend([pop]);
    let graph = declarations
        .check(CheckingLimits::default())
        .map_err(|refusal| Error::Check(format!("{refusal:?}")))?;
    with(&CheckedPackage::link(graph))
}

/// The spine-model unit run S1, S2, I1 (against `packages`) and the
/// assembler, as spine `compile` runs it.
fn assemble(packages: &BTreeMap<[u8; 32], Vec<u8>>) -> Result<PackageDeclarations, Error> {
    let stage = |stage| move |detail: String| Error::Assemble { stage, detail };
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("agent-ix", "test:spine-model", "fixture", "fixture:1"),
        "program.native",
        SPINE_MODEL_UNIT.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .map_err(|refusal| stage("S1")(format!("{refusal:?}")))?;
    let raw = parsed.source().reference().clone();
    let unit =
        qsl_forms::build_unit(&parsed).map_err(|refusal| stage("S2")(format!("{refusal:?}")))?;
    let models = qsl_semantics::model::intake::admit_unit(
        &unit.selections().models,
        packages,
        qsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .map_err(|refusal| stage("I1")(format!("{refusal:?}")))?;
    PackageDeclarations::assemble(raw, unit, models, Vec::new())
        .map_err(|refusal| stage("assemble")(format!("{refusal:?}")))
}

/// One case: the v2 bytes the emitter wrote for a bound list and the
/// `package_id` IR's reader admitted them at.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Fixture {
    /// The `quire.checked-package/v2` bytes.
    pub bytes: Vec<u8>,
    /// The `package_id` the checked-package v2 reader admitted `bytes` at.
    pub package_id: PackageId,
    /// Each graph node's family and form as IR's reader decoded it.
    pub node_kinds: Vec<CheckedNodeKind>,
}

/// `bounds`' package through the v2 emitter, with nothing omitted, and read
/// back through QSL's I2 read, which runs IR's checked-package v2 reader and
/// refuses unless it admits the bytes at the emitted `package_id`.
pub fn fixture(bounds: &[u64]) -> Result<Fixture, Error> {
    let bindings: Vec<Binding> = bounds
        .iter()
        .map(|bound| binding("M::Gadget", *bound))
        .collect();
    with_population_package(&bindings, |package| Ok(emit_and_read(package)?.fixture))
}

/// A [`Variant`]'s package as the bytes IR's reader admitted and the typed
/// package it admitted them as.
#[derive(Clone, Debug)]
pub struct VariantFixture {
    /// The `quire.checked-package/v2` bytes.
    pub bytes: Vec<u8>,
    /// The package IR's checked-package v2 reader admitted `bytes` as.
    pub package: CheckedPackageV2,
}

/// `variant`'s package. The first two variants are emitted and read back as
/// [`fixture`] does; the third is the first with a parameter retargeted and
/// read by IR's public reader.
pub fn variant_fixture(variant: Variant) -> Result<VariantFixture, Error> {
    with_population_package(variant.bindings(), |package| {
        let read = emit_and_read(package)?;
        match variant {
            Variant::SameTypeAndOtherType | Variant::ChangedMax => Ok(VariantFixture {
                bytes: read.fixture.bytes,
                package: read.admitted,
            }),
            Variant::RetargetedParameter => retarget(&read.admitted, read.evidence),
        }
    })
}

/// What [`emit_and_read`] produced.
struct EmittedRead {
    fixture: Fixture,
    admitted: CheckedPackageV2,
    /// The evidence the reader was given, for reading edited bytes.
    evidence: CheckedPackageEvidence,
}

/// [`fixture`] over `package`.
fn emit_and_read(package: &CheckedPackage) -> Result<EmittedRead, Error> {
    // The forms carry no spans, so every occurrence is placed at the whole
    // of the unit the package was checked from.
    let source = package.graph().source().clone();
    let length = u64::try_from(SPINE_MODEL_UNIT.len()).unwrap_or(u64::MAX);
    let emission = qsl_package::emit_package(package, |_| {
        SourceRegion::new(source.clone(), 0, length).ok()
    })
    .map_err(|refusal| Error::Emit(format!("{refusal:?}")))?;
    if !emission.omitted().is_empty() {
        return Err(Error::Omitted {
            omitted: emission.omitted().len(),
        });
    }
    let library =
        LibraryName::new("pkg").map_err(|refusal| Error::Admission(format!("{refusal:?}")))?;
    let domain_packages = qsl_semantics::model::intake::package_input([DOMAIN_PACKAGE]);
    let mut admitted = AdmittedPackages::default();
    let view = read_import_view(
        package,
        &emission,
        library,
        &domain_packages,
        &mut admitted,
        V2ReadLimits::default(),
    )
    .map_err(|refusal| Error::Admission(format!("{refusal:?}")))?;
    let package_id = view.package();
    let admitted = admitted
        .package(&package_id)
        .ok_or_else(|| Error::Admission("the read recorded no admitted package".to_owned()))?
        .clone();
    let mut evidence = emission.evidence().clone();
    for (digest, document) in &domain_packages {
        let digest: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
        evidence
            .insert_domain_package_document(digest, document.as_slice())
            .map_err(|conflict| Error::Admission(format!("{conflict:?}")))?;
    }
    Ok(EmittedRead {
        fixture: Fixture {
            bytes: emission.package().bytes().to_vec(),
            package_id,
            node_kinds: admitted.node_kinds().to_vec(),
        },
        admitted,
        evidence,
    })
}

/// The `max` of a population node's body, when it has one.
fn population_max(node: &CheckedSemanticNodeV2) -> Option<u64> {
    node.body
        .get("members")?
        .as_array()?
        .iter()
        .find(|member| member.get("name").and_then(|name| name.as_str()) == Some("max"))?
        .get("value")?
        .get("value")?
        .as_str()?
        .parse()
        .ok()
}

/// The ceiling on canonical bytes and visits of the identity derivations.
const DERIVATION_BYTES: u64 = u64::MAX;
const DERIVATION_WORK: u64 = u64::MAX;

/// `base` with the `semantic_type` of the parameter typed by the population
/// of the first same-type binding pointed at the population of the second,
/// every identity that change moves rederived, and the result read back by
/// IR's reader under `evidence`.
fn retarget(
    base: &CheckedPackageV2,
    evidence: CheckedPackageEvidence,
) -> Result<VariantFixture, Error> {
    let refuse = |refusal| Error::Retarget(refusal);
    let [first, second, ..] = SAME_TYPE_BINDINGS;
    let mut nodes = base.graph().nodes.clone();
    let population = |max: u64| -> Result<CheckedNodeId, Error> {
        let found: Vec<&CheckedSemanticNodeV2> = nodes
            .iter()
            .zip(base.node_kinds())
            .filter(|(node, kind)| {
                **kind == CheckedNodeKind::BoundedDomain(BoundedDomainForm::ModelPopulation)
                    && population_max(node) == Some(max)
            })
            .map(|(node, _)| node)
            .collect();
        match found.as_slice() {
            [one] => Ok(one.node_id.clone()),
            _ => Err(refuse(RetargetRefusal::PopulationNodes {
                max,
                found: found.len(),
            })),
        }
    };
    let (from, to) = (population(first.bound)?, population(second.bound)?);
    let mut typed_by_first = nodes
        .iter_mut()
        .filter(|node| &*node.node_tag == "value" && &*node.semantic_form == "parameter")
        .filter(|node| node.semantic_type == from);
    let (Some(parameter), None) = (typed_by_first.next(), typed_by_first.next()) else {
        let found = nodes
            .iter()
            .filter(|node| node.semantic_type == from)
            .count();
        return Err(refuse(RetargetRefusal::ParameterNodes { found }));
    };
    parameter.semantic_type = to;

    let mut source_map = base.source_map().to_vec();
    rekey(&mut nodes, &mut source_map).map_err(refuse)?;
    nodes.sort_by(|left, right| left.node_id.digest.cmp(&right.node_id.digest));
    source_map.sort_by(|left, right| left.node_id.digest.cmp(&right.node_id.digest));

    let limits = quire_canonical::Limits::new(u64::MAX);
    let mut preimage = base.identity_preimage().clone();
    preimage.identity_projection = nodes.iter().map(Into::into).collect();
    let digest = quire_canonical::sha256(&preimage, limits)
        .map_err(|error| refuse(RetargetRefusal::Encode(error.to_string())))?;
    let package_id = CheckedSemanticId {
        digest: digest
            .as_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
            .into(),
        ..base.package_id().clone()
    };
    let graph = CheckedSemanticGraphV2 {
        graph_version: base.graph().graph_version.clone(),
        nodes,
    };
    let bytes = encode_checked_package_v2(
        &preimage,
        &package_id,
        base.lock(),
        &graph,
        &source_map,
        base.capability_report(),
        base.diagnostics(),
        limits,
    )
    .map_err(|error| refuse(RetargetRefusal::Encode(error.to_string())))?;
    match CheckedPackageV2::read(&bytes, CheckedPackageReadLimits::bounded(), &evidence) {
        CheckedPackageV2ReadResult::Admitted(package) => Ok(VariantFixture {
            bytes,
            package: *package,
        }),
        other => Err(Error::Admission(format!("{other:?}"))),
    }
}

/// Renames every node whose retained key is not the one IR derives for it to
/// the derived key, wherever the old key is named, until none is left.
fn rekey(
    nodes: &mut [CheckedSemanticNodeV2],
    source_map: &mut [CheckedSourceMapEntry],
) -> Result<(), RetargetRefusal> {
    let budget = nodes.len().saturating_mul(nodes.len()).saturating_add(1);
    for _ in 0..budget {
        let derived = derived_keys(nodes)?;
        let stale = nodes
            .iter()
            .zip(derived)
            .find(|(node, derived)| *derived != *node.node_id.digest);
        let Some((node, new)) = stale else {
            return Ok(());
        };
        let old = node.node_id.clone();
        rename(nodes, source_map, &old, &new);
    }
    Err(RetargetRefusal::Unsettled { renames: budget })
}

/// The key IR derives for each of `nodes`, in order: structural candidates
/// for the structural families, the application derivation for the rest.
fn derived_keys(nodes: &[CheckedSemanticNodeV2]) -> Result<Vec<String>, RetargetRefusal> {
    let derivation =
        |error: &dyn std::fmt::Debug| RetargetRefusal::Derivation(format!("{error:?}"));
    let structural = derive_structural_candidates(nodes, DERIVATION_BYTES, DERIVATION_WORK)
        .map_err(|error| derivation(&error))?;
    let mut derived: Vec<Option<String>> = vec![None; nodes.len()];
    for candidate in structural.nodes() {
        let slot = usize::try_from(candidate.position())
            .ok()
            .and_then(|position| derived.get_mut(position));
        if let Some(slot) = slot {
            *slot = Some(candidate.candidate_node_id().digest.to_string());
        }
    }
    nodes
        .iter()
        .zip(derived)
        .map(|(node, structural)| match structural {
            Some(digest) => Ok(digest),
            None => {
                derive_application_node_id(ApplicationNodeDraft::new(node), None, DERIVATION_BYTES)
                    .map(|candidate| candidate.candidate_node_id().digest.to_string())
                    .map_err(|error| derivation(&error))
            }
        })
        .collect()
}

/// Names `new` for `old` in every node's identity, type, dependencies and
/// body, and in the source map; dependencies are kept in digest order.
fn rename(
    nodes: &mut [CheckedSemanticNodeV2],
    source_map: &mut [CheckedSourceMapEntry],
    old: &CheckedNodeId,
    new: &str,
) {
    let renamed = |id: &mut CheckedNodeId| {
        if *id == *old {
            id.digest = new.into();
        }
    };
    for node in nodes {
        renamed(&mut node.node_id);
        renamed(&mut node.semantic_type);
        node.dependencies.iter_mut().for_each(renamed);
        node.dependencies
            .sort_by(|left, right| left.digest.cmp(&right.digest));
        let mut pending = vec![&mut node.body];
        while let Some(term) = pending.pop() {
            match term {
                serde_json::Value::Object(members) => {
                    let names_old = members.get("domain").and_then(|domain| domain.as_str())
                        == Some(&*old.domain)
                        && members.get("digest").and_then(|digest| digest.as_str())
                            == Some(&*old.digest);
                    if names_old {
                        members.insert("digest".to_owned(), new.into());
                    }
                    pending.extend(members.values_mut());
                }
                serde_json::Value::Array(items) => pending.extend(items.iter_mut()),
                _ => {}
            }
        }
    }
    for row in source_map {
        renamed(&mut row.node_id);
    }
}

/// Writes the v2 package of every case of [`CASES`] and variant of
/// [`VARIANTS`] and the domain package document into `directory`, which must not exist. Every package is emitted
/// before the directory is created, so an emitter refusal leaves nothing
/// behind.
pub fn write(directory: &Path) -> Result<(), Error> {
    let mut files = vec![(DOMAIN_PACKAGE_FILE.to_owned(), DOMAIN_PACKAGE.to_vec())];
    for bounds in CASES {
        files.push((file_name(bounds), fixture(bounds)?.bytes));
    }
    for variant in VARIANTS {
        files.push((
            variant.file_name().to_owned(),
            variant_fixture(variant)?.bytes,
        ));
    }
    fs::create_dir(directory).map_err(|source| match source.kind() {
        io::ErrorKind::AlreadyExists => Error::OutputExists {
            path: directory.to_owned(),
        },
        _ => io_at(directory, source),
    })?;
    for (name, bytes) in files {
        let path = directory.join(name);
        fs::write(&path, bytes).map_err(|source| io_at(&path, source))?;
    }
    Ok(())
}

fn io_at(path: &Path, source: io::Error) -> Error {
    Error::Io {
        path: path.to_owned(),
        source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    use quire_contract_model::{
        BoundedDomainForm, CheckedNodeId, CheckedOccurrenceRole, CheckedPackageV2,
        CheckedSemanticNodeV2,
    };

    fn written() -> (tempfile::TempDir, PathBuf) {
        let parent = tempfile::tempdir().unwrap();
        let output = parent.path().join("population");
        write(&output).expect("the fixture set is written");
        (parent, output)
    }

    #[trace("FR-093-AC-19", "TC-416")]
    #[test]
    fn writes_every_case_into_a_new_directory() {
        let (_parent, output) = written();
        let mut files: Vec<String> = fs::read_dir(&output)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        files.sort();
        let mut expected: Vec<String> = CASES.iter().map(|bounds| file_name(bounds)).collect();
        expected.extend(
            VARIANTS
                .iter()
                .map(|variant| variant.file_name().to_owned()),
        );
        expected.push(DOMAIN_PACKAGE_FILE.to_owned());
        expected.sort();
        assert_eq!(files, expected);
    }

    #[trace("FR-093-AC-19", "TC-416")]
    #[test]
    fn written_bytes_equal_the_emitted_package_and_are_admitted_by_ir() {
        let (_parent, output) = written();
        for bounds in CASES {
            // `fixture` fails unless IR's reader admits the bytes.
            let fixture =
                super::fixture(bounds).unwrap_or_else(|refusal| panic!("{bounds:?}: {refusal}"));
            let on_disk = fs::read(output.join(file_name(bounds))).unwrap();
            assert_eq!(on_disk, fixture.bytes, "{bounds:?}");
            assert_eq!(
                fixture,
                super::fixture(bounds).unwrap(),
                "{bounds:?}: emission is deterministic"
            );
        }
        assert_eq!(
            fs::read(output.join(DOMAIN_PACKAGE_FILE)).unwrap(),
            DOMAIN_PACKAGE
        );
    }

    /// TC-416 step 11: IR's reader decodes one `model_population` bounded
    /// domain per bound.
    #[trace("FR-093-AC-19", "TC-416")]
    #[test]
    fn ir_admits_one_model_population_node_per_bound() {
        for bounds in CASES {
            let admitted = fixture(bounds)
                .unwrap()
                .node_kinds
                .iter()
                .filter(|kind| {
                    **kind == CheckedNodeKind::BoundedDomain(BoundedDomainForm::ModelPopulation)
                })
                .count();
            assert_eq!(admitted, bounds.len(), "{bounds:?}");
        }
    }

    /// Each population type node is a `model_population` node carrying role
    /// `type` and only that role, with the declared bound as its `max`.
    #[trace("FR-093-AC-19", "TC-416")]
    #[test]
    fn each_bound_is_a_typed_population_node() {
        for bounds in CASES {
            let wire: serde_json::Value =
                serde_json::from_slice(&fixture(bounds).unwrap().bytes).unwrap();
            let nodes = wire["semantic_graph"]["nodes"].as_array().unwrap();
            let mut maxima: Vec<u64> = Vec::new();
            for node in nodes.iter().filter(|node| {
                node["node_tag"] == "bounded_domain" && node["semantic_form"] == "model_population"
            }) {
                let roles: Vec<&str> = node["occurrences"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|occurrence| occurrence["role"].as_str().unwrap())
                    .collect();
                assert_eq!(roles, ["type"], "{bounds:?}");
                let max = node["body"]["members"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|member| member["name"] == "max")
                    .unwrap_or_else(|| panic!("{bounds:?}: no max in {}", node["body"]));
                maxima.push(max["value"]["value"].as_str().unwrap().parse().unwrap());
            }
            maxima.sort_unstable();
            assert_eq!(maxima, bounds, "{bounds:?}");
        }
    }

    /// The population nodes of an admitted package, as `(node, max)`.
    fn populations(package: &CheckedPackageV2) -> Vec<(&CheckedSemanticNodeV2, u64)> {
        package
            .graph()
            .nodes
            .iter()
            .zip(package.node_kinds())
            .filter(|(_, kind)| {
                **kind == CheckedNodeKind::BoundedDomain(BoundedDomainForm::ModelPopulation)
            })
            .map(|(node, _)| {
                (
                    node,
                    population_max(node).expect("a population node has a max"),
                )
            })
            .collect()
    }

    fn maxima(package: &CheckedPackageV2) -> Vec<u64> {
        let mut maxima: Vec<u64> = populations(package).iter().map(|(_, max)| *max).collect();
        maxima.sort_unstable();
        maxima
    }

    fn population_with_max(package: &CheckedPackageV2, max: u64) -> &CheckedSemanticNodeV2 {
        let found: Vec<_> = populations(package)
            .into_iter()
            .filter(|(_, bound)| *bound == max)
            .collect();
        assert_eq!(found.len(), 1, "one population has max {max}");
        found[0].0
    }

    fn node<'a>(package: &'a CheckedPackageV2, id: &CheckedNodeId) -> &'a CheckedSemanticNodeV2 {
        package
            .graph()
            .nodes
            .iter()
            .find(|node| node.node_id == *id)
            .expect("the id names a node of the package")
    }

    /// The object type a population ranges over: the dependency of the set
    /// type the population node is typed by.
    fn object_type(
        package: &CheckedPackageV2,
        population: &CheckedSemanticNodeV2,
    ) -> CheckedNodeId {
        let set = node(package, &population.semantic_type);
        assert_eq!(set.dependencies.len(), 1);
        set.dependencies[0].clone()
    }

    /// The parameters typed by a population node: those of `pop`.
    fn parameters(package: &CheckedPackageV2) -> Vec<&CheckedSemanticNodeV2> {
        let population_ids: Vec<&CheckedNodeId> = populations(package)
            .into_iter()
            .map(|(population, _)| &population.node_id)
            .collect();
        package
            .graph()
            .nodes
            .iter()
            .filter(|node| &*node.node_tag == "value" && &*node.semantic_form == "parameter")
            .filter(|node| population_ids.contains(&&node.semantic_type))
            .collect()
    }

    fn has_type_row(package: &CheckedPackageV2, id: &CheckedNodeId) -> bool {
        package.source_map().iter().any(|row| {
            row.node_id == *id && row.role == CheckedOccurrenceRole::Type && row.ordinal == 0
        })
    }

    /// Contract IR TC-453 step 1: two bindings of one object type and one of another,
    /// each its own node with its own `Type` occurrence and source row.
    #[trace("FR-093-AC-19", "TC-416")]
    #[test]
    fn two_same_type_bindings_and_a_second_object_type_are_admitted() {
        let package = variant_fixture(Variant::SameTypeAndOtherType)
            .unwrap()
            .package;
        assert_eq!(maxima(&package), [0, 3, 5]);
        let first = population_with_max(&package, 0);
        let second = population_with_max(&package, 5);
        let other = population_with_max(&package, 3);
        assert_ne!(first.node_id, second.node_id);
        assert_eq!(object_type(&package, first), object_type(&package, second));
        assert_ne!(object_type(&package, first), object_type(&package, other));
        for population in [first, second, other] {
            assert_eq!(population.occurrences.len(), 1);
            assert!(has_type_row(&package, &population.node_id));
        }
        assert_eq!(parameters(&package).len(), 3);
    }

    /// Contract IR TC-453 step 3: the regenerated package differs only in one source
    /// maximum, and the node and extent that maximum names change with it.
    #[trace("FR-093-AC-19", "TC-416")]
    #[test]
    fn a_changed_maximum_regenerates_an_admitted_package() {
        let base = variant_fixture(Variant::SameTypeAndOtherType).unwrap();
        let changed = variant_fixture(Variant::ChangedMax).unwrap();
        assert_eq!(maxima(&changed.package), [0, 3, 6]);
        assert_ne!(base.bytes, changed.bytes);
        assert_ne!(base.package.package_id(), changed.package.package_id());
        // The zero and the other-type bindings keep their nodes.
        for max in [0, 3] {
            assert_eq!(
                population_with_max(&base.package, max).node_id,
                population_with_max(&changed.package, max).node_id
            );
        }
        let moved = population_with_max(&changed.package, 6);
        assert_ne!(moved.node_id, population_with_max(&base.package, 5).node_id);
        assert!(has_type_row(&changed.package, &moved.node_id));
    }

    /// Contract IR TC-453 step 5: one parameter's `semantic_type` is retargeted from the
    /// first population node to the second; identities are rederived, the
    /// original population nodes and source rows are kept, and IR's reader
    /// admits the package.
    #[trace("FR-093-AC-19", "TC-416")]
    #[test]
    fn a_retargeted_parameter_is_rederived_and_admitted() {
        let base = variant_fixture(Variant::SameTypeAndOtherType).unwrap();
        let retargeted = variant_fixture(Variant::RetargetedParameter).unwrap();
        let (base, retargeted) = (&base.package, &retargeted.package);
        let first = population_with_max(base, 0).node_id.clone();
        let second = population_with_max(base, 5).node_id.clone();
        // Both population nodes and their source rows are the original ones.
        assert_eq!(population_with_max(retargeted, 0).node_id, first);
        assert_eq!(population_with_max(retargeted, 5).node_id, second);
        assert!(has_type_row(retargeted, &first));
        assert!(has_type_row(retargeted, &second));
        // Before: one parameter per population. After: none names the first
        // and two name the second.
        let pointing_at = |package: &CheckedPackageV2, id: &CheckedNodeId| {
            parameters(package)
                .iter()
                .filter(|parameter| parameter.semantic_type == *id)
                .count()
        };
        assert_eq!(
            (pointing_at(base, &first), pointing_at(base, &second)),
            (1, 1)
        );
        assert_eq!(
            (
                pointing_at(retargeted, &first),
                pointing_at(retargeted, &second)
            ),
            (0, 2)
        );
        assert_ne!(base.package_id(), retargeted.package_id());
        assert_eq!(base.graph().nodes.len(), retargeted.graph().nodes.len());
    }

    #[trace("FR-093-AC-19", "TC-416")]
    #[test]
    fn variants_are_deterministic_and_written_beside_the_cases() {
        let (_parent, output) = written();
        for variant in VARIANTS {
            let fixture = variant_fixture(variant).unwrap();
            assert_eq!(fixture.bytes, variant_fixture(variant).unwrap().bytes);
            assert_eq!(
                fs::read(output.join(variant.file_name())).unwrap(),
                fixture.bytes
            );
        }
    }

    #[trace("FR-093-AC-19", "TC-416")]
    #[test]
    fn an_existing_output_path_is_refused_and_left_untouched() {
        let parent = tempfile::tempdir().unwrap();
        let existing = parent.path().join("existing");
        fs::create_dir(&existing).unwrap();
        fs::write(existing.join("keep"), b"kept").unwrap();
        match write(&existing) {
            Err(Error::OutputExists { path }) => assert_eq!(path, existing),
            other => panic!("expected OutputExists, got {other:?}"),
        }
        assert_eq!(fs::read(existing.join("keep")).unwrap(), b"kept");
        assert_eq!(fs::read_dir(&existing).unwrap().count(), 1);

        let file = parent.path().join("file");
        fs::write(&file, b"kept").unwrap();
        assert!(matches!(write(&file), Err(Error::OutputExists { .. })));
    }

    #[trace("FR-093-AC-19", "TC-416")]
    #[test]
    fn an_output_path_under_a_missing_parent_is_a_typed_io_refusal() {
        let parent = tempfile::tempdir().unwrap();
        let output = parent.path().join("missing").join("population");
        match write(&output) {
            Err(Error::Io { path, source }) => {
                assert_eq!(path, output);
                assert_eq!(source.kind(), io::ErrorKind::NotFound);
            }
            other => panic!("expected Io, got {other:?}"),
        }
    }
}

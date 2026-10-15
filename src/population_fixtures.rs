// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-771 (FR-093-AC-19, TC-416 step 11): the `Population<T>[N]` Contract IR
//! v2 packages the admission corpus emits, as files.
//!
//! [`population_package`] builds one package over the TC-442 spine model
//! whose function `pop` takes a `Population<M::Gadget>[N]` parameter for each
//! of its bounds, and [`emit`] writes it through the real v2 emitter. The
//! admission corpus asserts those bytes are admitted by IR's checked-package
//! v2 reader; [`write`] writes the same bytes, for [`CASES`], into a new
//! directory so IR's conformance gate can read them without depending on QSL.
//! The `population_v2_fixtures` example is a thin caller of [`write`].
//!
//! Beside each case the directory holds the domain package document the
//! reader needs as evidence, under [`DOMAIN_PACKAGE_FILE`].

use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
};

use qsl_forms::{BuiltinType, Expression, FunctionDeclaration, TypeForm, TypeFormHead};
use qsl_foundation::{source::provenance::SourceRegion, Span};
use qsl_package::{CheckedPackage, Emission};
use qsl_semantics::check::PackageDeclarations;
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

/// Why [`write`] or [`emit`] produced no complete fixture set.
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
    /// The emitter omitted nodes, so the bytes are not the whole package.
    #[error("the v2 emitter omitted {omitted} nodes of the population fixture package")]
    Omitted {
        /// How many nodes the emission left out.
        omitted: usize,
    },
}

/// The file `bounds`' package is written to: `population-2-5.json` for
/// `[2, 5]`.
pub fn file_name(bounds: &[u64]) -> String {
    let bounds: Vec<String> = bounds.iter().map(u64::to_string).collect();
    format!("population-{}.json", bounds.join("-"))
}

/// The domain package document the cases' reader evidence is keyed by.
pub fn domain_package() -> &'static [u8] {
    DOMAIN_PACKAGE
}

/// A package over the TC-442 spine model whose one function `pop` takes a
/// `Population<M::Gadget>[N]` parameter for each of `bounds`, built from
/// source forms (source text spells no `Population` type).
pub fn population_package(bounds: &[u64]) -> Result<CheckedPackage, Error> {
    let span = Span { start: 0, end: 0 };
    let parameters = bounds
        .iter()
        .enumerate()
        .map(|(at, bound)| {
            let population = TypeForm::new(TypeFormHead::Population, span)
                .with_arguments(vec![TypeForm::name("M::Gadget", span)])
                .with_bounds(vec![bound.to_string()]);
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
    Ok(CheckedPackage::link(graph))
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

/// `bounds`' package through the v2 emitter, with nothing omitted.
pub fn emit(bounds: &[u64]) -> Result<Emission, Error> {
    let package = population_package(bounds)?;
    // The forms carry no spans, so every occurrence is placed at the whole
    // of the unit the package was checked from.
    let source = package.graph().source().clone();
    let length = u64::try_from(SPINE_MODEL_UNIT.len()).unwrap_or(u64::MAX);
    let emission = qsl_package::emit_package(&package, |_| {
        SourceRegion::new(source.clone(), 0, length).ok()
    })
    .map_err(|refusal| Error::Emit(format!("{refusal:?}")))?;
    match emission.omitted().len() {
        0 => Ok(emission),
        omitted => Err(Error::Omitted { omitted }),
    }
}

/// Writes the v2 package of every case of [`CASES`] and the domain package
/// document into `directory`, which must not exist. Every package is emitted
/// before the directory is created, so an emitter refusal leaves nothing
/// behind.
pub fn write(directory: &Path) -> Result<(), Error> {
    let mut files = vec![(DOMAIN_PACKAGE_FILE.to_owned(), DOMAIN_PACKAGE.to_vec())];
    for bounds in CASES {
        let emission = emit(bounds)?;
        files.push((file_name(bounds), emission.package().bytes().to_vec()));
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
    use qsl_package::{read_import_view, AdmittedPackages, V2ReadLimits};
    use qsl_semantics::library::LibraryName;

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
        expected.push(DOMAIN_PACKAGE_FILE.to_owned());
        expected.sort();
        assert_eq!(files, expected);
    }

    #[trace("FR-093-AC-19", "TC-416")]
    #[test]
    fn written_bytes_equal_the_emitted_package_and_are_admitted_by_ir() {
        let (_parent, output) = written();
        let domain_packages = qsl_semantics::model::intake::package_input([domain_package()]);
        for bounds in CASES {
            let emission = emit(bounds).unwrap();
            let on_disk = fs::read(output.join(file_name(bounds))).unwrap();
            assert_eq!(on_disk, emission.package().bytes(), "{bounds:?}");

            // `read_import_view` runs IR's checked-package v2 reader over the
            // emission's bytes and cross-checks the `package_id`.
            let package = population_package(bounds).unwrap();
            let view = read_import_view(
                &package,
                &emission,
                LibraryName::new("pkg").unwrap(),
                &domain_packages,
                &mut AdmittedPackages::default(),
                V2ReadLimits::default(),
            )
            .unwrap_or_else(|refusal| panic!("{bounds:?}: {refusal:?}"));
            assert_eq!(
                view.package(),
                emission.package().package_id(),
                "{bounds:?}"
            );
        }
        assert_eq!(
            fs::read(output.join(DOMAIN_PACKAGE_FILE)).unwrap(),
            domain_package()
        );
    }

    /// Each population type node is a `model_population` node carrying role
    /// `type` and only that role, with the declared bound as its `max`.
    #[trace("FR-093-AC-19", "TC-416")]
    #[test]
    fn each_bound_is_a_typed_population_node() {
        for bounds in CASES {
            let wire: serde_json::Value =
                serde_json::from_slice(emit(bounds).unwrap().package().bytes()).unwrap();
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

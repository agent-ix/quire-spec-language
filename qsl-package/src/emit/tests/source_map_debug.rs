// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-1358 (QSL-381): a source map read from indexed wire IDs formats for
//! debug in time and output linear in the graph. Independent wire IDs share
//! their resolved operand keys through the authoritative document graph.

use std::time::{Duration, Instant};

use qsl_foundation::source::provenance::{OccurrenceKey, PackageSourceMap};
use quire_exact::Origin;

use super::*;
use qsl_semantics::check::wire_id::WireGraph;
use quire_contract_model::CheckedNodePathSegment;

/// The key of the sum `x + x + ...` of `depth` additions: each key nests the
/// previous, every nested node one shared `Arc`.
fn chain(depth: u64) -> Vec<NodeKey> {
    let mut keys = vec![NodeKey::new("pkg", vec![PathSegment::Text("x".into())], 0).unwrap()];
    for level in 0..depth {
        let next = NodeKey::new(
            "pkg",
            vec![
                PathSegment::Text("add".into()),
                PathSegment::Node(keys[keys.len() - 1].clone()),
            ],
            level,
        )
        .unwrap();
        keys.push(next);
    }
    keys
}

fn map_of(keys: impl IntoIterator<Item = NodeKey>) -> PackageSourceMap {
    let source = qsl_foundation::source::provenance::RawSourceRef::new("a", "unit").unwrap();
    PackageSourceMap::from_entries(keys.into_iter().enumerate().map(|(at, key)| {
        let region = SourceRegion::new(source.clone(), at as u64, at as u64 + 1).unwrap();
        (
            OccurrenceKey::new(key, Origin::new(Role::new("expression"), 0)),
            vec![region],
        )
    }))
    .unwrap()
}

/// The fastest of three runs of `map`'s debug formatting.
fn debug_time(map: &impl std::fmt::Debug) -> (Duration, usize) {
    let mut best = Duration::MAX;
    let mut bytes = 0;
    for _ in 0..3 {
        let start = Instant::now();
        bytes = format!("{map:?}").len();
        best = best.min(start.elapsed());
    }
    (best, bytes)
}

/// Independently allocated flat IDs resolve through one graph, keeping the
/// same output and formatting time as the original shared chain.
/// Provenance: QSL-1358, QSL-381; component scaling regression.
#[test]
fn a_read_source_map_formats_in_linear_time_though_the_wire_ids_share_nothing() {
    std::thread::Builder::new()
        .stack_size(512 * 1024)
        .spawn(|| {
            for depth in [500, 1_000, 2_000] {
                let shared = chain(depth);
                let wire: Vec<CheckedNodeId> = shared
                    .iter()
                    .enumerate()
                    .map(|(at, key)| CheckedNodeId {
                        package: key.package().into(),
                        path: key
                            .path()
                            .into_iter()
                            .map(|segment| match segment {
                                PathSegment::Text(text) => {
                                    CheckedNodePathSegment::Name(text.into())
                                }
                                PathSegment::Node(_) => {
                                    CheckedNodePathSegment::Index((at - 1) as u64)
                                }
                            })
                            .collect(),
                        ordinal: key.ordinal(),
                    })
                    .collect();
                let graph = WireGraph::new(&wire).unwrap();
                let read: Vec<_> = wire.iter().map(|id| graph.key(id).unwrap()).collect();
                assert_eq!(read, shared, "the read keys are the keys the ids name");
                let (shared_time, shared_bytes) = debug_time(&map_of(shared));
                let (read_time, read_bytes) = debug_time(&map_of(read));
                eprintln!("depth {depth}: shared {shared_time:?}, graph-read {read_time:?}, output {read_bytes} bytes");
                assert_eq!(
                    read_bytes, shared_bytes,
                    "depth {depth}: same complete output"
                );
                assert!(
                    read_time < shared_time * 20 + Duration::from_millis(20),
                    "depth {depth}: graph-read keys {read_time:?} against shared {shared_time:?}"
                );
            }
        })
        .unwrap()
        .join()
        .unwrap();
}

/// Counts complete debug output without retaining its bytes.
struct ByteCount(usize);

impl std::fmt::Write for ByteCount {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        self.0 += text.len();
        Ok(())
    }
}

/// The checked package's debug output (its graph's occurrence map,
/// requirement map, semantic graph and every other node-keyed field) grows
/// linearly with the depth of a sum, not with its square.
#[test]
fn the_checked_package_debug_output_grows_linearly_with_depth() {
    use qsl_semantics::check::depth_forms::{declarations, Form};
    use std::fmt::Write;
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(|| {
            let sizes: Vec<usize> = [100, 200, 400]
                .into_iter()
                .map(|levels| {
                    let graph = declarations(Form::Add, levels)
                        .check(CheckingLimits::default())
                        .unwrap_or_else(|refusals| panic!("checks: {refusals:?}"));
                    let package = CheckedPackage::link(graph);
                    let mut count = ByteCount(0);
                    write!(&mut count, "{package:?}").unwrap();
                    count.0
                })
                .collect();
            for pair in sizes.windows(2) {
                assert!(
                    pair[1] < pair[0] * 5 / 2,
                    "each doubling of the depth at most doubles the output: {sizes:?}"
                );
            }
        })
        .unwrap()
        .join()
        .unwrap();
}

/// A real source-backed package, including spans for its later emission.
fn source_package_for_debug(body: &str) -> CheckedPackage {
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         function f using v(a: Boolean, x: Integer): Integer pure {{ {body} }}\n"
    );
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        source.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("the source parses");
    assert!(parsed.is_admissible(), "the source is admissible");
    let forms = qsl_forms::build_unit(&parsed).unwrap();
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "pkg".into(),
        forms,
        Vec::new(),
        Vec::new(),
    )
    .unwrap();
    let graph = declarations
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("checks: {refusals:?}"));
    CheckedPackage::link(graph)
}

/// Indexed I2 output, including the retained IR package, stays linear before
/// the full FR-264 acceptance case is attempted.
/// Provenance: QSL-1358, QSL-381; component scaling regression.
#[test]
fn the_indexed_v2_read_debug_output_grows_linearly_with_depth() {
    use std::fmt::Write;
    std::thread::Builder::new()
        .stack_size(512 * 1024)
        .spawn(|| {
            let sizes: Vec<(usize, usize, usize)> = [100, 200, 400]
                .into_iter()
                .map(|levels| {
                    let package = source_package_for_debug(&format!("x{}", " + x".repeat(levels)));
                    let mut before = ByteCount(0);
                    write!(&mut before, "{package:?}").unwrap();
                    let emission = emit_checked(&package).unwrap();
                    let mut after = ByteCount(0);
                    let started = Instant::now();
                    write!(&mut after, "{package:?}").unwrap();
                    let package_time = started.elapsed();
                    assert!(
                        after.0 > before.0,
                        "the retained emission contributes to package Debug"
                    );
                    let read = crate::checked_v2::read_checked_package_v2(
                        emission.package.bytes(),
                        library(),
                        V2ReadLimits::default(),
                        &read_evidence(&emission),
                    )
                    .unwrap_or_else(|failure| panic!("admitted: {failure:?}"))
                    .into_value();
                    let mut count = ByteCount(0);
                    let started = Instant::now();
                    write!(&mut count, "{read:?}").unwrap();
                    eprintln!(
                        "depth {levels}: emitted-package Debug {package_time:?}, I2 Debug {:?}",
                        started.elapsed()
                    );
                    (before.0, after.0, count.0)
                })
                .collect();
            eprintln!("Debug bytes at depths 100/200/400 (before emit, after emit, I2): {sizes:?}");
            for pair in sizes.windows(2) {
                assert!(
                    pair[1].0 < pair[0].0 * 5 / 2,
                    "pre-emission Debug sizes: {sizes:?}"
                );
                assert!(
                    pair[1].1 < pair[0].1 * 5 / 2,
                    "retained-emission Debug sizes: {sizes:?}"
                );
                assert!(
                    pair[1].2 < pair[0].2 * 5 / 2,
                    "indexed I2 Debug sizes: {sizes:?}"
                );
            }
        })
        .unwrap()
        .join()
        .unwrap();
}

/// Canonical NodeKind children remain arena IDs when the checked body is
/// formatted on its own, outside the package's node-table scope.
/// Provenance: QSL-1358, QSL-381; canonical vocabulary integration regression.
#[test]
fn canonical_standalone_checked_body_debug_grows_linearly_with_depth() {
    std::thread::Builder::new()
        .stack_size(512 * 1024)
        .spawn(|| {
            for label in ["sum", "else if", "let"] {
                let mut sizes = Vec::new();
                for depth in [100, 200, 400] {
                    let expression = match label {
                        "sum" => format!("x{}", " + x".repeat(depth)),
                        "else if" => format!("{}x", "if a then x else ".repeat(depth)),
                        _ => format!(
                            "{}x",
                            (0..depth)
                                .map(|at| format!("let b{at} = x in "))
                                .collect::<String>()
                        ),
                    };
                    let package = source_package_for_debug(&expression);
                    let body = package.graph().function_state(0).unwrap().body.body();
                    let clone = body.clone();
                    assert_eq!(&clone, body, "{label}: the body equals its clone");
                    let started = Instant::now();
                    let text = format!("{clone:?}");
                    eprintln!(
                        "standalone {label} depth {depth}: {} bytes, {:?}",
                        text.len(),
                        started.elapsed()
                    );
                    assert!(text.starts_with("CheckedBody { nodes: ["));
                    assert!(text.ends_with("] }"));
                    assert_eq!(
                        text.matches("Node { kind: ").count(),
                        body.len(),
                        "{label}: every arena node is complete"
                    );
                    sizes.push(text.len());
                }
                for pair in sizes.windows(2) {
                    assert!(
                        pair[1] < pair[0] * 5 / 2,
                        "{label}: standalone body sizes {sizes:?}"
                    );
                }
            }
        })
        .unwrap()
        .join()
        .unwrap();
}

/// Repeated field reads print the complete canonical declaration identity
/// once per arena field node; the declared key itself has no expression tree.
/// Provenance: QSL-1358, QSL-381; canonical RecordField integration regression.
#[test]
fn canonical_record_field_debug_keeps_complete_keys_as_the_body_grows() {
    use qsl_semantics::check::depth_forms::{name_expr, types};
    let mut sizes = Vec::new();
    for depth in [100, 200, 400] {
        let field = || Expression::field(name_expr("n"), "v".into());
        let mut expression = field();
        for _ in 0..depth {
            expression = Expression::binary(BinaryOperator::And, field(), expression);
        }
        let span = qsl_foundation::Span { start: 0, end: 0 };
        let graph = PackageDeclarations {
            types: types(),
            functions: vec![FunctionDeclaration::new(
                "f",
                vec![("n".into(), TypeForm::name("N", span))],
                TypeForm::builtin(BuiltinType::Boolean, span),
                None,
                expression,
            )],
            ..PackageDeclarations::new(
                qsl_semantics::check::fixture_source(),
                "fixture",
                qsl_foundation::IdentityLimits::default(),
            )
        }
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("checks: {refusals:?}"));
        let body = graph.function_state(0).unwrap().body.body();
        let started = Instant::now();
        let text = format!("{body:?}");
        eprintln!(
            "canonical field depth {depth}: {} bytes, {:?}",
            text.len(),
            started.elapsed()
        );
        assert_eq!(text.matches("Node { kind: ").count(), body.len());
        assert_eq!(
            text.matches("RecordField { declaration: NodeKey(fixture:[\"N\",]:0), name: \"v\" }")
                .count(),
            depth + 1,
            "each field retains its complete declared identity"
        );
        sizes.push(text.len());
    }
    for pair in sizes.windows(2) {
        assert!(
            pair[1] < pair[0] * 5 / 2,
            "canonical field body sizes {sizes:?}"
        );
    }
}

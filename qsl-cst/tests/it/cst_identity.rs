// SPDX-License-Identifier: AGPL-3.0-or-later
//! CST identity: a revision digest, a span and a structural path derived on
//! request, with reuse decided on request by typed comparison. Parsing does
//! no per-node hashing and copies no per-node path.
use ix_trace_rs::trace;
use qsl_cst::Limits;
use qsl_cst::{CstElement, CstNode, LosslessCst, NodeIdentity, Production};
use qsl_foundation::SourceIdentity;

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\nprofile Complete = \"quire.value.complete/v1\" version \"1\" digest \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n";

fn parse(revision: &str, text: &str) -> qsl_cst::ParsedSource {
    let parsed = qsl_cst::parse(
        SourceIdentity {
            identity: "test:cst-identity".into(),
            revision: revision.into(),
        },
        "identity.native",
        text.as_bytes(),
        Limits::default(),
    )
    .expect("within the default ceilings");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    parsed
}

fn functions(count: usize) -> String {
    let mut text = HEADER.to_string();
    for index in 0..count {
        text.push_str(&format!(
            "function f{index} using Complete (x: Integer): Integer pure {{ if x > 0 then x + {index} else 0 }}\n"
        ));
    }
    text
}

#[trace("TC-222", "FR-302-AC-3")]
#[test]
fn a_parse_hashes_no_per_node_data() {
    let small = parse("r1", &functions(1));
    let large = parse("r1", &functions(200));
    assert!(large.cst().nodes().len() > 100 * small.cst().nodes().len());
    // Only the revision labels and the source digest are hashed, so the
    // count is the same however many nodes the parse built.
    assert_eq!(
        small.cst().identity_hashed_bytes(),
        large.cst().identity_hashed_bytes()
    );
}

// Identities are fixed-size values: nothing per node holds a copied path or
// ancestor list. A type that owned a `Vec` could not be `Copy`.
#[trace("TC-222", "FR-302-AC-1")]
#[test]
fn node_identity_is_a_fixed_size_value() {
    fn copy<T: Copy>() {}
    copy::<NodeIdentity>();
    assert!(std::mem::size_of::<NodeIdentity>() <= 64);
}

/// The structural path of every node, computed independently by walking
/// children down from the root.
fn paths_from_the_root(cst: &LosslessCst) -> Vec<Option<Vec<u32>>> {
    let mut paths = vec![None; cst.nodes().len()];
    let root = cst.root().identity().node;
    let mut pending = vec![(root, Vec::new())];
    while let Some((index, path)) = pending.pop() {
        let children = cst.nodes()[index]
            .children()
            .iter()
            .filter_map(|child| match child {
                CstElement::Node(node) => Some(*node),
                CstElement::Token(_) => None,
            });
        for (ordinal, child) in children.enumerate() {
            let mut child_path: Vec<u32> = path.clone();
            child_path.push(u32::try_from(ordinal).unwrap());
            pending.push((child, child_path));
        }
        paths[index] = Some(path);
    }
    paths
}

#[trace("TC-222", "FR-302-AC-3")]
#[test]
fn structural_paths_are_derived_from_parent_links_on_request() {
    let parsed = parse("r1", &functions(3));
    let cst = parsed.cst();
    let expected = paths_from_the_root(cst);
    for node in cst.nodes() {
        assert_eq!(
            Some(cst.structural_path(node)),
            expected[node.identity().node]
        );
        assert_eq!(
            cst.ancestor_productions(node).count(),
            cst.structural_path(node).len()
        );
    }
}

/// The FR-302 reuse rule restated over production *positions* in the
/// closed inventory instead of production values: the same decision under
/// a relabelling that shares nothing with the variant names.
fn relabelled_reuse(
    before: &LosslessCst,
    node: &CstNode,
    after: &LosslessCst,
    candidate: &CstNode,
) -> bool {
    let label = |production: Production| {
        Production::all()
            .iter()
            .position(|known| *known == production)
            .expect("closed inventory")
    };
    let bytes = |cst: &LosslessCst, node: &CstNode| {
        cst.source().text().as_bytes()[node.span().start..node.span().end].to_vec()
    };
    let labels = |cst: &LosslessCst, node: &CstNode| {
        std::iter::once(label(node.production()))
            .chain(cst.ancestor_productions(node).map(label))
            .collect::<Vec<_>>()
    };
    bytes(before, node) == bytes(after, candidate)
        && labels(before, node) == labels(after, candidate)
}

// Reuse compares `Production` values and byte slices. Renaming a variant
// changes neither, so every decision equals the one taken over name-free
// labels.
#[trace("TC-222", "FR-302-AC-3")]
#[test]
fn reuse_decisions_do_not_depend_on_production_names() {
    let before = parse("r1", &functions(2));
    let after = parse("r2", &functions(2).replace("x + 1", "x + 7"));
    let (before, after) = (before.cst(), after.cst());
    let mut reused = 0_usize;
    let mut refused = 0_usize;
    for node in before.nodes() {
        for candidate in after.nodes() {
            let decision = before.may_reuse(node, after, candidate);
            assert_eq!(decision, relabelled_reuse(before, node, after, candidate));
            if decision {
                reused += 1;
            } else {
                refused += 1;
            }
        }
    }
    assert!(reused > 0 && refused > 0);
}

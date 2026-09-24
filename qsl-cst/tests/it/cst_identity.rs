// SPDX-License-Identifier: AGPL-3.0-or-later
//! CST identity: a revision digest, a span and a structural path derived on
//! request, with reuse decided by a one-to-one span mapping over typed
//! productions. The digest count and the name-independence of reuse are
//! unit tests beside `LosslessCst` in `src/cst.rs`.
use ix_trace_rs::trace;
use qsl_cst::{CstElement, CstNode, Limits, LosslessCst, NodeIdentity, Production, SourceChange};
use qsl_foundation::{SourceIdentity, Span};

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\nprofile Complete = \"quire.value.complete/v1\" version \"1\" digest \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n";

fn parse(revision: &str, text: &str) -> qsl_cst::ParsedSource {
    let parsed = qsl_cst::parse(
        SourceIdentity {
            authority: "test".into(),
            identity: "test:cst-identity".into(),
            revision_namespace: "test".into(),
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
    let root = cst.root().identity().node.get();
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
            expected[node.identity().node.get()]
        );
        assert_eq!(
            cst.ancestor_productions(node).count(),
            cst.structural_path(node).len()
        );
    }
}

/// The `Primary` nodes of the unit, in source order.
fn primaries(cst: &LosslessCst) -> Vec<&CstNode> {
    let mut primaries: Vec<_> = cst
        .nodes()
        .iter()
        .filter(|node| node.production() == Production::Primary)
        .collect();
    primaries.sort_by_key(|node| node.span().start);
    primaries
}

// In `x + x` the two operands have equal bytes, productions and ancestor
// paths. Reuse keeps each on its own counterpart, never both on one, for an
// edit before, between and after them.
#[trace("TC-222", "FR-302-AC-3")]
#[test]
fn reuse_is_one_to_one_between_equal_operands() {
    let text =
        format!("{HEADER}function f using Complete (x: Integer): Integer pure {{ x + x }}\n");
    let first = text.find("x + x").expect("body");
    let second = first + "x + ".len();
    let before = parse("r1", &text);
    for (at, removed, replacement) in [
        (first, 0, "  "),
        (first + "x ".len(), 1, "-"),
        (second + 1, 0, " "),
    ] {
        let edited = format!("{}{replacement}{}", &text[..at], &text[at + removed..]);
        let after = parse("r2", &edited);
        let change = SourceChange {
            range: Span {
                start: at,
                end: at + removed,
            },
            inserted: replacement.len(),
        };
        let map = before
            .cst()
            .reuse_map(after.cst(), change)
            .expect("the edited source is this edit applied");
        let mut targets: Vec<_> = map.iter().flatten().collect();
        let reused = targets.len();
        targets.sort_unstable();
        targets.dedup();
        assert_eq!(targets.len(), reused, "no two nodes share one successor");

        let old = primaries(before.cst());
        let new = primaries(after.cst());
        assert_eq!((old.len(), new.len()), (2, 2));
        for (old, new) in old.iter().zip(&new) {
            assert_eq!(map[old.identity().node.get()], Some(new.identity().node));
        }
    }
    // A successor that is not the stated edit maps nothing.
    let unrelated = parse("r3", &text.replace("x + x", "x * x"));
    let change = SourceChange {
        range: Span {
            start: first,
            end: first,
        },
        inserted: 0,
    };
    assert!(before.cst().reuse_map(unrelated.cst(), change).is_none());
}

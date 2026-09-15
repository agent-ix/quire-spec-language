// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-036: bounded lookup of a declaration's contiguous parser-arena region.

use super::binding_work::{Dimension, Exhaustion, Work};
use crate::Span;

/// Parser arenas are postorder within each source-ordered declaration (the same
/// invariant used by dependencies::declaration_at). Nodes inside a declaration
/// need not be source-ordered, but every node of an earlier declaration precedes
/// every node of a later one. A declaration boundary therefore partitions the
/// arena. Binary lookup avoids rescanning other declarations and needs no copied
/// AST or separately allocated ownership index. Charge every inspected node.
/// The parser-behavior test below checks containment and contiguous ownership
/// across all three arenas, including internally non-source-ordered nodes.
pub(super) fn owned<'a, T>(
    nodes: &'a [T],
    owner: Span,
    span: impl Fn(&T) -> Span,
    work: &mut Work,
) -> Result<&'a [T], Exhaustion> {
    let range = crate::syntax::composed::arena::owned_range(nodes, owner, span, || {
        work.charge(Dimension::References, 1)
    })?;
    Ok(&nodes[range])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linking::composed::binding_work::Limits as BindingLimits;
    use crate::syntax::composed::NativeUnit;
    use crate::{parse_native, Limits, SourceIdentity};
    use ix_trace_rs::trace;

    fn assert_regions<T>(nodes: &[T], owners: &[Span], span: impl Fn(&T) -> Span) {
        for node in nodes {
            let node = span(node);
            assert_eq!(
                owners
                    .iter()
                    .filter(|owner| owner.start <= node.start && node.end <= owner.end)
                    .count(),
                1,
                "every parser node must belong to exactly one declaration"
            );
        }
        let mut next = 0;
        for &owner in owners {
            // Independent full traversal is a test oracle, never a production scan.
            let expected: Vec<_> = nodes
                .iter()
                .enumerate()
                .filter(|(_, node)| {
                    let node = span(node);
                    owner.start <= node.start && node.end <= owner.end
                })
                .map(|(index, _)| index)
                .collect();
            assert_eq!(expected, (next..next + expected.len()).collect::<Vec<_>>());
            let actual = owned(
                nodes,
                owner,
                &span,
                &mut Work::new(BindingLimits::default()),
            )
            .unwrap();
            assert_eq!(actual.len(), expected.len());
            for (node, index) in actual.iter().zip(&expected) {
                assert!(std::ptr::eq(node, &nodes[*index]));
            }
            next += expected.len();
        }
        assert_eq!(next, nodes.len());
    }

    #[test]
    #[trace("TC-114", "FR-036-AC-1", "FR-036-AC-7")]
    fn real_parser_arenas_partition_by_declaration_despite_postorder_nodes() {
        // Unresolved selections are syntax inputs; this tests parser ownership.
        let text = r#"language "ix:native" edition "1-draft";
            profile S = "syntax-only" version "1" digest "unresolved";
            profile T = "syntax-only" version "1" digest "unresolved";
            profile P = "syntax-only" version "1" digest "unresolved";
            predicate First using S (): Boolean { true and false }
            temporal Soon using T over (view: M::Node) clock "clock" on origin {
                eventually[0,1] holds(true and false)
            }
            predicate Gap using S (): Boolean { true }
            protocol Flow using P over (view: M::Node) on origin {
                role Service on M::Node;
                run sequence Main {
                    check One using S { true };
                    check Two using S { false and true };
                }
                finish Closed as (closed: M::Node) { true };
            }
            temporal Last using T over (view: M::Node) clock "clock" on origin {
                always[0,1] holds(true)
            }
        "#;
        let NativeUnit::Composed(unit) = parse_native(
            SourceIdentity {
                identity: "arena-order".into(),
                revision: "1".into(),
            },
            "arena.native",
            text.as_bytes(),
            Limits::default(),
        )
        .unwrap() else {
            panic!("composed fixture")
        };
        let owners: Vec<_> = unit.declarations().iter().map(|decl| decl.span).collect();
        assert_eq!(owners.len(), 5);
        assert!(owners.windows(2).all(|pair| pair[0].end <= pair[1].start));
        assert_eq!(unit.expressions().len(), 13);
        assert_eq!(unit.temporal_nodes().len(), 4);
        assert_eq!(unit.controls().len(), 3);
        assert!(unit
            .expressions()
            .windows(2)
            .any(|pair| pair[0].span.start > pair[1].span.start));
        assert_regions(unit.expressions(), &owners, |node| node.span);
        assert_regions(unit.temporal_nodes(), &owners, |node| node.span);
        assert_regions(unit.controls(), &owners, |node| node.span);
    }

    #[test]
    #[trace("TC-114", "FR-036-AC-7")]
    fn empty_regions_and_exact_boundary_cost_refuse_before_an_unpaid_read() {
        let zero = BindingLimits {
            references: 0,
            ..BindingLimits::default()
        };
        let empty: &[Span] = &[];
        assert!(owned(
            empty,
            Span { start: 0, end: 1 },
            |span| *span,
            &mut Work::new(zero)
        )
        .unwrap()
        .is_empty());
        let nodes = [
            Span { start: 0, end: 2 },
            Span { start: 4, end: 6 },
            Span { start: 8, end: 10 },
        ];
        let owner = Span { start: 3, end: 7 };
        // Two comparisons for each boundary in this three-node arena.
        let exact = BindingLimits {
            references: 4,
            ..zero
        };
        let mut work = Work::new(exact);
        assert_eq!(
            owned(&nodes, owner, |span| *span, &mut work).unwrap(),
            &nodes[1..2]
        );
        assert_eq!(work.usage().references, 4);
        let error = owned(
            &nodes,
            owner,
            |span| *span,
            &mut Work::new(BindingLimits {
                references: 3,
                ..exact
            }),
        )
        .unwrap_err();
        assert_eq!(error.dimension, Dimension::References);
        assert_eq!((error.used, error.requested, error.limit), (3, 1, 3));
        for owner in [Span { start: 2, end: 4 }, Span { start: 10, end: 12 }] {
            assert!(owned(&nodes, owner, |span| *span, &mut Work::new(exact))
                .unwrap()
                .is_empty());
        }
    }
}

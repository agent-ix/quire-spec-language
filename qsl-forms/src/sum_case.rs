// SPDX-License-Identifier: AGPL-3.0-or-later
//! SumCase's S2 union declaration reading (FR-313). Expression
//! bodies are mapped by the existing iterative arena builder in `value`.

use qsl_cst::Production;

use crate::dispatch::{Construct, FormsRefusal};
use crate::syntax::{DeclarationForm, UnionForm, UnionMemberForm};
use crate::value::{declared_name, items, nodes_of, production_node, type_form};

pub(crate) fn union_declaration(construct: Construct<'_>) -> Result<DeclarationForm, FormsRefusal> {
    let cst = construct.cst;
    let node = production_node(construct)?;
    let declaration_items = items(cst, node);
    let mut members = Vec::new();
    for member in nodes_of(&declaration_items, Production::UnionMember) {
        let member_items = items(cst, member);
        let payload = nodes_of(&member_items, Production::TypeReference)
            .into_iter()
            .map(|reference| type_form(cst, reference))
            .collect::<Result<_, _>>()?;
        members.push(UnionMemberForm {
            name: declared_name(&member_items, member)?,
            payload,
            span: member.span(),
        });
    }
    Ok(DeclarationForm::Union(UnionForm {
        name: declared_name(&declaration_items, node)?,
        members,
        span: node.span(),
    }))
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use qsl_cst::Production;
    use qsl_foundation::SourceIdentity;

    use crate::{build_unit, DeclarationForm};

    #[trace("TC-815", "FR-313-AC-1", "FR-313-AC-2", "FR-257-AC-1")]
    #[test]
    fn case_and_payload_stacks_fit_the_cst_node_charge_including_wide_binders() {
        let depth = 1_000;
        let binders = (0..4_096)
            .map(|index| format!("b{index}"))
            .collect::<Vec<_>>()
            .join(",");
        let source = format!("language \"ix:native\" edition \"1-draft\"; profile v = \"quire.value.complete/v1\"; union U {{ A({}Integer{}) }} function f using v(s: U): Integer pure {{ case s {{ A({binders}): {}s{}; }} }}", "Option<".repeat(depth), ">".repeat(depth), "case s { A(x): ".repeat(depth), "; }".repeat(depth));
        let parsed = qsl_cst::parse(
            SourceIdentity::new("test", "stacks", "git", "1"),
            "case.native",
            source.as_bytes(),
            qsl_cst::Limits::default(),
        )
        .expect("S1 reads the unit");
        assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
        let nodes = parsed.cst().nodes().len();
        assert_eq!(
            parsed
                .cst()
                .nodes()
                .iter()
                .filter(|node| node.production() == Production::CaseBinder)
                .count(),
            depth + 4_096
        );
        let unit = build_unit(&parsed).expect("S2 builds the arena");
        let DeclarationForm::Function(function) = unit.forms()[1].form() else {
            panic!("function")
        };
        assert!(!format!("{:?}", function.body).is_empty());
        for stack in ["expression", "expression_debug", "type_form"] {
            let peak = crate::syntax::stack_peak::take(stack);
            assert!(peak > 0, "{stack} was used");
            assert!(
                peak <= nodes,
                "{stack}: {peak} entries for {nodes} charged CST nodes"
            );
        }
    }
}

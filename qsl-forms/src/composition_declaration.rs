// SPDX-License-Identifier: AGPL-3.0-or-later
//! The component composition's S2 form (QSL FR-530, QSpec FR-491): a
//! `compose S rule R { component ...; members }` declaration.
//!
//! This module reads the CST of one `CompositionDeclaration` and builds its
//! [`CompositionDeclarationForm`]. It selects no meaning, the same contract the
//! implementation guarantee's form keeps: every name stays spelled and
//! spanned. QSpec FR-491 owns the static checks; S3 applies them.

use qsl_cst::{CstNode, LosslessCst, Production, TokenKind};
use qsl_foundation::Span;

use super::dispatch::{Construct, FormsRefusal};
use super::guarantee_declaration::GuaranteeEnvironmentForm;
use super::syntax::{DeclarationForm, DeclaredName, FairnessConstraintForm};
use super::temporal_clause::fairness_form;
use super::value::{
    items, nodes_of, only, production_node, text, tokens_of, unexpected, Item,
};

/// A composition rule spelling (QSpec FR-491 "Declaration form").
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CompositionRuleForm {
    /// `assume-guarantee` (FR-176).
    AssumeGuarantee,
    /// `protocol-refinement` (FR-177).
    ProtocolRefinement,
    /// `refinement-transfer` (FR-380).
    RefinementTransfer,
    /// `interface` (FR-348).
    Interface,
}

/// A `component C from I using M;` member.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompositionComponentForm {
    /// The component name `C`.
    pub name: DeclaredName,
    /// The import alias `I`, as written.
    pub import: DeclaredName,
    /// The model alias `M`, as written.
    pub model: DeclaredName,
    /// The span of the whole member, `;` included.
    pub span: Span,
}

/// A `C::G` reference to a clause of a component's package, as written.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentClauseForm {
    /// The component name `C`.
    pub component: DeclaredName,
    /// The clause name `G`.
    pub clause: DeclaredName,
}

/// A `guarantee C::G;` member.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompositionGuaranteeForm {
    /// The guarantee.
    pub guarantee: ComponentClauseForm,
    /// The span of the whole member, `;` included.
    pub span: Span,
}

/// A `rely C::G on D::H;` or `depends C::G on D::H;` member.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompositionPremiseForm {
    /// The guarantee `C::G` that carries the premise.
    pub guarantee: ComponentClauseForm,
    /// The peer guarantee `D::H` the premise is on.
    pub on: ComponentClauseForm,
    /// The span of the whole member, `;` included.
    pub span: Span,
}

/// A `supplies fair [kind] [granularity] D::T::o;` member.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompositionSuppliesForm {
    /// The fairness constraint.
    pub constraint: FairnessConstraintForm,
    /// The span of the whole member, `;` included.
    pub span: Span,
}

/// One member after the components, in source order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompositionMemberForm {
    /// `guarantee C::G;`.
    Guarantee(CompositionGuaranteeForm),
    /// `rely C::G on D::H;`.
    Rely(CompositionPremiseForm),
    /// `depends C::G on D::H;`.
    Depends(CompositionPremiseForm),
    /// `supplies fair ...;`.
    Supplies(CompositionSuppliesForm),
    /// `environment E by "s" ...;`.
    Environment(GuaranteeEnvironmentForm),
}

/// A `compose S rule R { ... }` declaration (QSpec FR-491).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompositionDeclarationForm {
    /// The declared composition name `S`.
    pub name: DeclaredName,
    /// The selected rule.
    pub rule: CompositionRuleForm,
    /// The span of the rule spelling.
    pub rule_span: Span,
    /// The `component` members in source order.
    pub components: Vec<CompositionComponentForm>,
    /// The other members in source order.
    pub members: Vec<CompositionMemberForm>,
    /// The span of the whole declaration, closing brace included.
    pub span: Span,
}

/// `compose S rule R { ... }` (QSpec FR-491).
pub(crate) fn declaration(construct: Construct<'_>) -> Result<DeclarationForm, FormsRefusal> {
    let cst = construct.cst;
    let node = production_node(construct)?;
    let declaration_items = items(cst, node);
    // `compose S rule R`: the contextual leading word, the name, the
    // contextual `rule` word, then the rule's spelling. A hyphenated spelling
    // is one token whatever its kind, so the first four tokens are read.
    let leading: Vec<&qsl_cst::CstToken> = declaration_items
        .iter()
        .filter_map(|item| match item {
            Item::Token(token) => Some(*token),
            Item::Node(_) => None,
        })
        .take(4)
        .collect();
    let [_, name, _, rule] = leading[..] else {
        return Err(unexpected(node));
    };
    let rule_spelling = text(rule, node)?;
    let mut components = Vec::new();
    let mut members = Vec::new();
    for item in declaration_items.iter().copied() {
        let Item::Node(child) = item else {
            continue;
        };
        match child.production() {
            Production::CompositionComponent => components.push(component(cst, child)?),
            Production::CompositionGuarantee => {
                members.push(CompositionMemberForm::Guarantee(CompositionGuaranteeForm {
                    guarantee: clause_ref(cst, child, 0)?,
                    span: child.span(),
                }));
            }
            Production::CompositionRely => {
                members.push(CompositionMemberForm::Rely(premise(cst, child)?));
            }
            Production::CompositionDepends => {
                members.push(CompositionMemberForm::Depends(premise(cst, child)?));
            }
            Production::CompositionSupplies => {
                let member_items = items(cst, child);
                members.push(CompositionMemberForm::Supplies(CompositionSuppliesForm {
                    constraint: fairness_form(
                        cst,
                        only(&member_items, Production::Fairness, child)?,
                    )?,
                    span: child.span(),
                }));
            }
            Production::GuaranteeEnvironment => {
                members.push(CompositionMemberForm::Environment(
                    super::guarantee_declaration::environment(cst, child)?,
                ));
            }
            _ => return Err(unexpected(child)),
        }
    }
    Ok(DeclarationForm::Composition(Box::new(
        CompositionDeclarationForm {
            name: declared(name, node)?,
            rule: rule_of(&rule_spelling).ok_or_else(|| unexpected(node))?,
            rule_span: rule.span(),
            components,
            members,
            span: node.span(),
        },
    )))
}

#[ix_trace_rs::string_edge]
fn rule_of(spelling: &str) -> Option<CompositionRuleForm> {
    match spelling {
        "assume-guarantee" => Some(CompositionRuleForm::AssumeGuarantee),
        "protocol-refinement" => Some(CompositionRuleForm::ProtocolRefinement),
        "refinement-transfer" => Some(CompositionRuleForm::RefinementTransfer),
        "interface" => Some(CompositionRuleForm::Interface),
        _ => None,
    }
}

fn declared(token: &qsl_cst::CstToken, node: &CstNode) -> Result<DeclaredName, FormsRefusal> {
    Ok(DeclaredName {
        name: text(token, node)?,
        span: token.span(),
    })
}

fn component(
    cst: &LosslessCst,
    node: &CstNode,
) -> Result<CompositionComponentForm, FormsRefusal> {
    let member_items = items(cst, node);
    // The contextual `component` word, then `C`, `I` and `M`.
    let [_, name, import, model] = tokens_of(&member_items, TokenKind::Identifier)[..] else {
        return Err(unexpected(node));
    };
    Ok(CompositionComponentForm {
        name: declared(name, node)?,
        import: declared(import, node)?,
        model: declared(model, node)?,
        span: node.span(),
    })
}

/// The `index`th `C::G` name of `node`.
fn clause_ref(
    cst: &LosslessCst,
    node: &CstNode,
    index: usize,
) -> Result<ComponentClauseForm, FormsRefusal> {
    let member_items = items(cst, node);
    let names = nodes_of(&member_items, Production::ModelName);
    let name = names.get(index).copied().ok_or_else(|| unexpected(node))?;
    // `C :: G`: the component, the separator and the clause, each its own
    // token whatever trivia lies between them.
    let [component, _, clause] = super::value::significant_tokens(cst, name)[..] else {
        return Err(unexpected(node));
    };
    Ok(ComponentClauseForm {
        component: declared(component, node)?,
        clause: declared(clause, node)?,
    })
}

fn premise(cst: &LosslessCst, node: &CstNode) -> Result<CompositionPremiseForm, FormsRefusal> {
    Ok(CompositionPremiseForm {
        guarantee: clause_ref(cst, node, 0)?,
        on: clause_ref(cst, node, 1)?,
        span: node.span(),
    })
}

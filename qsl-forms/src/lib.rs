// SPDX-License-Identifier: AGPL-3.0-or-later
//! S2: parsed semantic forms, produced from a lossless CST with no error or
//! recovery node (ADR-011 §2.1 E2; ADR-012 §3, §4.3).
//!
//! This crate is the `forms` core (ADR-011 §6.1 layer 2) — the closed,
//! family-keyed dispatch entry table over the closed leading-token-kind
//! enum, and the shared parsed-form contract — and the family form
//! builders that have migrated onto it: the `Value` family's (`value`,
//! FR-091, M-3b for `Value`). Each other family's own migration ticket adds
//! its production function and its variants when it migrates.
//!
//! The `Expression` arena, its `ExprNode` enum and their sibling
//! parsed-form types are defined exactly once, in `syntax`, and re-exported
//! at this crate's top level.
//!
//! ## Layering
//!
//! `qsl-forms` is layer 2 (ADR-011 §6.1), which depends on layer 1
//! (`qsl-cst`), F (`qsl-foundation`), K (`quire-exact`) and the shared
//! walker leaf W (`quire-walk`), and on nothing else. It carries kernel types directly as parsed-form payloads
//! (ADR-013 OQ-A): `quire_exact::Integer` for an integer literal and
//! `quire_exact::CollectionKind` for a collection kind, both in an
//! [`ExprNode`] and in a [`TypeForm`]'s collection head. A parsed form
//! holds no `ValueType` and no `NodeKey` (FR-091-AC-11): [`ExprNode`]'s
//! `AllInstances`, `Lookup` and `Convert` variants, and
//! [`FunctionDeclaration`]'s `parameters`/`result`, each carry a
//! [`TypeForm`], resolved to the kernel `ValueType` only at check (E3,
//! ADR-013 O-14/C-26). See `syntax::TypeForm`'s own doc.

mod dispatch;
mod protocol_clause;
mod spans;
mod syntax;
mod value;

pub use dispatch::{
    build_unit, FormsCause, FormsRefusal, LeadingTokenKind, ParsedForm, ParsedUnit,
};
pub use spans::{DeclarationSpans, ExpressionSpans, SpanId, SpanRefusal, SpansMismatch};
pub use syntax::{
    Accumulation, AliasForm, AnchorForm, AnchorSegment, AnchorSite, AttemptForm, BinaryOperator,
    BinderForm, BinderKind, BinderQuery, BuiltinType, ClauseKind, DeclarationForm, DeclarationKind,
    DeclaredClauseKind, DeclaredName, DimensionForm, DimensionTermForm, EnumForm, EnumMemberForm,
    ExactNumberForm, ExactNumberKind, ExprId, ExprNode, ExprRef, Expression, ExpressionBuilder,
    FieldInitializer, FunctionDeclaration, NameForm, ProtocolBodyForm, ProtocolConstructForm,
    ProtocolConstructKind, ProtocolDeclarationForm, ProtocolNodeDeclaration, ProtocolNodeKind,
    RecordFieldForm, RecordForm, RoleForm, ScopeEntry, ScopeId, ScopeName, ScopedAnchorForm,
    ShapeChanged, StateClauseForm, StateClauseKind, TermOperator, TreeRefusal, TupleForm, TypeForm,
    TypeFormHead, UnitForm, UsingAlias,
};

/// Each explicit heap stack S2 and the form traits keep grows by at most a
/// constant per CST node S1 already charged (ADR-030 D-1 item 3).
#[cfg(test)]
mod stack_charge_tests {
    use super::syntax::stack_peak::take;
    use super::{build_unit, DeclarationForm, ParsedUnit};
    use ix_trace_rs::trace;
    use qsl_foundation::SourceIdentity;

    const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
        profile v = \"quire.value.complete/v1\";\n\
        model Config = \"example/config-version\" version \"1\" digest \
        \"sha256-jcs:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n";

    /// Build `declarations` under S1 limits raised to fit them, returning
    /// the unit and the CST node count S1 charged.
    fn build(declarations: &str) -> (ParsedUnit, usize) {
        let text = format!("{HEADER}{declarations}\n");
        let parsed = qsl_cst::parse(
            SourceIdentity::new("a", "u", "git", "1"),
            "unit.native",
            text.as_bytes(),
            qsl_cst::Limits::default()
                .with_source_bytes(usize::MAX)
                .with_tokens(usize::MAX)
                .with_nodes(usize::MAX),
        )
        .expect("S1 reads the unit");
        assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
        let nodes = parsed.cst().nodes().len();
        (build_unit(&parsed).expect("S2 builds the unit"), nodes)
    }

    fn assert_charged(stack: &'static str, nodes: usize) {
        let peak = take(stack);
        assert!(peak > 0, "{stack} was not used");
        assert!(
            peak <= nodes,
            "{stack} peaked at {peak} for {nodes} CST nodes"
        );
    }

    /// The expression mapping stack and the expression `Debug` walk, on a
    /// 100,000-deep body.
    #[trace("TC-724", "FR-257-AC-1")]
    #[test]
    fn expression_stacks_grow_within_the_node_charge() {
        let (unit, nodes) = build(&format!(
            "function f using v(a: Boolean): Boolean pure {{ {}a }}",
            "not ".repeat(100_000)
        ));
        let DeclarationForm::Function(function) = unit.forms()[0].form() else {
            panic!("a function form");
        };
        assert!(!format!("{:?}", function.body).is_empty());
        assert_charged("expression", nodes);
        assert_charged("expression_debug", nodes);
    }

    /// The type-form build stack and the `TypeForm` clone, compare and
    /// `Debug` walks, on a 100,000-deep parameter type.
    #[trace("TC-724", "FR-257-AC-1")]
    #[test]
    fn type_form_stacks_grow_within_the_node_charge() {
        let (unit, nodes) = build(&format!(
            "function f using v(p: {}Boolean{}): Boolean pure {{ true }}",
            "Option<".repeat(100_000),
            ">".repeat(100_000)
        ));
        let DeclarationForm::Function(function) = unit.forms()[0].form() else {
            panic!("a function form");
        };
        let parameter = &function.parameters[0].1;
        let copy = parameter.clone();
        assert_eq!(&copy, parameter);
        assert!(!format!("{copy:?}").is_empty());
        for stack in [
            "type_form",
            "type_form_clone",
            "type_form_clone_built",
            "type_form_compare",
            "type_form_format",
        ] {
            assert_charged(stack, nodes);
        }
    }

    /// The control-anchor walk's stack, on a 10,000-deep `repeat` chain.
    #[trace("TC-724", "FR-257-AC-2")]
    #[test]
    fn the_control_anchor_stack_grows_within_the_node_charge() {
        let depth = 10_000;
        let mut chain = String::new();
        for level in 0..depth {
            chain.push_str(&format!(
                "repeat L{level} by R visible (true) max 2 invariant {{ true }} variant {{ 1 }} \
                 while {{ false }} sequence B{level} {{ }} exhausted sequence E{level} {{ "
            ));
        }
        chain.push_str(&"} ".repeat(depth));
        let (_, nodes) = build(&format!(
            "protocol P using v over (input: Config::ConfigVersion) on origin {{\n\
             role R on Config::ConfigVersion;\n\
             run sequence Main {{ {chain} }}\n\
             finish End as (outcome: Boolean) {{ true }};\n\
             }}"
        ));
        assert_charged("control_anchors", nodes);
    }
}

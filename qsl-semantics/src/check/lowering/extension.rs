// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-354 "Emission", QSpec FR-469: lower a checked declaration form of a
//! selected extension into its `extension`/`extension_form` node.
//!
//! Every clause value is its own node, reached by `reference`, never an
//! inline `literal`, `aggregate` or `application`. A literal clause value is
//! a `value`/`literal` node typed at the node of the type the typed-node
//! schema names, so a `30` written for `Int[1, 3650]` is typed `Int[1, 3650]`.
//! The form's node is its own semantic type, carries the declared name and
//! the unit's `SourceOwner`, and has one `declaration` occurrence spanning the
//! whole form. Its id is the structural id of its declaration path; the source
//! span enters neither it nor any clause node's id.

use quire_checked::node::{CheckedLiteral, NodeKind};
use quire_exact::{NodeKey, Value};
use quire_semantic_value::location::Location;

use super::{identifiers, require_admitted, Binders, Lowering};
use crate::check::extension::{extension_location, CheckedClause, CheckedExtensionForm};
use crate::check::family::OccurrenceRole;
use crate::check::ir::CheckedNode;
use crate::check::node_key::{BodyTerm, ExtensionClauseTerm, ExtensionTerm, NodeTag};
use crate::check::refusal::{CheckRefusal, KeyFault};

/// One checked extension form, as lowering reads it.
pub(crate) struct ExtensionFormInput<'a> {
    /// The form's index among the unit's extension forms.
    pub(crate) index: usize,
    /// The checked form.
    pub(crate) form: &'a CheckedExtensionForm,
}

impl Lowering<'_> {
    /// Lower and key the extension form `input`, recording its one
    /// `declaration` occurrence at the whole form. Returns the node's key.
    pub(crate) fn extension_form(
        &mut self,
        input: &ExtensionFormInput<'_>,
    ) -> Result<NodeKey, CheckRefusal> {
        let at = extension_location(input.index);
        let mut clauses = Vec::with_capacity(input.form.clauses.len());
        for (position, clause) in input.form.clauses.iter().enumerate() {
            let location = crate::check::extension::extension_clause_location(input.index, position);
            let value = self.extension_clause_value(clause, &location)?;
            clauses.push(ExtensionClauseTerm::new(clause.name.clone(), value));
        }
        let term = ExtensionTerm::new(
            input.form.definition.clone(),
            input.form.node_kind.clone(),
            clauses,
        );
        let declaration = identifiers([input.form.name.as_str()], &at)?;
        let node = self.insert_ref(
            &at,
            NodeTag::Extension,
            "extension_form",
            None,
            Some(declaration),
            BodyTerm::Extension(term),
        )?;
        let key = require_admitted(node, &at)?;
        self.record(key.clone(), OccurrenceRole::Declaration, at);
        Ok(key)
    }

    /// The key of the node of `clause`'s checked value, recording the type
    /// node it is checked against at the clause value's region.
    fn extension_clause_value(
        &mut self,
        clause: &CheckedClause,
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
        let expected = self.type_ref(&clause.expected, location)?;
        self.record(expected, OccurrenceRole::Type, location.clone());
        let root = clause.body.root();
        let term = match literal_of(root) {
            Some((node, value)) => self.literal_as(node, value, &clause.expected)?,
            None => {
                let mut binders = Binders::new(&clause.slot_names, &[]);
                self.expression(root, &mut binders)?
            }
        };
        term.key()
            .ok_or_else(|| super::fault(location, KeyFault::UnkeyableRequirements))
    }
}

/// The literal a checked clause value is, directly or as the literal an
/// integer interval admits: the clause value's node and the literal value.
fn literal_of(root: CheckedNode<'_>) -> Option<(CheckedNode<'_>, &Value)> {
    match root.kind() {
        NodeKind::Literal(CheckedLiteral(value)) => Some((root, value)),
        NodeKind::Coerce(operand, _) => {
            match root.at(*operand).kind() {
                NodeKind::Literal(CheckedLiteral(value)) => Some((root, value)),
                _ => None,
            }
        }
        _ => None,
    }
}

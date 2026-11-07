// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-047-AC-8: the composed reader invents no graph semantics for `parent`.

use crate::support::protocol_artifact as setup;

use ix_trace_rs::trace;
use quire_spec_language::linking::definition_source::RegisteredDefinition as R;
use quire_spec_language::protocol_artifact::{wire as w, Error, Unsupported};
use setup::Fixture;

/// Tracing: TC-131.
#[trace("TC-131", "FR-047-AC-8")]
#[test]
fn composed_reader_refuses_parent_instead_of_inventing_graph_semantics() {
    let fixture = Fixture::families();
    let mut offered = fixture.package.clone();
    offered.declarations[2].values[0].profile = fixture.definition(R::StateGraph);
    offered.declarations[2].requires.clear();
    offered.declarations[2].values[0].operation = w::ValueOperation::Parent {
        reference: setup::owned(2, 1),
        edge: fixture.export(w::ExportKind::Field, &["Node", "signed"]),
        universe: fixture.export(w::ExportKind::Population, &["Node", "nodes"]),
    };
    match fixture.offered(&offered).result() {
        Ok(_) => panic!("the offer was admitted"),
        Err(actual) => assert_eq!(actual, &Error::Unsupported(Unsupported::Feature)),
    }
}

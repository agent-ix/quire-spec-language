// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-049: exact authored/source/operation correspondence through the public API.

use super::*;
use quire_spec_language::Source;

fn invalid(linked: LinkedPackage<'_>, bindings: CheckBindings) {
    let source = linked.unit().source().clone();
    let error = check(linked, bindings, CheckLimits::default()).unwrap_err();
    assert_eq!(
        (error.phase, error.code),
        (Phase::Check, Code::InvalidModelBinding)
    );
    assert_eq!(error.source, *source.identity());
    assert!(!error.is_incomplete());
}

#[test]
#[trace("TC-049", "FR-016-AC-6")]
fn tc_049_source_labels_path_bytes_and_formal_identity_must_correspond() {
    let models = [native_rule_model::parts().model()];
    for changed in ["identity", "revision", "path", "bytes", "formal_collision"] {
        let (linked, mut bindings) = prepared(&models, "true", ClauseKind::Invariant).unwrap();
        let original = bindings.source.source();
        let mut labels = original.identity().clone();
        let mut path = original.path().to_owned();
        let mut text = original.text().to_owned();
        let mut formal = bindings.source.identity().clone();
        match changed {
            "identity" => labels.identity.push_str(":foreign"),
            "revision" => labels.revision.push_str(":foreign"),
            "path" => path.push_str(".foreign"),
            "bytes" => text.push_str("// different exact source\n"),
            "formal_collision" => formal = models[0].source().identity().clone(),
            _ => unreachable!(),
        }
        bindings.source = FormalSource::new(
            Source::read(labels, path, text.as_bytes(), 1_048_576).unwrap(),
            formal,
        );
        invalid(linked, bindings);
    }
}

#[test]
#[trace("TC-049", "FR-016-AC-6")]
fn tc_049_complete_source_ordered_mappings_preserve_qualified_local_ids() {
    let models = [native_rule_model::parts().model()];
    let text = "invariant First on M::Node at current { self.n < 1000 implies self.n + 1 <= 1000 }\ninvariant Second on M::Node at current { true }";
    for changed in [
        "valid",
        "missing",
        "extra",
        "foreign",
        "duplicate_name",
        "duplicate_identity",
    ] {
        let (linked, source) = program(&models, text).unwrap();
        let original = linked.unit().clone();
        let make = |name: &str, owner| ClauseBinding {
            name: name.into(),
            requirement: owner,
            clause: ir::ClauseId::new("shared_local_id").unwrap(),
            execution_point: ir::ExecutionPoint::Handler {
                name: ir::AnchorName::new("validate").unwrap(),
            },
        };
        let second_owner = ir::RequirementRef::new(
            ir::PackageId::new("example/another-author").unwrap(),
            ir::RequirementId::new("ParentRule").unwrap(),
            ir::RequirementRevision::new(7).unwrap(),
        );
        let first = make("First", authored_owner());
        let second = make("Second", second_owner);
        // Caller order may differ from source order. Qualified IDs may share a local ID.
        let mut bindings = CheckBindings {
            source,
            clauses: vec![second.clone(), first.clone()],
        };
        match changed {
            "valid" => (),
            "missing" => {
                bindings.clauses.pop();
            }
            "extra" => bindings.clauses.push(first.clone()),
            "foreign" => bindings.clauses[0].name = "Unwritten".into(),
            "duplicate_name" => bindings.clauses[0].name = "First".into(),
            "duplicate_identity" => bindings.clauses[0].requirement = authored_owner(),
            _ => unreachable!(),
        }
        if changed != "valid" {
            invalid(linked, bindings);
            continue;
        }
        let checked = check(linked, bindings, CheckLimits::default()).unwrap();
        assert_eq!(
            checked.linked().unit().expressions(),
            original.expressions()
        );
        assert_eq!(checked.linked().unit().clauses(), original.clauses());
        assert_eq!(
            checked.linked().unit().source().digest(),
            original.source().digest()
        );
        assert_eq!(
            checked.linked().unit().source().identity(),
            original.source().identity()
        );
        assert_eq!(checked.clauses()[0].binding(), &first);
        assert_eq!(checked.clauses()[1].binding(), &second);
        assert!(std::ptr::eq(
            checked.linked().models()[0].native_model().unwrap(),
            &models[0]
        ));
        for (index, clause) in checked.clauses().iter().enumerate() {
            assert_eq!(
                clause.proof_environment().owner(),
                &clause.binding().requirement
            );
            assert_eq!(
                clause.expression_type(original.clauses()[index].expression),
                Some(&NativeType::Boolean)
            );
            assert_eq!(
                clause.expression_type(original.clauses()[1 - index].expression),
                None
            );
            for value in clause.proof_values() {
                let native = original.expression(value.expression).unwrap();
                assert_eq!(
                    value.source,
                    checked
                        .bindings()
                        .source
                        .to_ir(original.source(), native.span)
                        .unwrap()
                );
                assert!(clause
                    .proof_environment()
                    .values()
                    .iter()
                    .any(|d| d.name() == &value.symbol));
                for declaration in &value.declarations {
                    assert_eq!(declaration.identity.owner, *models[0].environment().owner());
                    assert!(models[0].source().to_native(&declaration.source).is_ok());
                }
            }
        }
    }
}

#[test]
#[trace("TC-049", "FR-016-AC-6")]
fn tc_049_execution_points_must_match_clause_category_and_selected_anchor() {
    let models = [native_rule_model::parts().model()];
    for (kind, point) in [
        (
            ClauseKind::Invariant,
            ir::ExecutionPoint::Pre {
                operation: ir::AnchorName::new("step").unwrap(),
            },
        ),
        (
            ClauseKind::Precondition,
            ir::ExecutionPoint::Post {
                operation: ir::AnchorName::new("step").unwrap(),
            },
        ),
        (
            ClauseKind::Postcondition,
            ir::ExecutionPoint::Handler {
                name: ir::AnchorName::new("step").unwrap(),
            },
        ),
        (
            ClauseKind::Precondition,
            ir::ExecutionPoint::Pre {
                operation: ir::AnchorName::new("other_step").unwrap(),
            },
        ),
        (
            ClauseKind::Postcondition,
            ir::ExecutionPoint::Post {
                operation: ir::AnchorName::new("other_step").unwrap(),
            },
        ),
    ] {
        let (linked, mut bindings) = prepared(&models, "true", kind).unwrap();
        bindings.clauses[0].execution_point = point;
        invalid(linked, bindings);
    }
    let (linked, mut bindings) = prepared(&models, "true", ClauseKind::Invariant).unwrap();
    let point = ir::ExecutionPoint::Initialization {
        name: ir::AnchorName::new("initialize").unwrap(),
    };
    bindings.clauses[0].execution_point = point.clone();
    let checked = check(linked, bindings, CheckLimits::default()).unwrap();
    assert_eq!(checked.clauses()[0].binding().execution_point, point);
}

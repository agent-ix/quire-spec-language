//! TC-235 state-machine binder vectors (FR-208).

use super::*;

fn lifecycle_state(owner: &str, name: &str) -> Value {
    let node = format!("{owner}/{name}");
    json!({"identity": id(&node), "name": name, "origin": origin(&node)})
}

fn lifecycle_transition(
    owner: &str,
    name: &str,
    from: &str,
    to: &str,
    trigger: &str,
    emits: Vec<String>,
) -> Value {
    let node = format!("{owner}/{name}");
    json!({
        "identity": id(&node), "from": id(&format!("{owner}/{from}")),
        "to": id(&format!("{owner}/{to}")),
        "trigger": id(&format!("{owner}/{trigger}")),
        "emits": emits, "origin": origin(&node),
    })
}

/// A typed `missing_declaration`/`missing-name` naming the state machine
/// `OrderLifecycle` itself (not a transition node), at reader pointer `path`.
fn assert_missing_name(refusal: &ModelRefusal, path: &str, reference: Option<&str>) {
    assert_eq!(refusal.code, Code::MissingDeclaration);
    assert_eq!(refusal.cause.as_str(), "missing-name");
    let ModelRefusalCause::IntakeMissingDeclaration {
        node,
        reference: absent,
        path: actual,
        ..
    } = &refusal.cause
    else {
        panic!("expected intake missing-name, got {refusal:?}");
    };
    assert_eq!(node, &id("OrderLifecycle"));
    assert_eq!(actual.as_str(), path);
    if let Some(reference) = reference {
        assert_eq!(absent, &id(reference));
    }
}

/// A typed `conflicting-binding` whose key is the repeated declaration.
fn assert_conflicting_binding(refusal: &ModelRefusal, key: &str, path: &str) {
    assert_eq!(refusal.code, Code::InvalidModelBinding);
    assert_eq!(refusal.cause.as_str(), "conflicting-binding");
    let ModelRefusalCause::ConflictingBinding {
        key: actual,
        member: actual_member,
    } = &refusal.cause
    else {
        panic!("expected conflicting-binding, got {refusal:?}");
    };
    assert_eq!(actual.node, id(key));
    assert_eq!(actual_member.as_ref(), Some(&member(path)));
}

/// M05: resolve transition targets, triggers, guards, and emitted event types
/// at the K2 intake/normalization boundary without accepting invalid forms.
#[test]
fn tc235_m05_state_machine_transition_reference_vectors() {
    let cases = [
        (
            "to",
            json!(id("OrderLifecycle/Shipped")),
            "/types/4/transitions/0/to",
            "OrderLifecycle/Shipped",
        ),
        (
            "trigger",
            json!(id("OrderLifecycle/ship")),
            "/types/4/transitions/0/trigger",
            "OrderLifecycle/ship",
        ),
        (
            "guard",
            json!("Unknown"),
            "/types/4/transitions/0/guard",
            "OrderLifecycle/Unknown",
        ),
    ];
    for (member, value, path, reference) in cases {
        let document = mutate_k2(|document| {
            type_named_mut(document, "OrderLifecycle")["transitions"][0][member] = value;
        });
        let refusals = normalized_refusals(&document, 21);
        assert_eq!(refusals.len(), 1, "M05 {member}: {refusals:?}");
        assert_missing_name(&refusals[0], path, Some(reference));
    }

    let wrong_emitted_kind = mutate_k2(|document| {
        type_named_mut(document, "OrderLifecycle")["transitions"][0]["emits"] =
            json!([id("Order")]);
    });
    let refusals = normalized_refusals(&wrong_emitted_kind, 21);
    assert_eq!(refusals.len(), 1);
    assert_malformed_at(
        &refusals[0],
        &id("OrderLifecycle"),
        "TypeDefinition.transitions[0].emits[0]",
    );

    let unknown_emitted_node = mutate_k2(|document| {
        type_named_mut(document, "OrderLifecycle")["transitions"][0]["emits"] =
            json!([id("OrderShipped")]);
    });
    let refusals = normalized_refusals(&unknown_emitted_node, 21);
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert_missing_name(
        &refusals[0],
        "/types/4/transitions/0/emits/0",
        Some("OrderShipped"),
    );

    let wrong_guard_language = mutate_k2(|document| {
        type_named_mut(document, "OrderLifecycle")["clauses"][0]["language"] = json!("ocl");
    });
    let refusals = normalized_refusals(&wrong_guard_language, 21);
    assert_eq!(refusals.len(), 1);
    assert_malformed_at(
        &refusals[0],
        &id("OrderLifecycle/transition0"),
        "TypeDefinition.transitions[0].guard",
    );
}

/// M06: require nonempty operations, states, and transitions; distinguish
/// invariant clauses from transition guards; refuse duplicate local names.
#[test]
fn tc235_m06_state_machine_required_members_and_bindings() {
    let empty_operations = mutate_k2(|document| {
        type_named_mut(document, "OrderLifecycle")["operations"] = json!([]);
    });
    let refusals = normalized_refusals(&empty_operations, 19);
    assert_eq!(refusals.len(), 4);
    assert_malformed_at(
        &refusals[0],
        &id("OrderLifecycle"),
        "TypeDefinition.operations",
    );
    for (position, (refusal, reference)) in refusals
        .iter()
        .skip(1)
        .zip(["place", "cancel", "cancel"])
        .enumerate()
    {
        assert_missing_name(
            refusal,
            &format!("/types/4/transitions/{position}/trigger"),
            Some(&format!("OrderLifecycle/{reference}")),
        );
    }

    let empty_states = mutate_k2(|document| {
        type_named_mut(document, "OrderLifecycle")["states"] = json!([]);
    });
    let refusals = normalized_refusals(&empty_states, 21);
    assert_eq!(refusals.len(), 7);
    assert_malformed_at(&refusals[0], &id("OrderLifecycle"), "TypeDefinition.states");
    for (refusal, path) in refusals.iter().skip(1).zip((0..3).flat_map(|position| {
        ["from", "to"].map(|member| format!("/types/4/transitions/{position}/{member}"))
    })) {
        assert_missing_name(refusal, &path, None);
    }

    let empty_transitions = mutate_k2(|document| {
        type_named_mut(document, "OrderLifecycle")["transitions"] = json!([]);
    });
    let refusals = normalized_refusals(&empty_transitions, 21);
    assert_eq!(refusals.len(), 1);
    assert_malformed_at(
        &refusals[0],
        &id("OrderLifecycle"),
        "TypeDefinition.transitions",
    );

    let extra_invariant = mutate_k2(|document| {
        type_named_mut(document, "OrderLifecycle")["clauses"]
            .as_array_mut()
            .unwrap()
            .push(clause(
                "OrderLifecycle",
                "Extra",
                "self.placed or not self.placed",
            ));
    });
    let package = read_k2(&extra_invariant).expect("an unreferenced invariant is valid");
    record_charges(&package, 22);
    // M06 run 4: a machine's invariants are its owned `quire` clauses that no
    // transition names as a guard (FR-208). `Extra` is one; `NotYetPlaced` is
    // the guard of `[0]` and so is no invariant. (The FR-094 clause kind is
    // chosen in check/lowering, outside this model-layer check.)
    let machine = package
        .records
        .iter()
        .find_map(|record| match record {
            DomainPackageRecord::StateMachine(machine)
                if machine.key.node == id("OrderLifecycle") =>
            {
                Some(machine)
            }
            _ => None,
        })
        .expect("OrderLifecycle retains its state machine");
    assert_eq!(
        machine.transitions[0].guard.as_deref(),
        Some("NotYetPlaced")
    );
    let guards: Vec<_> = machine
        .transitions
        .iter()
        .filter_map(|transition| transition.guard.as_deref())
        .map(|guard| id(&format!("OrderLifecycle/{guard}")))
        .collect();
    let invariants: Vec<_> = package
        .records
        .iter()
        .filter_map(|record| match record {
            DomainPackageRecord::Clause(clause)
                if clause.owner == machine.key
                    && clause.language == "quire"
                    && !guards.contains(&clause.key.node) =>
            {
                Some(clause.key.node.clone())
            }
            _ => None,
        })
        .collect();
    assert_eq!(invariants, [id("OrderLifecycle/Extra")]);

    let duplicate_state = mutate_k2(|document| {
        type_named_mut(document, "OrderLifecycle")["states"]
            .as_array_mut()
            .unwrap()
            .push(lifecycle_state("OrderLifecycle", "Draft"));
    });
    let refusals = normalized_refusals(&duplicate_state, 21);
    assert_eq!(refusals.len(), 1);
    assert_conflicting_binding(
        &refusals[0],
        "OrderLifecycle/Draft",
        "TypeDefinition.states[3]",
    );

    let duplicate_transition = mutate_k2(|document| {
        type_named_mut(document, "OrderLifecycle")["transitions"]
            .as_array_mut()
            .unwrap()
            .push(lifecycle_transition(
                "OrderLifecycle",
                "transition3",
                "Draft",
                "Cancelled",
                "cancel",
                vec![],
            ));
    });
    let refusals = normalized_refusals(&duplicate_transition, 21);
    assert_eq!(refusals.len(), 1);
    assert_conflicting_binding(
        &refusals[0],
        "OrderLifecycle/transition3",
        "TypeDefinition.transitions[3]",
    );

    let operation_postcondition = mutate_k2(|document| {
        type_named_mut(document, "OrderLifecycle")["operations"][0]["post"] = json!([{
            "language": "quire",
            "text": "self.placed",
            "origin": origin("OrderLifecycle/place"),
        }]);
    });
    let package = read_k2(&operation_postcondition).expect("operation postcondition is valid");
    // "no added declaration": the same 21 charges as K2 itself.
    record_charges(&package, 21);
}

/// M06 run 7: `self.placed` is bound as the postcondition of `place`.
#[test]
#[ignore = "QSL-726: intake keeps no operation pre/post clauses yet"]
fn tc235_m06_run7_postcondition_is_bound_to_place() {
    let document = mutate_k2(|document| {
        type_named_mut(document, "OrderLifecycle")["operations"][0]["post"] = json!([{
            "language": "quire",
            "text": "self.placed",
            "origin": origin("OrderLifecycle/place"),
        }]);
    });
    let package = read_k2(&document).expect("operation postcondition is valid");
    assert!(package.records.iter().any(|record| matches!(
        record,
        DomainPackageRecord::OperationMember(operation)
            if operation.key.node == id("OrderLifecycle/place")
                && !operation.own_postcondition_clauses.is_empty()
    )));
}

/// M11: only object/event/record/state-machine/process meanings admit
/// same-meaning supertypes; unsupported meanings must not be malformed refs.
#[test]
fn tc235_m11_state_machine_supertype_meaning_rules() {
    for node in ["Money", "OrderPlaced", "OrderLifecycle", "Fulfilment"] {
        let document = mutate_k2(|document| {
            type_named_mut(document, node)["supertypes"] = json!([id("Order")]);
        });
        let refusals = normalized_refusals(&document, 21);
        assert_eq!(refusals.len(), 1, "{node}: {refusals:?}");
        assert_malformed_at(&refusals[0], &id(node), "TypeDefinition.supertypes[0]");
    }
    for node in ["OrderStore", "OrderManagement"] {
        let document = mutate_k2(|document| {
            type_named_mut(document, node)["supertypes"] = json!([id("Order")]);
        });
        let refusals = normalized_refusals(&document, 21);
        assert_eq!(refusals.len(), 1, "{node}: {refusals:?}");
        assert_unsupported_at(&refusals[0], &id(node), "TypeDefinition.supertypes");
    }
}

/// M12: the explicit construct meaning is authoritative over a lifecycle or
/// saga kind name and shape.
#[test]
fn tc235_m12_state_machine_meaning_is_explicit() {
    for (kind, node) in [("lifecycle", "OrderLifecycle"), ("saga", "Fulfilment")] {
        let document = mutate_k2(|document| {
            let construct = document["constructs"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|entry| entry["kind"]["name"] == kind)
                .expect("K2 construct exists");
            construct["construct"]["meaning"] = json!(if kind == "lifecycle" {
                "quire.meaning.state-machine"
            } else {
                "quire.meaning.process"
            });
        });
        let refusals = normalized_refusals(&document, 21);
        assert_eq!(refusals.len(), 1, "{node}: {refusals:?}");
        assert_malformed_at(&refusals[0], &id(node), "constructs[].meaning");
    }
}

/// M19: inherited states/transitions form the subtype's effective machine;
/// duplicate local names and ancestor transitions to subtype-only states are
/// rejected at their authored declaration paths.
#[test]
fn tc235_m19_state_machine_specialization_vectors() {
    let positive = mutate_k2(|document| {
        document["types"].as_array_mut().unwrap().push(declaration(
            "ExpressLifecycle",
            "lifecycle",
            json!({
                "supertypes": [id("OrderLifecycle")],
                "operations": [operation("ExpressLifecycle", "expedite")],
                "states": [lifecycle_state("ExpressLifecycle", "Expedited")],
                "transitions": [lifecycle_transition(
                    "ExpressLifecycle",
                    "transition0", "Placed", "Expedited", "expedite", vec![],
                )],
            }),
        ));
    });
    let package = read_k2(&positive).expect("same-meaning state-machine subtype is valid");
    record_charges(&package, 23);

    let inherited_state_and_guard = mutate_k2(|document| {
        document["types"].as_array_mut().unwrap().push(declaration(
            "ExpressLifecycle",
            "lifecycle",
            json!({
                "supertypes": [id("OrderLifecycle")],
                "operations": [operation("ExpressLifecycle", "expedite")],
                "states": [lifecycle_state("ExpressLifecycle", "Expedited")],
                "transitions": [lifecycle_transition(
                    "ExpressLifecycle",
                    "transition0", "Placed", "Expedited", "expedite", vec![],
                )],
            }),
        ));
        type_named_mut(document, "ExpressLifecycle")["transitions"][0]["guard"] =
            json!("NotYetPlaced");
    });
    let package = read_k2(&inherited_state_and_guard)
        .expect("a subtype can name an inherited state and inherited transition guard");
    let view = record_charges(&package, 23);
    let child_key = DeclarationKey {
        package: PACKAGE.to_owned(),
        node: id("ExpressLifecycle"),
    };
    assert!(view.type_identities().contains_key(&child_key));

    let duplicate_state = mutate_k2(|document| {
        document["types"].as_array_mut().unwrap().push(declaration(
            "ExpressLifecycle",
            "lifecycle",
            json!({
                "supertypes": [id("OrderLifecycle")],
                "operations": [operation("ExpressLifecycle", "expedite")],
                "states": [lifecycle_state("ExpressLifecycle", "Expedited")],
                "transitions": [lifecycle_transition(
                    "ExpressLifecycle",
                    "transition0", "Placed", "Expedited", "expedite", vec![],
                )],
            }),
        ));
    });
    let refusals = normalized_refusals(&duplicate_state, 23);
    assert_eq!(refusals.len(), 1);
    assert_eq!(refusals[0].code, Code::InvalidModelBinding);
    assert_eq!(refusals[0].cause.as_str(), "derivation-conflict");
    assert_derivation_paths(
        &refusals[0],
        &[
            ("ExpressLifecycle", "TypeDefinition.states[1]"),
            ("OrderLifecycle", "TypeDefinition.states[1]"),
        ],
    );

    let duplicate_transition = mutate_k2(|document| {
        document["types"].as_array_mut().unwrap().push(declaration(
            "ExpressLifecycle",
            "lifecycle",
            json!({
                "supertypes": [id("OrderLifecycle")],
                "operations": [operation("ExpressLifecycle", "expedite")],
                "states": [lifecycle_state("ExpressLifecycle", "Expedited"), lifecycle_state("ExpressLifecycle", "Placed")],
                "transitions": [
                    lifecycle_transition("ExpressLifecycle", "transition0", "Placed", "Expedited", "expedite", vec![]),
                    lifecycle_transition("ExpressLifecycle", "transition1", "Draft", "Placed", "place", vec![]),
                ],
            }),
        ));
    });
    let refusals = normalized_refusals(&duplicate_transition, 23);
    assert_eq!(refusals.len(), 1);
    assert_eq!(refusals[0].cause.as_str(), "derivation-conflict");
    assert_derivation_paths(
        &refusals[0],
        &[
            ("ExpressLifecycle", "TypeDefinition.transitions[1]"),
            ("OrderLifecycle", "TypeDefinition.transitions[0]"),
        ],
    );

    let ancestor_to_subtype_state = mutate_k2(|document| {
        type_named_mut(document, "OrderLifecycle")["transitions"]
            .as_array_mut()
            .unwrap()
            .push(lifecycle_transition(
                "OrderLifecycle",
                "transition3",
                "Draft",
                "Expedited",
                "cancel",
                vec![],
            ));
        document["types"].as_array_mut().unwrap().push(declaration(
            "ExpressLifecycle",
            "lifecycle",
            json!({
                "supertypes": [id("OrderLifecycle")],
                "operations": [operation("ExpressLifecycle", "expedite")],
                "states": [lifecycle_state("ExpressLifecycle", "Expedited")],
                "transitions": [lifecycle_transition(
                    "ExpressLifecycle",
                    "transition0", "Placed", "Expedited", "expedite", vec![],
                )],
            }),
        ));
    });
    let refusals = normalized_refusals(&ancestor_to_subtype_state, 23);
    assert_eq!(refusals.len(), 1);
    assert_malformed_at(
        &refusals[0],
        &id("OrderLifecycle"),
        "TypeDefinition.transitions[3].to",
    );

    let wrong_supertype = mutate_k2(|document| {
        document["types"].as_array_mut().unwrap().push(declaration(
            "ExpressLifecycle",
            "lifecycle",
            json!({
                "supertypes": [id("Fulfilment")],
                "operations": [operation("ExpressLifecycle", "expedite")],
                "states": [lifecycle_state("ExpressLifecycle", "Expedited")],
                "transitions": [lifecycle_transition(
                    "ExpressLifecycle",
                    "transition0", "Placed", "Expedited", "expedite", vec![],
                )],
            }),
        ));
    });
    let refusals = normalized_refusals(&wrong_supertype, 23);
    assert_eq!(refusals.len(), 1);
    assert_malformed_at(
        &refusals[0],
        &id("ExpressLifecycle"),
        "TypeDefinition.supertypes[0]",
    );
}

/// M19 run 2: the derivation conflict carries both sides' member paths as
/// typed values (the spec's `ExpressLifecycle.states[1]` is the owner and the
/// member `TypeDefinition.states[1]`).
#[test]
fn tc235_m19_derivation_conflict_member_paths() {
    let document = mutate_k2(|document| {
        document["types"].as_array_mut().unwrap().push(declaration(
            "ExpressLifecycle",
            "lifecycle",
            json!({
                "supertypes": [id("OrderLifecycle")],
                "operations": [operation("ExpressLifecycle", "expedite")],
                "states": [
                    lifecycle_state("ExpressLifecycle", "Expedited"),
                    lifecycle_state("ExpressLifecycle", "Placed"),
                ],
                "transitions": [lifecycle_transition(
                    "ExpressLifecycle", "transition0", "Placed", "Expedited", "expedite", vec![],
                )],
            }),
        ));
    });
    let refusals = normalized_refusals(&document, 23);
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert_eq!(refusals[0].cause.as_str(), "derivation-conflict");
    assert_derivation_paths(
        &refusals[0],
        &[
            ("ExpressLifecycle", "TypeDefinition.states[1]"),
            ("OrderLifecycle", "TypeDefinition.states[1]"),
        ],
    );
}

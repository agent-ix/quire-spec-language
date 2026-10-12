//! TC-235 state-machine binder vectors (FR-208).

use super::*;

fn lifecycle_state(name: &str) -> Value {
    let node = format!("OrderLifecycle/{name}");
    json!({"identity": id(&node), "name": name, "origin": origin(&node)})
}

fn lifecycle_transition(
    name: &str,
    from: &str,
    to: &str,
    trigger: &str,
    emits: Vec<String>,
) -> Value {
    let node = format!("OrderLifecycle/{name}");
    json!({
        "identity": id(&node), "from": id(&format!("OrderLifecycle/{from}")),
        "to": id(&format!("OrderLifecycle/{to}")),
        "trigger": id(&format!("OrderLifecycle/{trigger}")),
        "emits": emits, "origin": origin(&node),
    })
}

/// M05: resolve transition targets, triggers, guards, and emitted event types
/// at the K2 intake/normalization boundary without accepting invalid forms.
#[test]
fn tc235_m05_state_machine_transition_reference_vectors() {
    let cases = [
        (
            "to",
            json!(id("OrderLifecycle/Shipped")),
            "TypeDefinition.transitions[0].to",
        ),
        (
            "trigger",
            json!(id("OrderLifecycle/ship")),
            "TypeDefinition.transitions[0].trigger",
        ),
        (
            "guard",
            json!("Unknown"),
            "TypeDefinition.transitions[0].guard",
        ),
    ];
    for (member, value, path) in cases {
        let document = mutate_k2(|document| {
            type_named_mut(document, "OrderLifecycle")["transitions"][0][member] = value;
        });
        let refusals = read_refusals(&document);
        assert!(
            refusals.iter().any(|refusal| {
                refusal.code == Code::MissingDeclaration
                    && refusal.cause.as_str() == "missing-name"
                    && refusal.detail.contains(path)
                    && refusal.detail.contains("OrderLifecycle")
            }),
            "M05 {member} should retain the typed missing-name locus: {refusals:?}"
        );
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
    let refusals = read_refusals(&unknown_emitted_node);
    assert!(
        refusals.iter().any(|refusal| {
            refusal.code == Code::MissingDeclaration
                && refusal.cause.as_str() == "missing-name"
                && refusal
                    .detail
                    .contains("TypeDefinition.transitions[0].emits[0]")
                && refusal.detail.contains("OrderShipped")
        }),
        "M05 absent emitted event must be missing-name: {refusals:?}"
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
    for refusal in refusals.iter().skip(1) {
        assert_eq!(refusal.code, Code::MissingDeclaration);
        assert_eq!(refusal.cause.as_str(), "missing-name");
        assert!(refusal.detail.contains("TypeDefinition.transitions"));
    }

    let empty_states = mutate_k2(|document| {
        type_named_mut(document, "OrderLifecycle")["states"] = json!([]);
    });
    let refusals = normalized_refusals(&empty_states, 21);
    assert_eq!(refusals.len(), 7);
    assert_malformed_at(&refusals[0], &id("OrderLifecycle"), "TypeDefinition.states");
    assert!(refusals.iter().skip(1).all(|refusal| {
        refusal.code == Code::MissingDeclaration
            && refusal.cause.as_str() == "missing-name"
            && (refusal.detail.contains(".from") || refusal.detail.contains(".to"))
    }));

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

    let duplicate_state = mutate_k2(|document| {
        type_named_mut(document, "OrderLifecycle")["states"]
            .as_array_mut()
            .unwrap()
            .push(lifecycle_state("Draft"));
    });
    let refusals = normalized_refusals(&duplicate_state, 21);
    assert_eq!(refusals.len(), 1);
    assert_eq!(refusals[0].code, Code::InvalidModelBinding);
    assert_eq!(refusals[0].cause.as_str(), "conflicting-binding");
    assert!(refusals[0].detail.contains("TypeDefinition.states[3]"));

    let duplicate_transition = mutate_k2(|document| {
        type_named_mut(document, "OrderLifecycle")["transitions"]
            .as_array_mut()
            .unwrap()
            .push(lifecycle_transition(
                "transition3",
                "Draft",
                "Cancelled",
                "cancel",
                vec![],
            ));
    });
    let refusals = normalized_refusals(&duplicate_transition, 21);
    assert_eq!(refusals.len(), 1);
    assert_eq!(refusals[0].code, Code::InvalidModelBinding);
    assert_eq!(refusals[0].cause.as_str(), "conflicting-binding");
    assert!(refusals[0].detail.contains("TypeDefinition.transitions[3]"));

    let operation_postcondition = mutate_k2(|document| {
        type_named_mut(document, "OrderLifecycle")["operations"][0]["ensures"] =
            json!([clause("OrderLifecycle/place", "Placed", "self.placed")]);
    });
    let package = read_k2(&operation_postcondition).expect("operation postcondition is valid");
    record_charges(&package, 21);
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
        assert_eq!(refusals[0].code, Code::UnsupportedConstruct);
        assert_eq!(refusals[0].cause.as_str(), "declaration-form");
        assert!(refusals[0].detail.contains(&id(node)));
        assert!(refusals[0].detail.contains("TypeDefinition.supertypes"));
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
                "states": [lifecycle_state("Expedited")],
                "transitions": [lifecycle_transition(
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
                "states": [lifecycle_state("Expedited")],
                "transitions": [lifecycle_transition(
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
                "states": [lifecycle_state("Expedited")],
                "transitions": [lifecycle_transition(
                    "transition0", "Placed", "Expedited", "expedite", vec![],
                )],
            }),
        ));
    });
    let refusals = normalized_refusals(&duplicate_state, 23);
    assert_eq!(refusals.len(), 1);
    assert_eq!(refusals[0].code, Code::InvalidModelBinding);
    assert_eq!(refusals[0].cause.as_str(), "derivation-conflict");
    assert!(refusals[0].detail.contains("ExpressLifecycle.states[1]"));
    assert!(refusals[0].detail.contains("OrderLifecycle.states[1]"));

    let duplicate_transition = mutate_k2(|document| {
        document["types"].as_array_mut().unwrap().push(declaration(
            "ExpressLifecycle",
            "lifecycle",
            json!({
                "supertypes": [id("OrderLifecycle")],
                "operations": [operation("ExpressLifecycle", "expedite")],
                "states": [lifecycle_state("Expedited"), lifecycle_state("Placed")],
                "transitions": [
                    lifecycle_transition("transition0", "Placed", "Expedited", "expedite", vec![]),
                    lifecycle_transition("transition1", "Draft", "Placed", "place", vec![]),
                ],
            }),
        ));
    });
    let refusals = normalized_refusals(&duplicate_transition, 23);
    assert_eq!(refusals.len(), 1);
    assert_eq!(refusals[0].cause.as_str(), "derivation-conflict");
    assert!(refusals[0]
        .detail
        .contains("ExpressLifecycle.transitions[1]"));
    assert!(refusals[0].detail.contains("OrderLifecycle.transitions[0]"));

    let ancestor_to_subtype_state = mutate_k2(|document| {
        type_named_mut(document, "OrderLifecycle")["transitions"]
            .as_array_mut()
            .unwrap()
            .push(lifecycle_transition(
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
                "states": [lifecycle_state("Expedited")],
                "transitions": [lifecycle_transition(
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
                "states": [lifecycle_state("Expedited")],
                "transitions": [lifecycle_transition(
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

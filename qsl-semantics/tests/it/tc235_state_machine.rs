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
    // A state already qualified as `Owner/State` names a state of another type.
    let qualify = |state: &str| {
        if state.contains('/') {
            state.to_owned()
        } else {
            format!("{owner}/{state}")
        }
    };
    let (from, to) = (qualify(from), qualify(to));
    json!({
        "identity": id(&node), "from": id(&from),
        "to": id(&to),
        "trigger": id(&qualify(trigger)),
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
fn assert_conflicting_binding(refusal: &ModelRefusal, key: &str) {
    assert_eq!(refusal.code, Code::InvalidModelBinding);
    assert_eq!(refusal.cause.as_str(), "conflicting-binding");
    let ModelRefusalCause::ConflictingBinding { key: actual } = &refusal.cause else {
        panic!("expected conflicting-binding, got {refusal:?}");
    };
    assert_eq!(actual.node, id(key));
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
    assert_conflicting_binding(&refusals[0], "OrderLifecycle/Draft");

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
    assert_conflicting_binding(&refusals[0], "OrderLifecycle/transition3");

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

/// The retained state machine at `node`, then every ancestor reached through
/// its `supertypes`.
fn machine_with_ancestors<'a>(
    package: &'a DomainPackage,
    node: &str,
) -> Vec<&'a qsl_semantics::model::domain_package::StateMachineRecord> {
    let mut chain = Vec::new();
    let mut pending = vec![id(node)];
    while let Some(next) = pending.pop() {
        let machine = package
            .records
            .iter()
            .find_map(|record| match record {
                DomainPackageRecord::StateMachine(machine) if machine.key.node == next => {
                    Some(machine)
                }
                _ => None,
            })
            .expect("the chain names a retained state machine");
        pending.extend(machine.supertypes.iter().map(|key| key.node.clone()));
        chain.push(machine);
    }
    chain
}

/// M19 runs 1 and 7: `ExpressLifecycle` conforms to `OrderLifecycle`, and its
/// effective states, transitions, operations and field are its own plus its
/// ancestor's (FR-208), with `NotYetPlaced` guarding the inherited `[0]` and,
/// when `own_guard`, its own `[0]`.
fn assert_express_effective_machine(package: &DomainPackage, own_guard: bool) {
    let chain = machine_with_ancestors(package, "ExpressLifecycle");
    let states: std::collections::BTreeSet<_> = chain
        .iter()
        .flat_map(|machine| machine.states.iter().map(|state| state.name.as_str()))
        .collect();
    assert_eq!(
        states,
        ["Cancelled", "Draft", "Expedited", "Placed"]
            .into_iter()
            .collect()
    );
    assert_eq!(
        chain[0]
            .supertypes
            .iter()
            .map(|key| &key.node)
            .collect::<Vec<_>>(),
        [&id("OrderLifecycle")],
        "ExpressLifecycle conforms to OrderLifecycle"
    );
    let order = chain[1];
    assert_eq!(order.transitions.len(), 3, "OrderLifecycle [0]-[2]");
    assert_eq!(order.transitions[0].guard.as_deref(), Some("NotYetPlaced"));
    assert_eq!(chain[0].transitions.len(), 1, "its own [0]");
    assert_eq!(
        chain[0].transitions[0].guard.as_deref(),
        own_guard.then_some("NotYetPlaced")
    );
    let operations: std::collections::BTreeSet<_> = package
        .records
        .iter()
        .filter_map(|record| match record {
            DomainPackageRecord::OperationMember(operation)
                if chain.iter().any(|machine| machine.key == operation.owner) =>
            {
                operation.key.node.rsplit('/').next()
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        operations,
        ["cancel", "expedite", "place"].into_iter().collect()
    );
    assert!(package.records.iter().any(|record| matches!(
        record,
        DomainPackageRecord::FieldMember(field)
            if field.key.node == id("OrderLifecycle/placed") && field.owner == order.key
    )));
}

fn assert_derivation_conflict(refusal: &ModelRefusal, type_: &str, member: &str, redefiner: &str) {
    assert_eq!(refusal.code, Code::InvalidModelBinding);
    let ModelRefusalCause::DerivationConflict {
        type_: actual_type,
        member: actual_member,
        redefiners,
    } = &refusal.cause
    else {
        panic!(
            "expected a typed derivation conflict, got {:?}",
            refusal.cause
        );
    };
    assert_eq!(actual_type.node, id(type_), "{refusal:?}");
    assert_eq!(actual_member.node, id(member), "{refusal:?}");
    assert_eq!(
        redefiners.iter().map(|key| &key.node).collect::<Vec<_>>(),
        [&id(redefiner)],
        "{refusal:?}"
    );
}

/// `ExpressLifecycle`, a subtype whose one transition runs from `from` to its
/// own `Expedited`, as the TC-235 M19 row declares it.
fn express_lifecycle(supertype: &str, states: &[&str], from: &str) -> Value {
    let own_states: Vec<Value> = states
        .iter()
        .map(|name| lifecycle_state("ExpressLifecycle", name))
        .collect();
    declaration(
        "ExpressLifecycle",
        "lifecycle",
        json!({
            "supertypes": [id(supertype)],
            "operations": [operation("ExpressLifecycle", "expedite")],
            "states": own_states,
            "transitions": [lifecycle_transition(
                "ExpressLifecycle", "transition0", from, "Expedited", "expedite", vec![],
            )],
        }),
    )
}

/// M19: inherited states/transitions form the subtype's effective machine;
/// duplicate local names and ancestor transitions to subtype-only states are
/// rejected at their authored declaration paths.
#[trace("QSpec-TC-235")]
#[test]
fn tc235_m19_state_machine_specialization_vectors() {
    let inherited = "OrderLifecycle/Placed";

    // Run 1.
    let positive = mutate_k2(|document| {
        document["types"]
            .as_array_mut()
            .unwrap()
            .push(express_lifecycle(
                "OrderLifecycle",
                &["Expedited"],
                inherited,
            ));
    });
    let package = read_k2(&positive).expect("same-meaning state-machine subtype is valid");
    record_charges(&package, 23);
    assert_express_effective_machine(&package, false);

    // Run 7: `NotYetPlaced` guards the subtype's transition too.
    let inherited_state_and_guard = mutate_k2(|document| {
        document["types"]
            .as_array_mut()
            .unwrap()
            .push(express_lifecycle(
                "OrderLifecycle",
                &["Expedited"],
                inherited,
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
    assert_express_effective_machine(&package, true);

    // Run 2: `Placed` redeclared beside the inherited one.
    let duplicate_state = mutate_k2(|document| {
        document["types"]
            .as_array_mut()
            .unwrap()
            .push(express_lifecycle(
                "OrderLifecycle",
                &["Expedited", "Placed"],
                "Placed",
            ));
    });
    let refusals = normalized_refusals(&duplicate_state, 23);
    assert_eq!(refusals.len(), 1);
    assert_derivation_conflict(
        &refusals[0],
        "ExpressLifecycle",
        "OrderLifecycle/Placed",
        "ExpressLifecycle/Placed",
    );
    assert!(refusals[0].detail.contains("ExpressLifecycle.states[1]"));
    assert!(refusals[0].detail.contains("OrderLifecycle.states[1]"));

    // Run 3: a transition repeating the inherited `Draft -> Placed` on `place`.
    let duplicate_transition = mutate_k2(|document| {
        let mut declaration = express_lifecycle("OrderLifecycle", &["Expedited"], inherited);
        declaration["transitions"]
            .as_array_mut()
            .unwrap()
            .push(lifecycle_transition(
                "ExpressLifecycle",
                "transition1",
                "OrderLifecycle/Draft",
                "OrderLifecycle/Placed",
                "OrderLifecycle/place",
                vec![],
            ));
        document["types"].as_array_mut().unwrap().push(declaration);
    });
    let refusals = normalized_refusals(&duplicate_transition, 23);
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert_derivation_conflict(
        &refusals[0],
        "ExpressLifecycle",
        "OrderLifecycle/transition0",
        "ExpressLifecycle/transition1",
    );
    assert!(refusals[0]
        .detail
        .contains("ExpressLifecycle.transitions[1]"));
    assert!(refusals[0].detail.contains("OrderLifecycle.transitions[0]"));

    // Run 4: an ancestor transition to a subtype-only state.
    let ancestor_to_subtype_state = mutate_k2(|document| {
        type_named_mut(document, "OrderLifecycle")["transitions"]
            .as_array_mut()
            .unwrap()
            .push(lifecycle_transition(
                "OrderLifecycle",
                "transition3",
                "Draft",
                "ExpressLifecycle/Expedited",
                "cancel",
                vec![],
            ));
        document["types"]
            .as_array_mut()
            .unwrap()
            .push(express_lifecycle(
                "OrderLifecycle",
                &["Expedited"],
                inherited,
            ));
    });
    let refusals = normalized_refusals(&ancestor_to_subtype_state, 23);
    assert_eq!(refusals.len(), 1);
    assert_malformed_at(
        &refusals[0],
        &id("OrderLifecycle"),
        "TypeDefinition.transitions[3].to",
    );

    // Run 5: the row's literal input, `supertypes: [Fulfilment]`. The single
    // refusal is at `supertypes[0]`; the `from` it also leaves unresolvable
    // is a phase-3 finding that never runs after an intake refusal.
    let wrong_supertype = mutate_k2(|document| {
        document["types"]
            .as_array_mut()
            .unwrap()
            .push(express_lifecycle("Fulfilment", &["Expedited"], inherited));
    });
    let refusals = normalized_refusals(&wrong_supertype, 23);
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert_malformed_at(
        &refusals[0],
        &id("ExpressLifecycle"),
        "TypeDefinition.supertypes[0]",
    );
}

/// M19 run 6 (FR-208-AC-12): a subtype transition whose guard names an
/// ancestor's invariant that no ancestor transition names would remove that
/// invariant from the subtype.
#[trace("QSpec-TC-235", "FR-208-AC-12")]
#[test]
fn tc235_m19_run6_a_subtype_guard_naming_an_ancestor_invariant_is_malformed() {
    let document = mutate_k2(|document| {
        type_named_mut(document, "OrderLifecycle")["clauses"]
            .as_array_mut()
            .unwrap()
            .push(clause(
                "OrderLifecycle",
                "Extra",
                "self.placed or not self.placed",
            ));
        document["types"]
            .as_array_mut()
            .unwrap()
            .push(express_lifecycle(
                "OrderLifecycle",
                &["Expedited"],
                "OrderLifecycle/Placed",
            ));
        type_named_mut(document, "ExpressLifecycle")["transitions"][0]["guard"] = json!("Extra");
    });
    let refusals = normalized_refusals(&document, 24);
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert_malformed_at(
        &refusals[0],
        &id("ExpressLifecycle"),
        "TypeDefinition.transitions[0].guard",
    );
}

/// FR-208: a machine naming a state, operation or clause of a machine that is
/// not its ancestor is malformed in phase 3, not a missing name.
#[trace("QSpec-TC-235")]
#[test]
fn tc235_a_reference_to_a_non_ancestor_machines_member_is_malformed() {
    for member in ["from", "to", "trigger", "guard"] {
        let document = mutate_k2(|document| {
            document["types"].as_array_mut().unwrap().push(declaration(
                "Standalone",
                "lifecycle",
                json!({
                    "operations": [operation("Standalone", "act")],
                    "states": [lifecycle_state("Standalone", "Idle")],
                    "transitions": [lifecycle_transition(
                        "Standalone", "transition0", "Idle", "Idle", "act", vec![],
                    )],
                }),
            ));
            let transition = &mut type_named_mut(document, "Standalone")["transitions"][0];
            match member {
                "from" => transition["from"] = json!(id("OrderLifecycle/Placed")),
                "to" => transition["to"] = json!(id("OrderLifecycle/Placed")),
                "trigger" => transition["trigger"] = json!(id("OrderLifecycle/place")),
                _ => transition["guard"] = json!("NotYetPlaced"),
            }
        });
        let refusals = normalized_refusals(&document, 23);
        assert_eq!(refusals.len(), 1, "{member}: {refusals:?}");
        assert_malformed_at(
            &refusals[0],
            &id("Standalone"),
            &format!("TypeDefinition.transitions[0].{member}"),
        );
    }
}

/// FR-208: a state redeclared against a grandparent's is a derivation
/// conflict with that grandparent, not only with the direct supertype.
#[trace("QSpec-TC-235")]
#[test]
fn tc235_a_grandparent_state_redeclaration_is_a_derivation_conflict() {
    let document = mutate_k2(|document| {
        let types = document["types"].as_array_mut().unwrap();
        types.push(express_lifecycle(
            "OrderLifecycle",
            &["Expedited"],
            "OrderLifecycle/Placed",
        ));
        types.push(declaration(
            "UrgentLifecycle",
            "lifecycle",
            json!({
                "supertypes": [id("ExpressLifecycle")],
                "operations": [operation("UrgentLifecycle", "rush")],
                "states": [lifecycle_state("UrgentLifecycle", "Placed")],
                "transitions": [lifecycle_transition(
                    "UrgentLifecycle", "transition0", "Placed", "Placed", "rush", vec![],
                )],
            }),
        ));
    });
    let refusals = normalized_refusals(&document, 25);
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert_derivation_conflict(
        &refusals[0],
        "UrgentLifecycle",
        "OrderLifecycle/Placed",
        "UrgentLifecycle/Placed",
    );
}

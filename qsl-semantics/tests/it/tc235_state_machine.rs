//! TC-235 state-machine binder vectors (FR-208).

use super::*;
use ix_trace_rs::trace;

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
fn assert_missing_name(
    refusal: &ModelRefusal,
    path: &str,
    member_path: &str,
    reference: Option<&str>,
) {
    assert_eq!(refusal.code, Code::MissingDeclaration);
    assert_eq!(refusal.cause.as_str(), "missing-name");
    let ModelRefusalCause::IntakeMissingDeclaration {
        node,
        reference: absent,
        path: actual,
        member: actual_member,
        ..
    } = &refusal.cause
    else {
        panic!("expected intake missing-name, got {refusal:?}");
    };
    assert_eq!(actual_member, &member(member_path));
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
        let member_path = format!("TypeDefinition.transitions[0].{member}");
        assert_missing_name(&refusals[0], path, &member_path, Some(reference));
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
        "TypeDefinition.transitions[0].emits[0]",
        Some("OrderShipped"),
    );

    let wrong_guard_language = mutate_k2(|document| {
        type_named_mut(document, "OrderLifecycle")["clauses"][0]["language"] = json!("ocl");
    });
    let refusals = normalized_refusals(&wrong_guard_language, 21);
    assert_eq!(refusals.len(), 1);
    assert_malformed_at(
        &refusals[0],
        &id("OrderLifecycle"),
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
            &format!("TypeDefinition.transitions[{position}].trigger"),
            Some(&format!("OrderLifecycle/{reference}")),
        );
    }

    let empty_states = mutate_k2(|document| {
        type_named_mut(document, "OrderLifecycle")["states"] = json!([]);
    });
    let refusals = normalized_refusals(&empty_states, 21);
    assert_eq!(refusals.len(), 7);
    assert_malformed_at(&refusals[0], &id("OrderLifecycle"), "TypeDefinition.states");
    for (refusal, (path, member_path)) in refusals.iter().skip(1).zip((0..3).flat_map(|position| {
        ["from", "to"].map(|member| {
            (
                format!("/types/4/transitions/{position}/{member}"),
                format!("TypeDefinition.transitions[{position}].{member}"),
            )
        })
    })) {
        assert_missing_name(refusal, &path, &member_path, None);
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

/// M06 run 7 (FR-103-AC-7): `self.placed` is kept and bound as the
/// postcondition of `place`, with its source identity.
#[trace("FR-103-AC-7", "TC-993")]
#[test]
fn tc235_m06_run7_postcondition_is_bound_to_place() {
    let document = mutate_k2(|document| {
        type_named_mut(document, "OrderLifecycle")["operations"][0]["post"] = json!([{
            "language": "quire",
            "text": "self.placed",
            "origin": {"source": {
                "sourceIdentity": "ix://test/orders/source",
                "path": "orders.quire",
                "startLine": 7,
                "startColumn": 3,
            }},
        }]);
    });
    let package = read_k2(&document).expect("operation postcondition is valid");
    let place = package
        .records
        .iter()
        .find_map(|record| match record {
            DomainPackageRecord::OperationMember(operation)
                if operation.key.node == id("OrderLifecycle/place") =>
            {
                Some(operation)
            }
            _ => None,
        })
        .expect("K2 declares OrderLifecycle/place");
    assert!(place.requires.is_empty());
    assert_eq!(place.ensures.len(), 1);
    let ensures = &place.ensures[0];
    assert_eq!(ensures.operation, place.key);
    assert_eq!(ensures.position, 0);
    assert_eq!(ensures.language, "quire");
    assert_eq!(ensures.text, "self.placed");
    assert!(
        ensures.artifact.is_some(),
        "the clause keeps its artifact id"
    );
    assert!(ensures.span.is_some(), "the clause keeps its span");
    record_charges(&package, 21);
}

/// FR-103-AC-7: `pre` entries are kept as `requires`, in declared order, and
/// an operation that declares neither list keeps both empty.
#[trace("FR-103-AC-7", "TC-993")]
#[test]
fn operation_pre_is_kept_as_requires_in_order() {
    let document = mutate_k2(|document| {
        type_named_mut(document, "OrderLifecycle")["operations"][0]["pre"] = json!([
            {"language": "quire", "text": "not self.placed", "origin": origin("OrderLifecycle/place")},
            {"language": "other", "text": "anything", "origin": origin("OrderLifecycle/place")},
        ]);
    });
    let package = read_k2(&document).expect("operation precondition is valid");
    let place = package
        .records
        .iter()
        .find_map(|record| match record {
            DomainPackageRecord::OperationMember(operation)
                if operation.key.node == id("OrderLifecycle/place") =>
            {
                Some(operation)
            }
            _ => None,
        })
        .expect("K2 declares OrderLifecycle/place");
    let kept: Vec<_> = place
        .requires
        .iter()
        .map(|clause| {
            (
                clause.position,
                clause.language.as_str(),
                clause.text.as_str(),
            )
        })
        .collect();
    assert_eq!(
        kept,
        [(0, "quire", "not self.placed"), (1, "other", "anything")]
    );
    assert!(place.ensures.is_empty());
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

/// The summary of one effective state or transition: its declaring machine's
/// node, its key's node and its position.
type Entry = (String, String, usize);

fn entry(declaring: &DeclarationKey, key: &DeclarationKey, position: usize) -> Entry {
    (declaring.node.clone(), key.node.clone(), position)
}

fn expected_entries(entries: &[(&str, &str, usize)]) -> Vec<Entry> {
    entries
        .iter()
        .map(|(declaring, key, position)| (id(declaring), id(key), *position))
        .collect()
}

/// M19 runs 1 and 7 (FR-081-AC-17, AC-19, AC-21), asserted through the view:
/// `ExpressLifecycle`'s effective states and transitions are its own then
/// `OrderLifecycle`'s, ordered by declaring machine key bytes then position,
/// with `NotYetPlaced` guarding the inherited `[0]` and, when `own_guard`, its
/// own `[0]`.
fn assert_express_effective_machine(view: &EffectiveView, own_guard: bool) {
    let express = view
        .state_machine(&package_key("ExpressLifecycle"))
        .expect("ExpressLifecycle is a state machine of the package");
    assert_eq!(
        express
            .states
            .iter()
            .map(|state| entry(state.declaring, state.key, state.position))
            .collect::<Vec<_>>(),
        expected_entries(&[
            ("ExpressLifecycle", "ExpressLifecycle/Expedited", 0),
            ("OrderLifecycle", "OrderLifecycle/Draft", 0),
            ("OrderLifecycle", "OrderLifecycle/Placed", 1),
            ("OrderLifecycle", "OrderLifecycle/Cancelled", 2),
        ]),
    );
    assert_eq!(
        express
            .states
            .iter()
            .map(|state| state.name)
            .collect::<Vec<_>>(),
        ["Expedited", "Draft", "Placed", "Cancelled"]
    );
    assert_eq!(
        express
            .transitions
            .iter()
            .map(|transition| entry(transition.declaring, transition.key, transition.position))
            .collect::<Vec<_>>(),
        expected_entries(&[
            ("ExpressLifecycle", "ExpressLifecycle/transition0", 0),
            ("OrderLifecycle", "OrderLifecycle/transition0", 0),
            ("OrderLifecycle", "OrderLifecycle/transition1", 1),
            ("OrderLifecycle", "OrderLifecycle/transition2", 2),
        ]),
    );
    let own = &express.transitions[0];
    assert_eq!(own.from.node, id("OrderLifecycle/Placed"));
    assert_eq!(own.to.node, id("ExpressLifecycle/Expedited"));
    assert_eq!(own.trigger.node, id("ExpressLifecycle/expedite"));
    assert_eq!(own.guard, own_guard.then_some("NotYetPlaced"));
    assert!(own.emits.is_empty());
    let inherited = &express.transitions[1];
    assert_eq!(inherited.from.node, id("OrderLifecycle/Draft"));
    assert_eq!(inherited.to.node, id("OrderLifecycle/Placed"));
    assert_eq!(inherited.trigger.node, id("OrderLifecycle/place"));
    assert_eq!(inherited.guard, Some("NotYetPlaced"));
    assert_eq!(
        inherited
            .emits
            .iter()
            .map(|key| &key.node)
            .collect::<Vec<_>>(),
        [&id("OrderPlaced")]
    );
    assert!(view.conforms(
        &package_key("ExpressLifecycle"),
        &package_key("OrderLifecycle")
    ));
}

pub(super) fn assert_derivation_conflict(
    refusal: &ModelRefusal,
    type_: &str,
    member: &str,
    redefiner: &str,
) {
    assert_eq!(refusal.code, Code::InvalidModelBinding);
    let ModelRefusalCause::DerivationConflict {
        type_: actual_type,
        member: actual_member,
        redefiners,
        ..
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
/// rejected at their authored declaration paths. Runs 1 and 7 and the run 2
/// and 4 refusals (a refused package has no view) are TC-1049 steps 1, 5 and 6.
#[trace(
    "QSpec-TC-235",
    "TC-1049",
    "FR-081-AC-17",
    "FR-081-AC-19",
    "FR-081-AC-21"
)]
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
    let view = record_charges(&package, 23);
    let express = view.type_identities()[&DeclarationKey {
        package: PACKAGE.to_owned(),
        node: id("ExpressLifecycle"),
    }];
    let mut effective: Vec<_> = view
        .declarations()
        .iter()
        .filter(|entry| entry.visible && entry.preimage.owner_effective_type == Some(express))
        .map(|entry| entry.preimage.original.node.clone())
        .collect();
    effective.sort();
    // The declarations carry operations, fields and clauses; states and
    // transitions are read through `state_machine`.
    let mut expected: Vec<_> = [
        // Operations place, cancel, expedite; field placed; the guard clause.
        "OrderLifecycle/place",
        "OrderLifecycle/cancel",
        "ExpressLifecycle/expedite",
        "OrderLifecycle/placed",
        "OrderLifecycle/NotYetPlaced",
    ]
    .into_iter()
    .map(id)
    .collect();
    expected.sort();
    assert_eq!(effective, expected);
    assert_express_effective_machine(&view, false);

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
    assert_express_effective_machine(&view, true);

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
    assert_derivation_paths(
        &refusals[0],
        &[
            ("ExpressLifecycle", "TypeDefinition.states[1]"),
            ("OrderLifecycle", "TypeDefinition.states[1]"),
        ],
    );

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
    assert_derivation_paths(
        &refusals[0],
        &[
            ("ExpressLifecycle", "TypeDefinition.transitions[1]"),
            ("OrderLifecycle", "TypeDefinition.transitions[0]"),
        ],
    );

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
#[trace("QSpec-TC-235", "QSpec-FR-208-AC-12")]
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

/// FR-208: a trigger or guard naming an operation or clause that only an
/// object type declares names nothing of any state machine, so it is a
/// missing name, not a phase-3 malformed declaration.
#[trace("QSpec-TC-235")]
#[test]
fn tc235_a_reference_to_an_object_types_member_is_a_missing_name() {
    for member in ["trigger", "guard"] {
        let document = mutate_k2(|document| {
            let order = type_named_mut(document, "Order");
            for (collection, entry) in [
                ("clauses", clause("Order", "OnlyOnOrder", "true")),
                ("operations", operation("Order", "only_on_order")),
            ] {
                order
                    .as_object_mut()
                    .unwrap()
                    .entry(collection)
                    .or_insert_with(|| json!([]))
                    .as_array_mut()
                    .unwrap()
                    .push(entry);
            }
            let transition = &mut type_named_mut(document, "OrderLifecycle")["transitions"][0];
            match member {
                "trigger" => transition["trigger"] = json!(id("Order/only_on_order")),
                _ => transition["guard"] = json!("OnlyOnOrder"),
            }
        });
        let refusals = normalized_refusals(&document, 23);
        assert_eq!(refusals.len(), 1, "{member}: {refusals:?}");
        assert_eq!(refusals[0].code, Code::MissingDeclaration, "{member}");
        assert_eq!(refusals[0].cause.as_str(), "missing-name", "{member}");
    }
}

/// M19 runs 2-3: the derivation conflict carries both sides' member paths as
/// typed values, for a repeated state and for a repeated transition.
#[test]
fn tc235_m19_derivation_conflict_member_paths() {
    let state = mutate_k2(|document| {
        document["types"]
            .as_array_mut()
            .unwrap()
            .push(express_lifecycle(
                "OrderLifecycle",
                &["Expedited", "Placed"],
                "Placed",
            ));
    });
    let refusals = normalized_refusals(&state, 23);
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert_derivation_paths(
        &refusals[0],
        &[
            ("ExpressLifecycle", "TypeDefinition.states[1]"),
            ("OrderLifecycle", "TypeDefinition.states[1]"),
        ],
    );

    let transition = mutate_k2(|document| {
        let mut declaration =
            express_lifecycle("OrderLifecycle", &["Expedited"], "OrderLifecycle/Placed");
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
    let refusals = normalized_refusals(&transition, 23);
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert_derivation_paths(
        &refusals[0],
        &[
            ("ExpressLifecycle", "TypeDefinition.transitions[1]"),
            ("OrderLifecycle", "TypeDefinition.transitions[0]"),
        ],
    );
}

/// A state machine cycle is a `specialization-cycle`, at each machine on it.
#[trace("TC-1020", "FR-082-AC-15")]
#[test]
fn a_state_machine_cycle_is_a_specialization_cycle() {
    let document = mutate_k2(|document| {
        type_named_mut(document, "OrderLifecycle")["supertypes"] = json!([id("ExpressLifecycle")]);
        document["types"]
            .as_array_mut()
            .unwrap()
            .push(express_lifecycle(
                "OrderLifecycle",
                &["Expedited"],
                "OrderLifecycle/Placed",
            ));
    });
    let cycle = ["ExpressLifecycle", "OrderLifecycle"];
    assert_specialization_cycles(
        &document,
        &[("ExpressLifecycle", &cycle), ("OrderLifecycle", &cycle)],
    );
}

/// A clause named like an operation of its owner refuses `conflicting-binding`
/// (QSpec TC-235 / FR-154-AC-12, cited by id). Typed code, cause and member
/// path only.
#[trace("QSpec-TC-235", "QSpec-FR-154-AC-12")]
#[test]
fn tc235_m32_run2_clause_named_like_an_operation_is_conflicting_binding() {
    let document = mutate_k2(|document| {
        type_named_mut(document, "OrderLifecycle")["clauses"]
            .as_array_mut()
            .unwrap()
            .push(clause(
                "OrderLifecycle",
                "cancel",
                "self.placed or not self.placed",
            ));
    });
    let refusals = normalized_refusals(&document, 22);
    assert_eq!(refusals.len(), 1, "one refusal: {refusals:?}");
    assert_eq!(refusals[0].code, Code::InvalidModelBinding);
    assert_eq!(refusals[0].cause.as_str(), "conflicting-binding");
    let ModelRefusalCause::ConflictingBinding { member: at, .. } = &refusals[0].cause else {
        panic!("expected conflicting-binding, got {:?}", refusals[0]);
    };
    assert_eq!(at.as_ref(), Some(&member("TypeDefinition.clauses[1]")));
}

/// `ExpressLifecycleCopy`: a full machine sharing `ExpressLifecycle`'s display
/// name but not its key, with no supertypes.
fn express_lifecycle_copy() -> Value {
    let mut copy = declaration(
        "ExpressLifecycleCopy",
        "lifecycle",
        json!({
            "operations": [operation("ExpressLifecycleCopy", "expedite")],
            "states": [lifecycle_state("ExpressLifecycleCopy", "Expedited")],
            "transitions": [lifecycle_transition(
                "ExpressLifecycleCopy", "transition0", "Expedited", "Expedited", "expedite", vec![],
            )],
        }),
    );
    copy["displayName"] = json!("ExpressLifecycle");
    copy
}

/// TC-1049 steps 2 to 4: the view answers `state_machine` for machines only,
/// answers `conforms` by key and not by display name, and exposes neither as
/// a declaration or identity change.
#[trace("TC-1049", "FR-081-AC-18", "FR-081-AC-20")]
#[test]
fn tc1049_view_exposes_machine_scope_and_conformance_by_key() {
    let document = mutate_k2(|document| {
        let types = document["types"].as_array_mut().unwrap();
        types.push(express_lifecycle(
            "OrderLifecycle",
            &["Expedited"],
            "OrderLifecycle/Placed",
        ));
        types.push(express_lifecycle_copy());
    });
    let package = read_k2(&document).expect("the copy is an admissible machine");
    let view = record_charges(&package, 25);
    let order = package_key("OrderLifecycle");
    let express = package_key("ExpressLifecycle");
    let copy = package_key("ExpressLifecycleCopy");

    // A second normalization of the package, which no accessor has read: its
    // identities are the baseline for the read view.
    let without_calls = {
        let (outcome, _) = normalize_with_meter(&package, ModelNormalizationLimits::default());
        let NormalizeOutcome::Completed(fresh) = outcome else {
            panic!("normalization must complete");
        };
        fresh
    };

    let own = view
        .state_machine(&order)
        .expect("OrderLifecycle is a machine");
    assert_eq!(
        own.states
            .iter()
            .map(|state| state.name)
            .collect::<Vec<_>>(),
        ["Draft", "Placed", "Cancelled"]
    );
    assert_eq!(
        own.transitions
            .iter()
            .map(|transition| entry(transition.declaring, transition.key, transition.position))
            .collect::<Vec<_>>(),
        expected_entries(&[
            ("OrderLifecycle", "OrderLifecycle/transition0", 0),
            ("OrderLifecycle", "OrderLifecycle/transition1", 1),
            ("OrderLifecycle", "OrderLifecycle/transition2", 2),
        ]),
    );
    assert_eq!(own.transitions[0].guard, Some("NotYetPlaced"));
    assert_eq!(
        own.transitions[0]
            .emits
            .iter()
            .map(|key| &key.node)
            .collect::<Vec<_>>(),
        [&id("OrderPlaced")]
    );

    // Step 2: an object type and an absent key are no machine.
    assert_eq!(view.state_machine(&package_key("Order")), None);
    assert_eq!(view.state_machine(&package_key("Absent")), None);

    // Step 3.
    assert!(view.conforms(&express, &order));
    assert!(view.conforms(&express, &express));
    assert!(!view.conforms(&order, &express));
    assert!(!view.conforms(&express, &package_key("Fulfilment")));
    assert!(!view.conforms(&copy, &express));
    assert!(!view.conforms(&copy, &order));
    assert!(!view.conforms(&express, &copy));

    // Step 4: the accessors changed nothing a caller can observe.
    assert_eq!(view.identity(), without_calls.identity());
    assert_eq!(view.declarations(), without_calls.declarations());
    assert_eq!(view.type_identities(), without_calls.type_identities());
    assert!(view.declarations().iter().all(|entry| {
        let node = &entry.preimage.original.node;
        !node.contains("/transition") && !node.ends_with("/Draft") && !node.ends_with("/Expedited")
    }));
}

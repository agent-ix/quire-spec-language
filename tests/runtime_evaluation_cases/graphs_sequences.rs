// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-067/068/070: independent finite graph and ordered sequence expectations.

use super::*;

#[test]
#[trace("TC-067", "FR-008-AC-1", "FR-008-AC-2")]
fn parent_order_and_aggregate_truth_follow_actual_populations() {
    let models = [native_rule_model::parts().model()];
    let parent = checked(
        &models,
        "present(self.parent) implies deref(value(self.parent)).n < self.n",
    );
    for (parent_n, truth) in [(0, true), (1, false), (2, false)] {
        let mut data = draft(&models[0]);
        let mut other = data.populations[0].objects[0].clone();
        other.key = "parent".into();
        let n = append(&mut data, ValueNode::Integer { value: parent_n });
        other
            .fields
            .iter_mut()
            .find(|f| f.name.as_str() == "n")
            .unwrap()
            .value = n;
        let reference = append(
            &mut data,
            ValueNode::Reference {
                identity: object(&models[0], "parent"),
            },
        );
        change_field(&mut data, "parent", ValueNode::Present { value: reference });
        data.populations[0].objects.push(other);
        let context = current(&parent, &models[0], data);
        let report = evaluate(&context, EvaluationLimits::default(), || false);
        assert_eq!(report.outcome(), &EvaluationOutcome::Completed(truth));
        assert_eq!(
            report.context().clause().binding().requirement,
            authored_owner()
        );
        assert_eq!(
            report
                .context()
                .checked()
                .linked()
                .unit()
                .source()
                .identity()
                .revision,
            "7"
        );
    }
    let aggregate = checked(&models, "forall(item in self.items: item <= self.n)");
    for (items, truth) in [(vec![], true), (vec![1, 1], true), (vec![0, 2, 0], false)] {
        let mut data = draft(&models[0]);
        let values = items
            .into_iter()
            .map(|value| append(&mut data, ValueNode::Integer { value }))
            .collect();
        change_field(&mut data, "items", ValueNode::Sequence { values });
        let context = current(&aggregate, &models[0], data);
        assert_eq!(
            evaluate(&context, EvaluationLimits::default(), || false).outcome(),
            &EvaluationOutcome::Completed(truth)
        );
    }
}

#[test]
#[trace("TC-068", "FR-008-AC-3", "FR-008-AC-11")]
fn every_small_functional_graph_matches_independent_matrix_closure() {
    let models = [native_rule_model::parts().model()];
    let clauses = [
        checked(&models, "reaches(self, other, parent)"),
        checked(&models, "reaches(self.peer, other, parent)"),
    ];
    let mut comparisons = 0;
    for n in 1_usize..=3 {
        for encoding in 0..(n + 1).pow(u32::try_from(n).unwrap()) {
            let mut digits = encoding;
            let edges: Vec<_> = (0..n)
                .map(|_| {
                    let digit = digits % (n + 1);
                    digits /= n + 1;
                    digit.checked_sub(1)
                })
                .collect();
            // Independent Boolean adjacency-matrix closure, without the evaluator's traversal/visited algorithm.
            let mut closure = vec![vec![false; n]; n];
            for (from, edge) in edges.iter().enumerate() {
                if let Some(to) = edge {
                    closure[from][*to] = true;
                }
            }
            for via in 0..n {
                for from in 0..n {
                    for to in 0..n {
                        closure[from][to] |= closure[from][via] && closure[via][to];
                    }
                }
            }
            for renamed in [false, true] {
                let keys: Vec<_> = (0..n)
                    .map(|i| {
                        if renamed {
                            format!("界-{}", n - i)
                        } else {
                            format!("node-{i}")
                        }
                    })
                    .collect();
                let mut population = draft(&models[0]);
                let template = population.populations[0].objects[0].clone();
                population.populations[0].objects.clear();
                for (index, edge) in edges.iter().enumerate() {
                    let mut node = template.clone();
                    node.key.clone_from(&keys[index]);
                    let own_ref = append(
                        &mut population,
                        ValueNode::Reference {
                            identity: object(&models[0], &keys[index]),
                        },
                    );
                    node.fields
                        .iter_mut()
                        .find(|f| f.name.as_str() == "peer")
                        .unwrap()
                        .value = own_ref;
                    if let Some(to) = edge {
                        let reference = append(
                            &mut population,
                            ValueNode::Reference {
                                identity: object(&models[0], &keys[*to]),
                            },
                        );
                        let parent =
                            append(&mut population, ValueNode::Present { value: reference });
                        node.fields
                            .iter_mut()
                            .find(|f| f.name.as_str() == "parent")
                            .unwrap()
                            .value = parent;
                    }
                    population.populations[0].objects.push(node);
                }
                for (from, expected) in closure.iter().enumerate() {
                    for (to, truth) in expected.iter().enumerate() {
                        let mut data = population.clone();
                        let target = append(
                            &mut data,
                            ValueNode::Object {
                                identity: object(&models[0], &keys[to]),
                            },
                        );
                        data.values.push(ValueBinding {
                            declaration: qualified(&models[0], "other"),
                            value: target,
                        });
                        for checked in &clauses {
                            let artifact = snapshot(data.clone());
                            let mut selected = selection(&models[0], artifact.reference());
                            let quire_spec_language::runtime::ObservationSelection::Current {
                                self_object,
                                ..
                            } = &mut selected.observation
                            else {
                                unreachable!()
                            };
                            *self_object = object(&models[0], &keys[from]);
                            let context = validate(
                                checked,
                                input(artifact),
                                selected,
                                ValidationLimits::default(),
                                || false,
                            )
                            .unwrap();
                            let result = evaluate(&context, EvaluationLimits::default(), || false);
                            assert_eq!(result.outcome(), &EvaluationOutcome::Completed(*truth), "n={n}, encoding={encoding}, from={from}, to={to}, renamed={renamed}");
                            comparisons += 1;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(comparisons, 2456);
}

#[test]
#[trace("TC-070", "FR-008-AC-13", "FR-008-AC-17")]
fn all_short_sequences_preserve_occurrences_order_and_short_circuit_events() {
    let models = [native_rule_model::parts().model()];
    let universal = checked(
        &models,
        "forall(item in self.items: item = self.n implies false)",
    );
    let existential = checked(
        &models,
        "exists(item in self.items: item = self.n implies false)",
    );
    let size = checked(&models, "size(self.items) = self.count");
    let mut sequences = 0;
    for length in 0..=3 {
        for encoding in 0..3_usize.pow(length) {
            let mut digits = encoding;
            let items: Vec<_> = (0..length)
                .map(|_| {
                    let value = digits % 3;
                    digits /= 3;
                    i64::try_from(value).unwrap()
                })
                .collect();
            let mut data = draft(&models[0]);
            let values = items
                .iter()
                .map(|value| append(&mut data, ValueNode::Integer { value: *value }))
                .collect();
            change_field(&mut data, "items", ValueNode::Sequence { values });
            change_field(
                &mut data,
                "count",
                ValueNode::Integer {
                    value: i64::from(length),
                },
            );
            let context = current(&size, &models[0], data.clone());
            assert_eq!(
                evaluate(&context, EvaluationLimits::default(), || false).outcome(),
                &EvaluationOutcome::Completed(true)
            );
            for (checked, all) in [(&universal, true), (&existential, false)] {
                let mut expected_events = Vec::new();
                let mut truth = all;
                for item in &items {
                    let antecedent = *item == 1;
                    expected_events.extend([
                        ImplicationEventKind::AntecedentEntered,
                        ImplicationEventKind::AntecedentCompleted(antecedent),
                    ]);
                    if antecedent {
                        expected_events.push(ImplicationEventKind::ConsequentEntered);
                    }
                    let predicate = !antecedent;
                    if predicate != all {
                        truth = predicate;
                        break;
                    }
                }
                let context = current(checked, &models[0], data.clone());
                let result = evaluate(&context, EvaluationLimits::default(), || false);
                assert_eq!(
                    result.outcome(),
                    &EvaluationOutcome::Completed(truth),
                    "{items:?}, forall={all}"
                );
                assert_eq!(
                    result.events().iter().map(|e| e.kind).collect::<Vec<_>>(),
                    expected_events,
                    "{items:?}, forall={all}"
                );
            }
            sequences += 1;
        }
    }
    assert_eq!(sequences, 40);
}

#[test]
#[trace("TC-070", "FR-008-AC-13")]
fn separately_admitted_reference_sequence_keeps_duplicate_targets_in_order() {
    let models = [authored_model(|data| {
        data["records"][0]["fields"][9]["type"]["value"] =
            serde_json::json!({"kind":"record", "name":"NodeRef"});
    })];
    let checked = checked(
        &models,
        "forall(p in self.items: deref(p).n = self.n implies true)",
    );
    let mut data = draft(&models[0]);
    let mut other = data.populations[0].objects[0].clone();
    other.key = "other".into();
    let n = append(&mut data, ValueNode::Integer { value: 2 });
    other
        .fields
        .iter_mut()
        .find(|f| f.name.as_str() == "n")
        .unwrap()
        .value = n;
    data.populations[0].objects.push(other);
    let reference = append(
        &mut data,
        ValueNode::Reference {
            identity: object(&models[0], "other"),
        },
    );
    change_field(
        &mut data,
        "items",
        ValueNode::Sequence {
            values: vec![ValueId::new(3), ValueId::new(3), reference],
        },
    );
    let context = current(&checked, &models[0], data);
    let report = evaluate(&context, EvaluationLimits::default(), || false);
    assert_eq!(report.outcome(), &EvaluationOutcome::Completed(true));
    let antecedents: Vec<_> = report
        .events()
        .iter()
        .filter_map(|event| match event.kind {
            ImplicationEventKind::AntecedentCompleted(truth) => Some(truth),
            _ => None,
        })
        .collect();
    assert_eq!(antecedents, [true, true, false]);
    assert_eq!(report.events().len(), 8);
}

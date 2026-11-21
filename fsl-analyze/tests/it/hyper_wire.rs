// SPDX-License-Identifier: AGPL-3.0-or-later
//! Hyperproperty checks through the command adapter and its wire document.

use ix_trace_rs::trace;
use serde_json::json;

use crate::analyze_cli_support::{analyze, label, request_over, Workspace};
use crate::hyper_world::{
    non_interference, opaque, request_files_over_starts, Vault, EQUAL_INPUTS, STARTS,
};

#[trace("TC-1591", "FR-631-AC-1", "FR-181-AC-1", "FR-163-AC-1")]
#[test]
fn analyze_hyper_single_existential_holds_and_refutes() {
    for (body, expected, exit) in [
        ("eventually always holds(v.l @ b = 1)", "proved", 0),
        ("eventually holds(v.l @ b != v.l @ b)", "refuted", 10),
    ] {
        let clause = format!("hyper Possible using inf over behaviours (v: V::Vault) exists trace b of V {{ {body} }}");
        let (unit, model, snapshots) =
            request_files_over_starts(Vault::Secure, false, &STARTS, &clause);
        let ws = Workspace::new(&format!("hyper-wire-single-{expected}"));
        let run = analyze(&request_over(
            &ws,
            &unit,
            &model,
            &snapshots,
            json!({"items": [{"claim": "Possible", "temporal": {"witness_samples": 0}}]}),
        ));
        assert_eq!(run.exit, exit, "{}", run.document());
        assert_eq!(label(&run.item(0)), expected);
        if exit == 0 {
            assert_eq!(run.item(0)["record"]["certification"], "certified");
            assert!(
                run.item(0)["record"]["evidence"]["paths"][0]["loop_entry"].is_string(),
                "{}",
                run.document()
            );
        } else {
            assert_eq!(run.item(0)["counterexample"]["arm"], "trap");
            assert_eq!(run.item(0)["replay"]["reproduced"], true);
        }
    }
}

#[trace("TC-2957", "FR-1000-AC-8")]
#[test]
fn analyze_hyper_hf_binds_a_fixed_reference_across_copies() {
    let body = "hyper HF using t0 over behaviours (c: M::Counter) forall trace a of M forall trace b of M { always holds(c.value @ a <= 3 or c.value @ b <= 3 or c = M::Counter#\"a\") }";
    let ws = Workspace::new("hyper-wire-fixed-reference");
    let run = analyze(&crate::analyze_cli_support::counter_request(
        &ws,
        body,
        json!([{"claim": "HF"}]),
    ));
    assert_eq!(run.exit, 0, "{}", run.document());
    assert_eq!(label(&run.item(0)), "proved");
}

#[trace("TC-1591", "FR-631-AC-1", "FR-631-AC-2", "FR-163-AC-1")]
#[test]
fn analyze_hyper_holds_with_a_checked_product_certificate() {
    let (unit, model, snapshots) = request_files_over_starts(
        Vault::Secure,
        false,
        &STARTS,
        &non_interference(EQUAL_INPUTS),
    );
    let ws = Workspace::new("hyper-wire-holds");
    let run = analyze(&request_over(
        &ws,
        &unit,
        &model,
        &snapshots,
        json!({"items": [{"claim": "NonInterference"}]}),
    ));
    assert_eq!(run.exit, 0, "{}", run.document());
    assert_eq!(label(&run.item(0)), "proved");
    assert_eq!(run.item(0)["record"]["basis"]["type"], "reduced");
    assert_eq!(run.item(0)["record"]["certification"], "certified");
    assert_eq!(run.item(0)["certificate"]["accepted"], true);
    assert!(run.item(0)["counterexample"].is_null());
}

/// A package with no hyper evaluator form refuses before running any item,
/// even while the source route can decide that same claim.
#[trace("TC-2958", "FR-940")]
#[test]
fn a_package_hyper_item_refuses_without_falling_back_to_source() {
    use crate::analyze_cli_support::{
        counter_request, package_selection, request_members, HANDOFF,
    };
    let body = "hyper Equal using t0 over behaviours (c: M::Counter) \
        forall trace a of M forall trace b of M { always holds(c.value @ a <= 3 and c.value @ b <= 3) }";
    let ws = Workspace::new("hyper-wire-package-pending");
    let items = json!([{"claim": "Equal"}]);
    let source = analyze(&counter_request(&ws, body, items.clone()));
    assert_eq!(source.exit, 0, "{}", source.document());
    assert_eq!(label(&source.item(0)), "proved");
    let (members, _) = package_selection(&ws, &request_members(Some(items)));
    // Neither the original source nor the handoff's retained source can be
    // recompiled to conceal the package evaluator's missing form.
    ws.write("program.fsl", b"not a native unit");
    std::fs::remove_dir_all(ws.path(&format!("{HANDOFF}/sources")))
        .expect("remove only the fixture's retained source files");
    let path = ws.write(
        "analyze-package.json",
        json!({"format": "native-analyze/1", "request": members})
            .to_string()
            .as_bytes(),
    );
    let package = analyze(&path);
    let document = package.document();
    assert_eq!(
        document["diagnostics"][0]["code"], "unsupported_construct",
        "{document}"
    );
    assert_eq!(
        document["diagnostics"][0]["cause"], "not-yet-implemented",
        "{document}"
    );
    assert_eq!(document["items"], json!([]));
    assert_ne!(package.exit, 0);
}

#[trace("TC-1591", "FR-631-AC-1", "FR-177-AC-1", "FR-163-AC-1")]
#[test]
fn analyze_hyper_forall_exists_holds_and_refutes() {
    for (vault, expected, exit) in [(Vault::Secure, "proved", 0), (Vault::Leaky, "refuted", 10)] {
        let (unit, model, snapshots) =
            request_files_over_starts(vault, false, &STARTS, &opaque(""));
        let ws = Workspace::new(&format!("hyper-wire-forall-exists-{expected}"));
        let run = analyze(&request_over(
            &ws,
            &unit,
            &model,
            &snapshots,
            json!({"items": [{"claim": "Opaque"}]}),
        ));
        assert_eq!(run.exit, exit, "{}", run.document());
        assert_eq!(label(&run.item(0)), expected);
        if exit == 10 {
            assert_eq!(run.item(0)["counterexample"]["kind"], "witness-exhausted");
            assert_eq!(run.item(0)["replay"]["reproduced"], true);
        } else {
            assert_eq!(run.item(0)["certificate"]["accepted"], true);
        }
    }
}

#[trace("TC-1591", "FR-631-AC-1", "FR-178-AC-1", "FR-163-AC-1")]
#[test]
fn analyze_hyper_projected_holds_and_refutes() {
    for (vault, expected, exit) in [
        (Vault::Busy, "proved", 0),
        (Vault::BusyLeaky, "refuted", 10),
    ] {
        let (unit, model, snapshots) =
            request_files_over_starts(vault, false, &STARTS, &crate::hyper_projected::aligned(&[]));
        let ws = Workspace::new(&format!("hyper-wire-projected-{expected}"));
        let run = analyze(&request_over(
            &ws,
            &unit,
            &model,
            &snapshots,
            json!({"items": [{"claim": "NonInterference"}]}),
        ));
        assert_eq!(run.exit, exit, "{}", run.document());
        assert_eq!(label(&run.item(0)), expected);
        if exit == 10 {
            assert_eq!(run.item(0)["counterexample"]["kind"], "projected");
            assert_eq!(run.item(0)["replay"]["reproduced"], true);
        } else {
            assert_eq!(run.item(0)["certificate"]["accepted"], true);
        }
    }
}

#[trace("TC-1591", "FR-631-AC-1", "FR-183-AC-1")]
#[test]
fn analyze_hyper_refuted_with_pair() {
    let (unit, model, snapshots) = request_files_over_starts(
        Vault::Leaky,
        false,
        &STARTS,
        &non_interference(EQUAL_INPUTS),
    );
    let ws = Workspace::new("hyper-wire-refuted");
    let run = analyze(&request_over(
        &ws,
        &unit,
        &model,
        &snapshots,
        json!({"items": [{"claim": "NonInterference"}]}),
    ));
    assert_eq!(run.exit, 10, "{}", run.document());
    let item = run.item(0);
    assert_eq!(label(&item), "refuted");
    let found = &item["counterexample"];
    assert_eq!(found["type"], "hyper-counterexample");
    assert_eq!(found["kind"], "lockstep");
    assert_eq!(found["traces"].as_array().map(Vec::len), Some(2));
    assert_eq!(found["traces"][0]["variable"], "a");
    assert_eq!(found["traces"][1]["variable"], "b");
    assert_eq!(item["replay"]["reproduced"], true);
}

#[trace("TC-1591", "FR-631-AC-3", "FR-184-AC-2")]
#[test]
fn analyze_hyper_witness_budget_is_incomplete() {
    let declaration = "hyper Opaque using inf over behaviours (v: V::Vault) \
        forall trace a of V exists trace b of V { always holds(v.l @ a = v.l @ b) }";
    let (unit, model, snapshots) =
        request_files_over_starts(Vault::Secure, false, &STARTS, declaration);
    let ws = Workspace::new("hyper-wire-witness-budget");
    let run = analyze(&request_over(
        &ws,
        &unit,
        &model,
        &snapshots,
        json!({"items": [{"claim": "Opaque", "temporal": {"limits": {"max_witness_set": 1}}}]}),
    ));
    assert_eq!(label(&run.item(0)), "incomplete", "{}", run.document());
    assert_eq!(run.item(0)["record"]["cause"]["limit"], "max_witness_set");
    assert_eq!(run.exit, 22);
}

/// A clause outside every admitted form (HP-4) is a typed refusal through the
/// command, never a verdict: the existential copy precedes the universal one,
/// so no engine owns it.
#[trace("TC-1591", "FR-631-AC-1", "FR-173-AC-2")]
#[test]
fn analyze_hyper_outside_the_admitted_forms_is_a_typed_refusal() {
    let declaration = "hyper Unowned using inf over behaviours (v: V::Vault) \
        exists trace a of V forall trace b of V { always holds(v.l @ a = v.l @ b) }";
    let (unit, model, snapshots) =
        request_files_over_starts(Vault::Secure, false, &STARTS, declaration);
    let ws = Workspace::new("hyper-wire-unowned");
    let run = analyze(&request_over(
        &ws,
        &unit,
        &model,
        &snapshots,
        json!({"items": [{"claim": "Unowned"}]}),
    ));
    assert_ne!(run.exit, 0, "{}", run.document());
    assert_eq!(label(&run.item(0)), "unsupported", "{}", run.document());
    assert_eq!(
        run.item(0)["record"]["diagnostic"]["code"],
        "unsupported_construct"
    );
    assert!(run.item(0)["counterexample"].is_null(), "{}", run.document());
}

/// FR-179: a step relation over reachable transitions has routing and an SMT
/// encoding but no explicit-state engine yet, so `fsl analyze` cannot return
/// holds or refuted for it.
#[trace("TC-1591", "FR-179")]
#[test]
#[ignore = "blocked on QSL-2238: the explicit-state step-relation engine does not exist; this ticket does not build it"]
fn hyper_step_relation_through_analyze() {
    let relation = "relation Det using v over (x: V::Vault::step, y: V::Vault::step) \
        { x.i = y.i and pre(x.self.h) = pre(y.self.h) implies x.self.l = y.self.l }";
    let (unit, model, snapshots) =
        request_files_over_starts(Vault::Secure, false, &STARTS, relation);
    let ws = Workspace::new("hyper-wire-step-relation");
    let run = analyze(&request_over(
        &ws,
        &unit,
        &model,
        &snapshots,
        json!({"items": [{"claim": "Det"}]}),
    ));
    assert_eq!(label(&run.item(0)), "proved", "{}", run.document());
    assert_eq!(run.exit, 0, "{}", run.document());
}

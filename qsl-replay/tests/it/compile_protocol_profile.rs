// SPDX-License-Identifier: AGPL-3.0-or-later
//! The public compile path selects the `protocol_profile` law from the unit's
//! own header, resolved in the protocol clause family's catalog (FR-110,
//! ADR-011 §2.4). As an integration test it reaches only the public API.

use ix_trace_rs::trace;
use qsl_foundation::Setting;
use qsl_replay::{
    compile_package, Code, DependencyInput, ReplayLimits, ReplayRefusal, SourceIdentity,
    StageLimits,
};
use serde_json::{json, Value};

const MODEL: &[u8] = include_bytes!("../../../tests/fixtures/spine-model.semantic-ir.json");
const FIXTURE: &str = include_str!("../../../tests/fixtures/spine-model.native");
const PROFILE: &str = "quire.protocol.finite-global/v1";

const PROTOCOL: &str = "protocol Fill using v over (input: Boolean) on origin {\n\
    role R on M::Gadget;\n\
    run sequence Main { }\n\
    finish End as (outcome: Boolean) { true };\n\
    }\n";
const REFERENCE: &str = "function refers using v(): Boolean pure { protocol Fill }\n";

/// The spine-model unit with `profile` selected after its `v` profile (or
/// none), then `declarations`.
fn unit(profile: Option<&str>, declarations: &str) -> String {
    let header = profile.map_or_else(String::new, |identity| {
        format!("profile p = \"{identity}\";\n")
    });
    let fixture = FIXTURE.replace(
        "profile v = \"quire.model.complete/v1\";\n",
        &format!("profile v = \"quire.model.complete/v1\";\n{header}"),
    );
    format!("{fixture}{declarations}")
}

/// `text` through the public compile path: its package wire, or the refusal.
fn compile(text: &str) -> Result<Value, ReplayRefusal> {
    let stages: StageLimits = [(Setting::S1InputBytes, 1 << 20)].into_iter().collect();
    let compiled = compile_package(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        [MODEL],
        &DependencyInput::default(),
        stages,
        ReplayLimits::default(),
    )?;
    Ok(serde_json::from_slice(compiled.bytes()).expect("the package bytes are JSON"))
}

fn refusal_code(result: Result<Value, ReplayRefusal>) -> Code {
    match result {
        Err(ReplayRefusal::Recompile(refusal)) => refusal.code(),
        other => panic!("expected a recompile refusal, got {other:?}"),
    }
}

/// Source to IR: a unit that selects the protocol profile and refers to a
/// protocol compiles, and its lock selects exactly the `protocol_profile`
/// the header named.
#[trace("FR-092-AC-18", "TC-1021")]
#[test]
fn a_selected_protocol_profile_reaches_the_emitted_lock() {
    let wire = compile(&unit(Some(PROFILE), &format!("{REFERENCE}{PROTOCOL}")))
        .expect("the unit compiles through the public path");
    assert_eq!(
        wire["lock"]["profile_selections"],
        json!([{
            "role": "protocol_profile",
            "definition": {"authority": "agent-ix", "identity": PROFILE},
        }])
    );
}

/// A header profile in no catalog refuses, and dropping the selection leaves the reference with
/// no `protocol_profile` law (no default is supplied).
#[trace("FR-092-AC-18", "TC-1021")]
#[test]
fn an_unknown_or_missing_profile_binding_refuses() {
    let declarations = format!("{REFERENCE}{PROTOCOL}");
    assert_eq!(
        refusal_code(compile(&unit(
            Some("quire.protocol.finite-global/v2"),
            &declarations
        ))),
        Code::UnknownProfile
    );
    assert_eq!(
        refusal_code(compile(&unit(None, &declarations))),
        Code::MissingDeclaration
    );
}

/// A unit that uses no protocol is unaffected by the protocol profile: its
/// lock selects none, whether or not the header names the profile.
#[trace("FR-092-AC-18", "TC-1021")]
#[test]
fn a_unit_without_a_protocol_reference_selects_no_protocol_profile() {
    for profile in [None, Some(PROFILE)] {
        let wire = compile(&unit(profile, "")).expect("the unit compiles");
        assert_eq!(wire["lock"]["profile_selections"], json!([]), "{profile:?}");
    }
}

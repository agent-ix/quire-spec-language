// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1088 (FR-394-AC-5): a consumer reaches `compile_admitted`, the lookups
//! and both refusal enums through the `qsl_replay` root alone. This file names
//! no `qsl_replay::spine` or `qsl_semantics` item, so it stops compiling if
//! the facade needs one.

use ix_trace_rs::trace;
use qsl_foundation::Setting;
use qsl_replay::{
    compile_admitted, AdmittedLookupRefusal, AdmittedRefusal, AdmittedResource, DependencyInput,
    Identifier, ReplayLimits, SourceIdentity, StageLimits, WireNodeId,
};

const SOURCE: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"quire.value.complete/v1\";\n\
     enum Color { BLUE, RED }\n";

fn limits(source_bytes: u64) -> StageLimits {
    [(Setting::S1InputBytes, source_bytes)]
        .into_iter()
        .collect()
}

fn admit(
    bytes: &[u8],
    stages: StageLimits,
) -> Result<qsl_replay::AdmittedCompilation, AdmittedRefusal> {
    compile_admitted(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        bytes,
        [],
        &DependencyInput::default(),
        stages,
        ReplayLimits::default(),
    )
}

/// The whole consumer path: compile, take the parts, look a declaration and a
/// case up by wire id, and read both refusal enums.
#[trace("TC-1088", "FR-394-AC-5")]
#[test]
fn a_consumer_looks_keys_up_through_the_crate_root() {
    let admitted = admit(SOURCE.as_bytes(), limits(1 << 20)).expect("the unit compiles");
    let (package, context) = admitted.into_parts();
    assert!(!package.bytes().is_empty());

    let color = context.enums().next().expect("an enum");
    let declaration = WireNodeId::from_digest(*color.key().as_bytes());
    let blue = Identifier::new("BLUE").expect("an identifier");
    assert_eq!(
        context.enum_declaration(declaration).map(|e| e.key()),
        Ok(color.key())
    );
    assert_eq!(
        context.enum_case(declaration, &blue).map(|c| c.key()),
        Ok(color.case(&blue).expect("BLUE").key())
    );

    let purple = Identifier::new("PURPLE").expect("an identifier");
    assert!(matches!(
        context.enum_case(declaration, &purple),
        Err(AdmittedLookupRefusal::Unknown { case: Some(_), .. })
    ));
    assert!(matches!(
        context.enum_declaration(WireNodeId::from_digest([7; 32])),
        Err(AdmittedLookupRefusal::Unknown { case: None, .. })
    ));

    assert!(matches!(
        admit(SOURCE.as_bytes(), limits(4)),
        Err(AdmittedRefusal::Resource(AdmittedResource::StageLimit(_)))
    ));
}

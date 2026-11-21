// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-2065 (FR-201-AC-51): `fsl-probability-check`, a separate process with
//! no `fsl-analyze` linkage, accepts every certificate kind EN-5 emits for
//! the exact classes FSpec FR-641 requires, and rejects each one altered.
//! The certificates are produced by the analyzer over the cyclic `link`
//! model, as the other suites of this binary produce theirs.

use super::bounded::{document, key, mutate, request_with, run};
use fsl_analyze::exact_probabilistic::ExactProbLimits;
use ix_trace_rs::trace;
use serde_json::{json, Value};

const SNAPSHOT: &[u8] = include_bytes!("../fixtures/link.snapshot.json");

fn domain() -> String {
    include_str!("../fixtures/reward.semantic-ir.json").replace(
        "\"typeRef\":\"ix://filament/native/Boolean\"}],\"post\"",
        "\"typeRef\":\"ix://test/link/Attempts\"}],\"post\"",
    )
}

/// The cyclic `link` model: `send_a` retries until delivered or two losses,
/// `send_b` and `send_c` are never enabled.
fn model() -> String {
    let language = include_str!("../fixtures/link.native")
        .lines()
        .next()
        .unwrap();
    format!(
        "{language}\n\
        profile v = \"filament.model.complete/v1\";\n\
        profile inf = \"filament.temporal.infinite-trace/v1\";\n\
        profile ev = \"filament.temporal.event-position.false-extension/v1\";\n\
        model M = \"test/link\";\n\
        pre A using v on M::Msg::send_a {{ self.attempts < 2 or self.delivered }}\n\
        post A_done using v on M::Msg::send_a {{ self.attempts - (if lost = 0 then 0 else 2) = 0 and self.delivered = (lost = 1) }}\n\
        pre B using v on M::Msg::send_b {{ false }}\n\
        post B_done using v on M::Msg::send_b {{ self.attempts = 0 and not self.delivered }}\n\
        pre C using v on M::Msg::send_c {{ false }}\n\
        post C_done using v on M::Msg::send_c {{ self.attempts = 0 and not self.delivered }}\n\
        random Msg::send_a(lost) ~ {{ 0: 1, 1: 1, 2: 1 }};\n\
        random Msg::send_b(lost) ~ {{ 0: 1, 1: 1, 2: 1 }};\n\
        random Msg::send_c(lost) ~ {{ 0: 1, 1: 1, 2: 1 }};\n\
        workload Even on M {{ weight Msg::send_a = 1; weight Msg::send_b = 1; weight Msg::send_c = 1; }}\n\
        terminal Stop using v on M when {{ self.attempts = 2 and not self.delivered }};\n"
    )
}

fn likely(profile: &str, scheduler: &str, form: &str) -> String {
    format!(
        "{}\nprobabilistic Likely using {profile} on M over (m: M::Msg) under {scheduler} on origin {{ {form} }}\n",
        model()
    )
}

/// The analyzer's retained certificate request for `source`, which the
/// decided behaviour requires to exist for every exact class.
fn certified(source: &str, limits: ExactProbLimits) -> Value {
    let (request, settlement) = request_with(source, SNAPSHOT, limits, domain().as_bytes());
    request.unwrap_or_else(|| {
        panic!("every exact proof retains an accepted certificate: {settlement:?}\n{source}")
    })
}

/// Accepted by the separate process with `kind`; with its first value
/// altered it is rejected.
fn accepted_then_rejected(request: &Value, kinds: &[&str]) {
    let (code, output) = run(request);
    assert_eq!(code, 0, "{output}");
    let doc = document(request);
    let kind = doc["kind"].as_str().expect("a certificate kind");
    assert!(kinds.contains(&kind), "{kind} not in {kinds:?}");
    // FR-196: every state is named by the complete eight-member key.
    for row in doc["values"].as_array().expect("values") {
        let members = key(&row["state"]);
        assert_eq!(members.as_array().map(Vec::len), Some(8), "{members}");
    }

    let altered = mutate(request, |doc| {
        doc["values"][0]["value"] = json!({"numerator": "1", "denominator": "7"});
    });
    let (code, output) = run(&altered);
    assert_ne!(code, 0, "an altered certificate is rejected: {output}");
}

/// Unbounded fair eventual reachability over every scheduler with a
/// fairness set: an `exact` certificate the checker accepts.
#[trace("TC-2065", "FR-201-AC-51")]
#[test]
#[ignore = "QSL-2158: FR-1041 (unbounded fair eventual) not implemented yet"]
fn an_unbounded_fair_eventual_certificate_is_accepted() {
    let source = likely(
        "inf",
        "every scheduler fair strong Msg::send_a",
        "probability >= rational(1,2) [ eventually holds(m.delivered) ]",
    );
    accepted_then_rejected(&certified(&source, ExactProbLimits::default()), &["exact"]);
}

/// A value-bounds proof under a tight rational budget: `lower` and `upper`
/// certificates the checker accepts.
#[trace("TC-2065", "FR-201-AC-51")]
#[test]
#[ignore = "QSL-2158: FR-1042 (lower/upper value bounds) not implemented yet"]
fn value_bounds_certificates_are_accepted() {
    let source = likely(
        "inf",
        "Even",
        "probability >= rational(1,2) [ eventually holds(m.delivered) ]",
    );
    let limits = ExactProbLimits {
        max_rational_bits: 2,
        ..ExactProbLimits::default()
    };
    accepted_then_rejected(&certified(&source, limits), &["lower", "upper"]);
}

/// A finite-horizon quantile over every scheduler: its certificate is
/// accepted by the checker.
#[trace("TC-2065", "FR-201-AC-51", "FR-1043-AC-6")]
#[test]
fn a_finite_horizon_quantile_certificate_is_accepted() {
    let source = likely(
        "ev",
        "every scheduler",
        "quantile 0.4 of (steps from holds(true) until holds(m.delivered) within 4) <= 3",
    );
    let request = package_request(&source);
    let doc = document(&request);
    assert_eq!(doc["objective"]["payoff"], "shifted");
    let initial = doc["ranking"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["rank"] == 4)
        .expect("the initial remaining-horizon row");
    let value = doc["values"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["state"] == initial["state"])
        .unwrap();
    assert_eq!(
        value["value"],
        json!({"numerator":"11","denominator":"135"})
    );
    accepted_then_rejected(&request, &["exact"]);
    let false_source = source.replace("quantile 0.4", "quantile 0.5");
    let (certificate, settlement) = request_with(
        &false_source,
        SNAPSHOT,
        ExactProbLimits::default(),
        domain().as_bytes(),
    );
    assert!(certificate.is_none());
    assert!(
        matches!(&settlement,fsl_analyze::analyze::Settlement::Terminal(record) if record.value()==&fsl_replay::TerminalValue::Refuted),
        "{settlement:?}"
    );
}

/// The long-run kind: a long-run fraction under a workload is accepted as a
/// `long-run` certificate. Every run of the cyclic model ends in the
/// terminal state where `delivered` is false, so the fraction is 0 and only
/// the `<=` side is provable.
#[trace("TC-2065", "FR-201-AC-51")]
#[test]
fn a_long_run_certificate_is_accepted() {
    let source = likely(
        "inf",
        "Even",
        "long_run_fraction holds(m.delivered) <= rational(1,2)",
    );
    accepted_then_rejected(
        &certified(&source, ExactProbLimits::default()),
        &["long-run"],
    );
}

/// The probabilistic unit's source with a reward, whose exact certificate
/// is checked over the package.
fn package_source() -> String {
    likely(
        "inf",
        "Even",
        "probability >= rational(1,2) [ eventually holds(m.delivered) ]",
    )
    .replace(
        "terminal Stop",
        "reward cost on Msg::send_a = 1;\nterminal Stop",
    )
}

/// The request that names the package S4 emits for `source` in place of the
/// source; its certificate is the document's bytes as the run wrote them.
fn package_request(source: &str) -> Value {
    let request = certified(source, ExactProbLimits::default());
    let (checked, _) = super::bounded::compile_source_with(source, domain().as_bytes());
    let emitted = fsl_replay::spine::package(
        &checked,
        fsl_replay::spine::PackageLimits::default(),
        &filament_exact::Cancel::new(),
    )
    .unwrap_or_else(|failure| panic!("S4 emits the probabilistic unit: {failure:?}"))
    .into_value();
    let mut later = request;
    later
        .as_object_mut()
        .unwrap()
        .remove("source")
        .expect("the source member");
    later["package"] = json!(emitted.package().bytes().to_vec());
    later
}

/// TC-2065 (FR-203-AC-8, FR-282-AC-20): the exact certificate `analyze`
/// writes for a claim over a unit with random parameters, a workload and a
/// reward is carried as bytes out of the run and accepted by the checker in
/// a separate, later process over the package S4 emits: the request names
/// the emitted `filament.checked-package/v2` bytes in place of the source,
/// and the same certificate with one value altered is rejected.
#[trace("TC-2065", "FR-203-AC-8")]
#[trace("TC-2065", "FR-282-AC-20")]
#[test]
fn an_analyze_certificate_is_accepted_later_over_the_emitted_package() {
    let later = package_request(&package_source());
    let (code, output) = run(&later);
    assert_eq!(code, 0, "{output}");

    let altered = mutate(&later, |doc| {
        doc["values"][0]["value"] = json!({"numerator": "1", "denominator": "7"});
    });
    let (code, output) = run(&altered);
    assert_ne!(code, 0, "an altered certificate is rejected: {output}");
}

/// A request the package route must refuse, never accept.
fn refused_package(request: &Value, reason: &str) {
    let (code, output) = run(request);
    assert_eq!(code, 2, "{output}");
    assert!(
        output["reason"]
            .as_str()
            .is_some_and(|text| text.contains(reason)),
        "{reason}: {output}"
    );
}

/// FR-282-AC-20 (negative): the package the provision recompiles to must be
/// the package the request names; a source that recompiles to other bytes is
/// refused.
#[trace("TC-2065", "FR-282-AC-20")]
#[test]
fn a_package_the_provision_does_not_recompile_to_is_refused() {
    let mut request = package_request(&package_source());
    let other = package_source().replace(
        "terminal Stop",
        "reward extra on Msg::send_a = 1;\nterminal Stop",
    );
    request["byte_provision"][0][2] = json!(other.as_bytes());
    refused_package(
        &request,
        "conflicting_identity: probability certificate package",
    );
}

/// FR-282-AC-20 (negative): a package that names no source, or two, and bytes
/// that are no document, are refused with their own cause.
#[trace("TC-2065", "FR-282-AC-20")]
#[test]
fn a_package_without_exactly_one_source_or_without_a_document_is_refused() {
    let request = package_request(&package_source());
    let bytes: Vec<u8> = serde_json::from_value(request["package"].clone()).unwrap();
    let mut document: Value = serde_json::from_slice(&bytes).unwrap();
    document["lock"]["sources"] = json!([]);
    let mut none = request.clone();
    none["package"] = json!(serde_json::to_vec(&document).unwrap());
    refused_package(&none, "names 0 sources");
    let mut unreadable = request;
    unreadable["package"] = json!([255, 254]);
    refused_package(&unreadable, "package unreadable");
}

/// FR-282-AC-20 (negative): a stated package name other than the package's
/// own is refused, and a request names exactly one of source and package.
#[trace("TC-2065", "FR-282-AC-20")]
#[test]
fn a_package_request_names_its_own_package_and_exactly_one_subject() {
    let request = package_request(&package_source());
    let mut renamed = request.clone();
    renamed["package_name"] = json!("another-package");
    refused_package(&renamed, "package name");
    let mut neither = request.clone();
    neither.as_object_mut().unwrap().remove("package");
    refused_package(&neither, "exactly one of source and package");
    let mut both = request;
    both["source"] = json!({"authority": "test", "identity": "bounded-certificate-source", "content": package_source()});
    refused_package(&both, "exactly one of source and package");
}

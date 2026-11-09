// SPDX-License-Identifier: AGPL-3.0-or-later
//! A separate process boundary around the qualified source-backed checker.
#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::io::{Read, Write};

use qsl_eval::simulation::model::{ExpansionLimits, PopulationUniverse};
use qsl_replay::spine::SourceComponent;
use qsl_replay::{
    check_probability_certificate_source, ByteProvision, DefinitionSourceRef,
    ExactProbabilityCertificateCheck, ObligationIdentity, ProbabilityDocumentLimits,
    ProbabilitySourceLimits, ProbabilitySourceRequest,
};
use qsl_semantics::model::observation::DocumentRef;
use quire_checked::key::DeclarationKey;
use quire_exact::{Cancel, CancelCause};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Identity {
    node_id: Value,
    model: Option<String>,
    ordinal: u32,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    authority: String,
    identity: String,
    content: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    authority: String,
    identity: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Population {
    package: String,
    node: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Universe {
    population: Population,
    keys: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Limits {
    bytes: u64,
    states: usize,
    edges: usize,
    scalar_bits: u64,
    expansion: u64,
    stages: BTreeMap<String, u64>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    source: Source,
    package_name: String,
    item: Identity,
    byte_provision: Vec<(String, String, Vec<u8>)>,
    documents: Vec<Vec<u8>>,
    initial: Vec<Document>,
    universes: Vec<Universe>,
    certificate: String,
    limits: Limits,
}

fn check(request: Request, cancel: &Cancel) -> Result<Value, String> {
    let source = DefinitionSourceRef::from_wire((request.source.authority.clone(), request.source.identity.clone()));
    let provision = ByteProvision::admit(request.byte_provision, request.documents, [("source".to_owned(), &source)])
        .map_err(|error| error.to_string())?;
    let node = qsl_foundation::node_wire::node_key_from_json(&request.item.node_id).map_err(|error| error.to_string())?;
    let item = ObligationIdentity::new(node, request.item.model.map(qsl_replay::ModelIdentity::new), request.item.ordinal);
    let operands = request.limits.stages.iter().map(|(name, bound)| format!("{name}={bound}")).collect::<Vec<_>>();
    let caller = qsl_replay::CallerLimits::from_operands(operands.iter().map(String::as_str)).map_err(|error| error.to_string())?;
    let expected = SourceComponent { authority: request.source.authority, identity: request.source.identity, content: request.source.content };
    let result = check_probability_certificate_source(ProbabilitySourceRequest {
        source: &source, expected_source: &expected, package_name: &request.package_name,
        provision: &provision, item: &item,
        initial: request.initial.into_iter().map(|document| DocumentRef { authority: document.authority, identity: document.identity }).collect(),
        universes: request.universes.into_iter().map(|universe| PopulationUniverse {
            population: DeclarationKey { package: universe.population.package, node: universe.population.node }, keys: universe.keys,
        }).collect(),
        certificate: request.certificate.as_bytes(),
    }, ProbabilitySourceLimits {
        spine: caller.spine, observation: caller.observation, expansion: ExpansionLimits { max_candidates: request.limits.expansion },
        scalar: caller.accounting.0, document: ProbabilityDocumentLimits {
            bytes: request.limits.bytes, states: request.limits.states, edges: request.limits.edges, scalar_bits: request.limits.scalar_bits, accounting: caller.accounting.0,
        },
    }, cancel).map_err(|error| error.to_string())?;
    Ok(match result {
        ExactProbabilityCertificateCheck::Accepted(accepted) => json!({"status":"accepted", "value": {
            "numerator": accepted.value().numerator().to_string(), "denominator": accepted.value().denominator().to_string(),
        }}),
        ExactProbabilityCertificateCheck::Rejected(rejection) => json!({"status":"rejected", "rule":format!("{:?}", rejection.rule)}),
    })
}

fn run() -> Result<Value, String> {
    let mut input = None;
    let mut bound = 16_777_216_u64;
    let cancel = Cancel::new();
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--max-input-bytes" => bound = arguments.next().ok_or("missing input byte ceiling")?.parse().map_err(|_| "invalid input byte ceiling")?,
            "--cancel" => cancel.cancel(CancelCause::Requested),
            "-" => {},
            _ if !argument.starts_with('-') && input.is_none() => input = Some(argument),
            _ => return Err(format!("unsupported argument: {argument}")),
        }
    }
    let reader: Box<dyn Read> = match input {
        Some(path) => Box::new(std::fs::File::open(path).map_err(|error| error.to_string())?),
        None => Box::new(std::io::stdin()),
    };
    let mut bytes = Vec::new();
    reader.take(bound.checked_add(1).ok_or("input byte ceiling overflow")?).read_to_end(&mut bytes).map_err(|error| error.to_string())?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > bound { return Err("checker input byte limit reached".to_owned()); }
    let request = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    check(request, &cancel)
}

fn main() {
    let (value, code) = match run() {
        Ok(value) => { let code = if value.get("status").and_then(Value::as_str) == Some("accepted") { 0 } else { 1 }; (value, code) },
        Err(error) => (json!({"status":"stopped", "reason":error}), 2),
    };
    let mut output = std::io::stdout().lock();
    if serde_json::to_writer(&mut output, &value).is_err() || output.write_all(b"\n").is_err() { std::process::exit(3); }
    std::process::exit(code);
}

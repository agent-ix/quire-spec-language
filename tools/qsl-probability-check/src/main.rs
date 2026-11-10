// SPDX-License-Identifier: AGPL-3.0-or-later
//! A separate process boundary around the qualified source-backed checker.
#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::io::{Read, Write};

use qsl_eval::simulation::model::{ExpansionLimits, PopulationUniverse};
use qsl_replay::spine::SourceComponent;
use qsl_replay::{
    check_probability_certificate_source, ByteProvision, DefinitionSourceRef,
    ExactProbabilityCertificateCheck, ObligationIdentity, ProbabilityDocumentError,
    ProbabilityDocumentLimits, ProbabilitySourceError, ProbabilitySourceLimits,
    ProbabilitySourceRequest,
};
use qsl_semantics::model::observation::DocumentRef;
use quire_checked::key::DeclarationKey;
use quire_exact::{Cancel, CancelCause};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Identity {
    node_id: Box<serde_json::value::RawValue>,
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
    storage_bytes: u64,
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
    let source = DefinitionSourceRef::from_wire((
        request.source.authority.clone(),
        request.source.identity.clone(),
    ));
    let provision = ByteProvision::admit(
        request.byte_provision,
        request.documents,
        [("source".to_owned(), &source)],
    )
    .map_err(|error| error.to_string())?;
    let node = qsl_foundation::node_wire::node_key_from_text(request.item.node_id.get())
        .map_err(|error| error.to_string())?;
    let item = ObligationIdentity::new(
        node,
        request.item.model.map(qsl_replay::ModelIdentity::new),
        request.item.ordinal,
    );
    let operands = request
        .limits
        .stages
        .iter()
        .map(|(name, bound)| format!("{name}={bound}"))
        .collect::<Vec<_>>();
    let caller = qsl_replay::CallerLimits::from_operands(operands.iter().map(String::as_str))
        .map_err(|error| error.to_string())?;
    let expected = SourceComponent {
        authority: request.source.authority,
        identity: request.source.identity,
        content: request.source.content,
    };
    let result = check_probability_certificate_source(
        ProbabilitySourceRequest {
            source: &source,
            expected_source: &expected,
            package_name: &request.package_name,
            provision: &provision,
            item: &item,
            initial: request
                .initial
                .into_iter()
                .map(|document| DocumentRef {
                    authority: document.authority,
                    identity: document.identity,
                })
                .collect(),
            universes: request
                .universes
                .into_iter()
                .map(|universe| PopulationUniverse {
                    population: DeclarationKey {
                        package: universe.population.package,
                        node: universe.population.node,
                    },
                    keys: universe.keys,
                })
                .collect(),
            certificate: request.certificate.as_bytes(),
        },
        ProbabilitySourceLimits {
            spine: caller.spine,
            observation: caller.observation,
            expansion: ExpansionLimits {
                max_candidates: request.limits.expansion,
            },
            scalar: caller.accounting.0,
            document: ProbabilityDocumentLimits {
                bytes: request.limits.bytes,
                storage_bytes: request.limits.storage_bytes,
                states: request.limits.states,
                edges: request.limits.edges,
                scalar_bits: request.limits.scalar_bits,
                accounting: caller.accounting.0,
            },
        },
        cancel,
    );
    let result = match result {
        Ok(result) => result,
        Err(error) => return source_failure(error),
    };
    Ok(match result {
        ExactProbabilityCertificateCheck::Accepted(accepted) => {
            json!({"status":"accepted", "value": {
                "numerator": accepted.value().numerator().to_string(), "denominator": accepted.value().denominator().to_string(),
            }})
        }
        ExactProbabilityCertificateCheck::Rejected(rejection) => {
            let rule = match rejection.rule {
                qsl_replay::ProbabilityCertificateRule::InequalityFails => {
                    qsl_replay::CertificateRule::InequalityFails
                }
                qsl_replay::ProbabilityCertificateRule::ThresholdMissed => {
                    qsl_replay::CertificateRule::ThresholdMissed
                }
                qsl_replay::ProbabilityCertificateRule::Malformed => {
                    qsl_replay::CertificateRule::Malformed
                }
                qsl_replay::ProbabilityCertificateRule::RankMissing => {
                    qsl_replay::CertificateRule::RankMissing
                }
                qsl_replay::ProbabilityCertificateRule::RankBroken => {
                    qsl_replay::CertificateRule::RankBroken
                }
            };
            let at = match rejection.at {
                qsl_replay::ProbabilityCertificateLocus::ProductState(key) => {
                    serde_json::to_value(qsl_replay::CertificateLocus::ProbabilityProductState(key))
                        .map_err(|error| error.to_string())?
                }
                qsl_replay::ProbabilityCertificateLocus::Member(member) => json!({"member":member}),
            };
            json!({"status":"rejected", "rule":rule, "at":at})
        }
    })
}

fn source_failure(error: ProbabilitySourceError) -> Result<Value, String> {
    use ProbabilityDocumentError as Document;
    let status = match &error {
        ProbabilitySourceError::Document(Document::Build(issue))
            if matches!(
                issue.as_ref(),
                qsl_replay::ExactProbBuildIssue::NotMarkov
                    | qsl_replay::ExactProbBuildIssue::Unsupported(_)
            ) =>
        {
            "unsupported"
        }
        ProbabilitySourceError::Compile(
            qsl_foundation::diagnostic::StageFailure::Refused(_)
            | qsl_foundation::diagnostic::StageFailure::RefusedWithPayload(_, _),
        ) => "refused",
        ProbabilitySourceError::Document(
            Document::Unsupported | Document::ZeroWeightComponent { .. },
        ) => "unsupported",
        ProbabilitySourceError::Document(
            Document::KeyEncoding(qsl_eval::simulation::EncodingRefusal::NoEncoding(_))
            | Document::UndefinedPredicate(_)
            | Document::MalformedMember(_)
            | Document::Malformed
            | Document::MissingMember(_)
            | Document::Noncanonical
            | Document::IdentityMismatch
            | Document::ObjectiveMismatch,
        )
        | ProbabilitySourceError::SourceMismatch
        | ProbabilitySourceError::Source { .. }
        | ProbabilitySourceError::Packages(_) => "refused",
        ProbabilitySourceError::Document(
            Document::KeyEncoding(
                qsl_eval::simulation::EncodingRefusal::Limit(_)
                | qsl_eval::simulation::EncodingRefusal::Allocation { .. },
            )
            | Document::KeySizeOverflow
            | Document::KeyDenied
            | Document::Limit { .. }
            | Document::Cancelled(_)
            | Document::Accounting(_)
            | Document::IntegerText { .. }
            | Document::Graph(_)
            | Document::Build(_),
        )
        | ProbabilitySourceError::ProvisionLimit { .. }
        | ProbabilitySourceError::Compile(_)
        | ProbabilitySourceError::Subject(_) => "stopped",
    };
    let limit = match &error {
        ProbabilitySourceError::Document(Document::KeyEncoding(
            qsl_eval::simulation::EncodingRefusal::Limit(limit),
        )) => Some(
            json!({"setting":limit.setting().name(),"bound":limit.configured_bound(),"actual":limit.actual().to_string()}),
        ),
        ProbabilitySourceError::Compile(
            qsl_foundation::diagnostic::StageFailure::Limit(limit)
            | qsl_foundation::diagnostic::StageFailure::LimitWithPayload(limit, _),
        ) => {
            let actual = serde_json::Number::from_u128(limit.actual())
                .map(Value::Number)
                .unwrap_or_else(|| Value::String(limit.actual().to_string()));
            Some(
                json!({"setting":limit.setting().name(), "bound":limit.configured_bound(), "actual":actual}),
            )
        }
        ProbabilitySourceError::Document(Document::Limit {
            limit,
            bound,
            actual,
        }) => Some(json!({"setting":limit.setting(), "bound":bound, "actual":actual})),
        ProbabilitySourceError::ProvisionLimit {
            setting,
            bound,
            actual,
        } => Some(json!({"setting":setting, "bound":bound, "actual":actual})),
        ProbabilitySourceError::Source {
            cause: qsl_foundation::source::SourceReadCause::ByteBudget,
            byte_bound,
            byte_length,
            ..
        } => Some(json!({"setting":"s1.input_bytes", "bound":byte_bound, "actual":byte_length})),
        ProbabilitySourceError::Document(Document::Accounting(incomplete)) => Some(json!({
            "setting":qsl_replay::AccountingLimits::setting_of(incomplete.limit_kind).name(),
            "bound":incomplete.limit, "consumed":incomplete.consumed, "next_charge":incomplete.next_charge.to_string(),
        })),
        ProbabilitySourceError::Document(Document::IntegerText {
            bit_bound,
            decimal_digits,
        }) => Some(
            json!({"setting":"exact_prob.max_rational_bits", "bound":bit_bound, "decimal_digits":decimal_digits}),
        ),
        _ => None,
    };
    let mut result = json!({"status": status, "reason": error.to_string(), "limit":limit});
    if let ProbabilitySourceError::Document(Document::ZeroWeightComponent { state }) = &error {
        result["at"] = serde_json::to_value(qsl_replay::CertificateLocus::ProbabilityProductState(
            state.clone(),
        ))
        .map_err(|error| error.to_string())?;
    }
    Ok(result)
}

fn run() -> Result<Value, String> {
    let mut input = None;
    let mut bound = 16_777_216_u64;
    let cancel = Cancel::new();
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--max-input-bytes" => {
                bound = arguments
                    .next()
                    .ok_or("missing input byte ceiling")?
                    .parse()
                    .map_err(|_| "invalid input byte ceiling")?
            }
            "--cancel" => cancel.cancel(CancelCause::Requested),
            "-" => {}
            _ if !argument.starts_with('-') && input.is_none() => input = Some(argument),
            _ => return Err(format!("unsupported argument: {argument}")),
        }
    }
    let reader: Box<dyn Read> = match input {
        Some(path) => Box::new(std::fs::File::open(path).map_err(|error| error.to_string())?),
        None => Box::new(std::io::stdin()),
    };
    let mut bytes = Vec::new();
    reader
        .take(bound.checked_add(1).ok_or("input byte ceiling overflow")?)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > bound {
        return Err("checker input byte limit reached".to_owned());
    }
    let request = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    check(request, &cancel)
}

fn main() {
    let (value, code) = match run() {
        Ok(value) => {
            let code = match value.get("status").and_then(Value::as_str) {
                Some("accepted") => 0,
                Some("rejected") => 1,
                _ => 2,
            };
            (value, code)
        }
        Err(error) => (json!({"status":"stopped", "reason":error}), 2),
    };
    let mut output = std::io::stdout().lock();
    if serde_json::to_writer(&mut output, &value).is_err() || output.write_all(b"\n").is_err() {
        std::process::exit(3);
    }
    std::process::exit(code);
}

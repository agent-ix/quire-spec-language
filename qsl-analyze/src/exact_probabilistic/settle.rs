// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-203's map from FR-196 product-build refusals.

use qsl_eval::simulation::NotSimulated;
use qsl_replay::statistical::SubjectLimits;
use qsl_replay::{settle_exact_prob_build_issue, ExactProbBuildIssue, ExactProbBuildRecord};
use quire_semantic_value::catalog::CatalogCode;

use super::{ProductRefusal, ProductStop};

/// Settle a product build refusal. A request refusal has no terminal record;
/// every run stop or settled build failure has an FR-331 result.
pub fn settle_product_refusal(
    refusal: ProductRefusal,
    subject: SubjectLimits<'_>,
) -> Result<ExactProbBuildRecord, ProductRefusal> {
    let issue = match refusal {
        request @ ProductRefusal::Admission(_) => return Err(request),
        request @ ProductRefusal::NotSimulated(NotSimulated::RequiresBound(_)) => {
            return Err(request)
        }
        ProductRefusal::NotSimulated(
            refusal @ (NotSimulated::Extent(_)
            | NotSimulated::GeneratorMismatch { .. }
            | NotSimulated::EmptyInitial
            | NotSimulated::KeyEncoding(_)),
        ) => ExactProbBuildIssue::Failed(refusal.catalog_code().unwrap_or(CatalogCode::new(
            "runtime_invariant",
            "established-invariant-broken",
        ))),
        ProductRefusal::Expansion(stop) => ExactProbBuildIssue::Expansion(stop),
        ProductRefusal::NotMarkov(_) => ExactProbBuildIssue::NotMarkov,
        ProductRefusal::UndecidedSuccessor(_) => ExactProbBuildIssue::UndecidedSuccessor,
        ProductRefusal::PredicateRefused(outcome) => ExactProbBuildIssue::Predicate(outcome),
        ProductRefusal::Translation(_) | ProductRefusal::Internal(_) => {
            ExactProbBuildIssue::BrokenInvariant
        }
        ProductRefusal::Stopped(ProductStop::Limit { limit, value }) => {
            ExactProbBuildIssue::Limit { limit, value }
        }
        ProductRefusal::Stopped(ProductStop::Cancelled(source)) => {
            ExactProbBuildIssue::Cancelled(source)
        }
    };
    Ok(settle_exact_prob_build_issue(&issue, subject))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    use qsl_eval::simulation::model::ExpansionLimits;
    use qsl_replay::{IncompleteCause, ModelCheckLimit, TerminalValue};
    use quire_exact::ScalarLimits;
    use quire_semantic_value::catalog::CatalogCode;

    #[trace("TC-1166", "FR-203-AC-7")]
    #[test]
    fn product_stops_settle_exact_limits_and_expansion_limits() {
        let meter = ScalarLimits {
            integer_bits: 64,
            decimal_digits: 64,
            scale_expansion: 64,
            text_input_bytes: 64,
            text_scalars: 64,
            normalized_scalars: 64,
            unit_edges: 64,
            value_occurrences: 64,
            work_units: 4_096,
            result_units: 64,
        };
        let subject = SubjectLimits {
            expansion: ExpansionLimits { max_candidates: 2 },
            meter: &meter,
        };
        let product_limit = settle_product_refusal(
            ProductRefusal::Stopped(ProductStop::Limit {
                limit: ModelCheckLimit::ExactProbMaxStates,
                value: 1_000,
            }),
            subject,
        )
        .expect("a reached budget settles");
        assert_eq!(
            product_limit.value(),
            &TerminalValue::Incomplete(IncompleteCause::LimitReached {
                limit: ModelCheckLimit::ExactProbMaxStates,
                value: 1_000,
            })
        );

        let expansion = settle_product_refusal(
            ProductRefusal::Expansion(qsl_eval::simulation::ExpansionStop {
                cause: CatalogCode::new("resource_exhausted", "insufficient-next-charge"),
                limit: Some(qsl_eval::simulation::ExpansionLimit::MaxCandidates),
            }),
            subject,
        )
        .expect("a named expansion budget settles");
        assert_eq!(
            expansion.value(),
            &TerminalValue::Incomplete(IncompleteCause::LimitReached {
                limit: ModelCheckLimit::MaxCandidates,
                value: 2,
            })
        );
    }
}

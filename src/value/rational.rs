// SPDX-License-Identifier: AGPL-3.0-or-later
//! Canonical exact rationals (AD-005, FR-140 loss records).
//!
//! [`Rational`], [`ZeroDenominator`], [`RationalDomain`] and
//! [`NonPositiveDenominatorBound`] are `quire_exact`'s own canonical items,
//! re-exported below rather than duplicated. Every method these types carry
//! -- `new`, `from_integer`, `numerator`, `denominator`, `is_integer`,
//! `is_zero`, `max_part_bits`, `add`, `sub`, `neg`, `mul`, `div`, `pow`,
//! `divided_by_power_of_ten`, `divided_by_power_of_two`,
//! `Ord`/`PartialOrd`/`Display`, `RationalDomain::new`/`numerator`/
//! `denominator`/`contains`/`contains_domain`/`excludes_zero`/`negated`/
//! `result_of` -- is reachable straight off the re-exported type; none of
//! it is duplicated here.
//!
//! `RationalDomain::result_of` takes `quire_exact::ArithmeticOperator`
//! (`value::numeric` re-exports the same type as `ArithmeticOperator`), so
//! callers pass it directly.
//!
//! `value::decimal`'s own evaluation engine (`evaluate_decimal` and
//! everything beneath it) stays local: it returns this crate's own
//! `Outcome`/`Refusal` (`value::outcome`), a strict superset of
//! `quire_exact`'s kernel `Outcome`/`Refusal`, and constructs
//! `DecimalResult`/`DecimalLoss` only through their own private struct
//! literals -- neither type has a public constructor in `quire_exact` -- so
//! that engine cannot be cut until `value::outcome` unifies with
//! `quire_exact::outcome` (QSL-166, QSL-174).

pub use quire_exact::{NonPositiveDenominatorBound, Rational, RationalDomain, ZeroDenominator};

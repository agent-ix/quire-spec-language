// SPDX-License-Identifier: AGPL-3.0-or-later
//! Canonical exact rationals (AD-005, FR-140 loss records).
//!
//! [`Rational`](quire_exact::Rational), [`ZeroDenominator`], [`RationalDomain`]
//! and [`NonPositiveDenominatorBound`] are `quire_exact`'s own canonical
//! items. `ZeroDenominator`, `RationalDomain` and `NonPositiveDenominatorBound`
//! are re-exported below rather than duplicated; `Rational` itself is not
//! re-exported here -- every caller in this crate imports it straight from
//! `quire_exact`. Every method these types carry -- `new`, `from_integer`,
//! `numerator`, `denominator`, `is_integer`,
//! `is_zero`, `max_part_bits`, `add`, `sub`, `neg`, `mul`, `div`, `pow`,
//! `divided_by_power_of_ten`, `divided_by_power_of_two`,
//! `Ord`/`PartialOrd`/`Display`, `RationalDomain::new`/`numerator`/
//! `denominator`/`contains`/`contains_domain`/`excludes_zero`/`negated`/
//! `result_of` -- is reachable straight off `quire_exact`'s own type; none of
//! it is duplicated here.
//!
//! `RationalDomain::result_of` takes `quire_exact::ArithmeticOperator`
//! directly.
//!
//! `value::decimal`'s own evaluation engine (`evaluate_decimal` and
//! everything beneath it) stays local: it returns this crate's own
//! `Outcome`/`Refusal` (`value::outcome`), a strict superset of
//! `quire_exact`'s kernel `Outcome`/`Refusal`, and constructs
//! `DecimalResult`/`DecimalLoss` only through their own private struct
//! literals -- neither type has a public constructor in `quire_exact` -- so
//! that engine cannot be cut until `value::outcome` unifies with
//! `quire_exact::outcome` (QSL-166, QSL-174).

pub use quire_exact::{NonPositiveDenominatorBound, RationalDomain, ZeroDenominator};

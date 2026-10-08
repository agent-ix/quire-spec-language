// SPDX-License-Identifier: AGPL-3.0-or-later
//! The caller limits and FR-255's settings operation.
//!
//! [`CallerLimits`] holds every limits type of the stages QSL runs, each at
//! its published default. [`CallerLimits::from_operands`] is the settings
//! operation the driver CLI calls with its `--limit <name>=<value>`
//! operands, and a replay request's `stage_limits` entries are applied by the
//! same [`SettingLimits::set_bound`], so one name raises one limit at every
//! entry point.

use qsl_eval::simulation::Limits as ExploreLimits;
use qsl_foundation::{AccountingSetting, Setting, SettingLimits, SettingName, UsageRefusal};
use qsl_semantics::library::bundle::PackageLimits as LibraryLimits;
use qsl_semantics::model::observation::ObservationLimits;
use qsl_semantics::model::population::PopulationAdmissionLimits;
use quire_exact::{LimitKind as Counter, ScalarLimits};

use crate::bounds::ReplayLimits;
use crate::spine::{default_accounting, SpineLimits, DEFAULT_WORK_UNITS};

/// The call's accounting limits (`quire.value.accounting/v1`): each of the
/// ten counters is an [`AccountingSetting`] named by the counter (FR-255's
/// accounting table). A newtype over the foreign [`ScalarLimits`]. The
/// default is FR-100's: `work_units` 1,000,000 and every other counter
/// `u64::MAX`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AccountingLimits(pub ScalarLimits);

impl Default for AccountingLimits {
    fn default() -> Self {
        Self(default_accounting(DEFAULT_WORK_UNITS))
    }
}

impl AccountingLimits {
    /// The accounting limits a replay's call runs under (FR-255
    /// Behavior 10): `base`, the request's own accounting limits, with each
    /// counter an entry names at the entry's bound.
    pub(crate) fn with_entries(
        base: ScalarLimits,
        entries: impl IntoIterator<Item = (AccountingSetting, u64)>,
    ) -> ScalarLimits {
        let mut limits = Self(base);
        for (setting, bound) in entries {
            limits.set_bound(setting, bound);
        }
        limits.0
    }

    /// The setting that raises `counter`'s bound (FR-255 Behavior 9).
    #[must_use]
    pub const fn setting_of(counter: Counter) -> AccountingSetting {
        match counter {
            Counter::IntegerBits => AccountingSetting::IntegerBits,
            Counter::DecimalDigits => AccountingSetting::DecimalDigits,
            Counter::ScaleExpansion => AccountingSetting::ScaleExpansion,
            Counter::TextInputBytes => AccountingSetting::TextInputBytes,
            Counter::TextScalars => AccountingSetting::TextScalars,
            Counter::NormalizedScalars => AccountingSetting::NormalizedScalars,
            Counter::UnitEdges => AccountingSetting::UnitEdges,
            Counter::ValueOccurrences => AccountingSetting::ValueOccurrences,
            Counter::WorkUnits => AccountingSetting::WorkUnits,
            Counter::ResultUnits => AccountingSetting::ResultUnits,
        }
    }

    /// Every counter with its setting and current bound, in the accounting
    /// table's order.
    #[must_use]
    pub fn bounds(&self) -> Vec<(AccountingSetting, u64)> {
        let limits = &self.0;
        vec![
            (AccountingSetting::IntegerBits, limits.integer_bits),
            (AccountingSetting::DecimalDigits, limits.decimal_digits),
            (AccountingSetting::ScaleExpansion, limits.scale_expansion),
            (AccountingSetting::TextInputBytes, limits.text_input_bytes),
            (AccountingSetting::TextScalars, limits.text_scalars),
            (
                AccountingSetting::NormalizedScalars,
                limits.normalized_scalars,
            ),
            (AccountingSetting::UnitEdges, limits.unit_edges),
            (
                AccountingSetting::ValueOccurrences,
                limits.value_occurrences,
            ),
            (AccountingSetting::WorkUnits, limits.work_units),
            (AccountingSetting::ResultUnits, limits.result_units),
        ]
    }

    /// Set the bound of the counter `setting` names and change nothing else.
    pub fn set_bound(&mut self, setting: AccountingSetting, bound: u64) {
        let limits = &mut self.0;
        let field = match setting {
            AccountingSetting::IntegerBits => &mut limits.integer_bits,
            AccountingSetting::DecimalDigits => &mut limits.decimal_digits,
            AccountingSetting::ScaleExpansion => &mut limits.scale_expansion,
            AccountingSetting::TextInputBytes => &mut limits.text_input_bytes,
            AccountingSetting::TextScalars => &mut limits.text_scalars,
            AccountingSetting::NormalizedScalars => &mut limits.normalized_scalars,
            AccountingSetting::UnitEdges => &mut limits.unit_edges,
            AccountingSetting::ValueOccurrences => &mut limits.value_occurrences,
            AccountingSetting::WorkUnits => &mut limits.work_units,
            AccountingSetting::ResultUnits => &mut limits.result_units,
        };
        *field = bound;
    }
}

/// Every limits type a caller can configure, one field per stage.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CallerLimits {
    /// S1, I1, S3, type-environment admission and the I2 read.
    pub spine: SpineLimits,
    /// Library resolution.
    pub library: LibraryLimits,
    /// Population admission.
    pub admission: PopulationAdmissionLimits,
    /// Observation admission.
    pub observation: ObservationLimits,
    /// Exploration.
    pub explore: ExploreLimits,
    /// The replay request and envelope readers.
    pub replay: ReplayLimits,
    /// The call's accounting budgets (`quire.value.accounting/v1`).
    pub accounting: AccountingLimits,
}

impl CallerLimits {
    /// The settings operation (FR-255 Behavior 6): the limits with each
    /// `<name>=<value>` operand's setting at its value, and every other
    /// limit at its default.
    ///
    /// # Errors
    ///
    /// [`UsageRefusal`] naming the first operand that is not
    /// `<name>=<value>`, names no setting, holds a value that is not a
    /// non-negative decimal integer, or repeats a name. Nothing runs.
    pub fn from_operands<'a>(
        operands: impl IntoIterator<Item = &'a str>,
    ) -> Result<Self, UsageRefusal> {
        let mut limits = Self::default();
        for (setting, bound) in qsl_foundation::setting::parse_operands(operands)? {
            match setting {
                SettingName::Stage(setting) => {
                    limits.set_bound(setting, bound);
                }
                SettingName::Accounting(setting) => limits.accounting.set_bound(setting, bound),
            }
        }
        Ok(limits)
    }

    /// The limits as the replay of a request with `stage_limits` runs them:
    /// `replay.input_bytes` from the library entry's own limit (a
    /// `stage_limits` entry for it is ignored, whichever way the map was
    /// built), and every other setting at the request's entry or its
    /// default. `accounting` is the request's own accounting limits; the
    /// result's [`Self::accounting`] is those with each counter a
    /// `stage_limits` entry names replaced (FR-255 Behavior 10).
    pub(crate) fn for_request(
        stage_limits: &crate::request::StageLimits,
        accounting: ScalarLimits,
        replay: ReplayLimits,
    ) -> Self {
        let mut limits = Self {
            replay,
            accounting: AccountingLimits(AccountingLimits::with_entries(
                accounting,
                stage_limits.accounting_entries(),
            )),
            ..Self::default()
        };
        for (setting, bound) in stage_limits.iter() {
            // A request does not carry `replay.input_bytes`: the library
            // entry's own limit bounds the request itself.
            if setting != Setting::ReplayInputBytes {
                limits.set_bound(setting, bound);
            }
        }
        limits
    }

    fn parts(&self) -> [&dyn SettingLimits; 6] {
        [
            &self.spine,
            &self.library,
            &self.admission,
            &self.observation,
            &self.explore,
            &self.replay,
        ]
    }
}

impl SettingLimits for CallerLimits {
    fn bounds(&self) -> Vec<(Setting, u64)> {
        self.parts().iter().flat_map(|part| part.bounds()).collect()
    }

    fn set_bound(&mut self, setting: Setting, bound: u64) -> bool {
        let parts: [&mut dyn SettingLimits; 6] = [
            &mut self.spine,
            &mut self.library,
            &mut self.admission,
            &mut self.observation,
            &mut self.explore,
            &mut self.replay,
        ];
        parts.into_iter().any(|part| part.set_bound(setting, bound))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use ix_trace_rs::trace;
    use qsl_foundation::UsageCause;

    use super::*;
    use crate::request::StageLimits;

    fn bound_of(limits: &CallerLimits, setting: Setting) -> u64 {
        limits
            .bounds()
            .into_iter()
            .find(|(reported, _)| *reported == setting)
            .map(|(_, bound)| bound)
            .unwrap_or_else(|| panic!("no limits type reports {setting}"))
    }

    /// One row of FR-255's setting table.
    struct Row {
        name: String,
        stage: String,
        kind: String,
        default: String,
    }

    const SPEC: &str = include_str!(
        "../../spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md"
    );

    /// The table rows of the section under `heading`, up to the next `## `.
    fn section_rows(heading: &str) -> Vec<Vec<&'static str>> {
        let body = SPEC
            .split_once(heading)
            .unwrap_or_else(|| panic!("FR-255 has no `{heading}` section"))
            .1;
        let body = body
            .split_once("\n## ")
            .map_or(body, |(section, _)| section);
        body.lines()
            .filter(|line| line.starts_with("| `"))
            .map(|line| line.trim_matches('|').split('|').map(str::trim).collect())
            .collect()
    }

    /// FR-255's setting table (stage settings only), read from the spec file
    /// itself, so the tests below judge the code against the requirement and
    /// not against the code's own list.
    fn spec_table() -> Vec<Row> {
        section_rows("\n## Setting names")
            .into_iter()
            .map(|cells| {
                assert_eq!(cells.len(), 5, "a table row has five cells: {cells:?}");
                Row {
                    name: cells[0].trim_matches('`').to_owned(),
                    stage: cells[1].to_owned(),
                    kind: cells[2].to_owned(),
                    default: cells[4].to_owned(),
                }
            })
            .collect()
    }

    /// FR-255's accounting table: each setting name with its default text.
    fn accounting_table() -> Vec<(String, String)> {
        section_rows("\n## Accounting setting names")
            .into_iter()
            .map(|cells| {
                assert_eq!(
                    cells.len(),
                    3,
                    "an accounting row has three cells: {cells:?}"
                );
                assert_eq!(cells[0], cells[1], "a counter is its own setting");
                (cells[0].trim_matches('`').to_owned(), cells[2].to_owned())
            })
            .collect()
    }

    /// The numeric default a table cell opens with.
    fn default_of(name: &str, cell: &str) -> u64 {
        cell.split_whitespace()
            .next()
            .and_then(|first| first.parse().ok())
            .unwrap_or_else(|| panic!("{name} has no numeric default: {cell}"))
    }

    /// FR-255-AC-3: the union of the limits types' setting names equals
    /// FR-255's setting and accounting tables, each name once, with the table's stage and kind, and
    /// setting one changes that field's bound and no other.
    #[trace("TC-720", "FR-255-AC-3")]
    #[test]
    fn each_limits_field_maps_to_one_distinct_setting_and_sets_only_itself() {
        let defaults = CallerLimits::default();
        let reported: Vec<Setting> = defaults
            .bounds()
            .into_iter()
            .map(|(name, _)| name)
            .collect();
        let distinct: BTreeSet<Setting> = reported.iter().copied().collect();
        assert_eq!(distinct.len(), reported.len(), "a setting is mapped twice");
        let table = spec_table();
        let accounting = accounting_table();
        let table_names: BTreeSet<&str> = table
            .iter()
            .map(|row| row.name.as_str())
            .chain(accounting.iter().map(|(name, _)| name.as_str()))
            .collect();
        assert_eq!(
            table_names.len(),
            table.len() + accounting.len(),
            "a table row is repeated"
        );
        let code_names: BTreeSet<&str> = reported
            .iter()
            .map(|setting| setting.name())
            .chain(
                defaults
                    .accounting
                    .bounds()
                    .into_iter()
                    .map(|(setting, _)| setting.name()),
            )
            .collect();
        assert_eq!(
            code_names, table_names,
            "the limits types and FR-255's table name different settings"
        );
        for row in &table {
            let setting = Setting::from_name(&row.name).expect("a table name is a setting");
            assert_eq!(setting.stage(), row.stage, "{}", row.name);
            let kind = setting
                .kind()
                .catalog_cause()
                .trim_end_matches("-exceeded")
                .replace('-', " ");
            assert_eq!(kind, row.kind, "{}", row.name);
        }
        for (setting, bound) in defaults.bounds() {
            let mut changed = defaults;
            assert!(changed.set_bound(setting, bound ^ 1));
            let before = defaults.bounds();
            let after = changed.bounds();
            assert_eq!(before.len(), after.len());
            for ((name, old), (_, new)) in before.into_iter().zip(after) {
                if name == setting {
                    assert_eq!(new, bound ^ 1, "{name} did not take its value");
                } else {
                    assert_eq!(new, old, "setting {setting} changed {name}");
                }
            }
        }
    }

    /// The builders and the settings mapping are one: each builder sets the
    /// field its setting names.
    #[trace("TC-720", "FR-255-AC-3")]
    #[test]
    fn builders_set_the_field_their_setting_names() {
        let by_builder = |limits: CallerLimits| limits.bounds();
        let mut mapped = CallerLimits::default();
        mapped.set_bound(Setting::ObservationInputBytes, 11);
        mapped.set_bound(Setting::ObservationObjects, 12);
        mapped.set_bound(Setting::ObservationValues, 13);
        mapped.set_bound(Setting::LibraryDefinitions, 14);
        mapped.set_bound(Setting::LibraryDependencyEdges, 15);
        mapped.set_bound(Setting::LibraryArtifactBytes, 16);
        mapped.set_bound(Setting::LibrarySingleArtifactBytes, 17);
        mapped.set_bound(Setting::ReplayInputBytes, 18);
        mapped.set_bound(Setting::I2InputBytes, 19);
        mapped.set_bound(Setting::I2Nodes, 20);
        mapped.set_bound(Setting::I2Edges, 21);
        mapped.set_bound(Setting::I2Occurrences, 22);
        mapped.set_bound(Setting::I2Diagnostics, 23);
        mapped.set_bound(Setting::I2WorkUnits, 24);
        mapped.set_bound(Setting::ExploreStates, 25);
        mapped.set_bound(Setting::ExploreTransitions, 26);
        mapped.set_bound(Setting::IdentityInputBytes, 27);
        mapped.set_bound(Setting::S1InputBytes, 28);
        mapped.set_bound(Setting::S1Tokens, 29);
        mapped.set_bound(Setting::S1Nodes, 30);
        mapped.set_bound(Setting::S1WorkUnits, 31);
        mapped.set_bound(Setting::S3Nodes, 32);
        mapped.set_bound(Setting::S3InputBytes, 33);
        mapped.set_bound(Setting::S3WorkUnits, 34);
        mapped.set_bound(Setting::IntakeInputBytes, 35);
        let built = CallerLimits {
            spine: SpineLimits {
                source: qsl_cst::Limits::default()
                    .with_source_bytes(28)
                    .with_tokens(29)
                    .with_nodes(30)
                    .with_work_units(31),
                checking: quire_semantic_value::checking::CheckingLimits::default()
                    .with_nodes(32)
                    .with_input_bytes(33)
                    .with_work_budget(34),
                assembly: qsl_semantics::check::AssemblyLimits {
                    identity: qsl_foundation::IdentityLimits::default().with_input_bytes(27),
                    ..qsl_semantics::check::AssemblyLimits::default()
                },
                imports: qsl_package::V2ReadLimits::default()
                    .with_artifact_bytes(19)
                    .with_nodes(20)
                    .with_edges(21)
                    .with_occurrences(22)
                    .with_diagnostics(23)
                    .with_work(24),
                model: qsl_semantics::model::accounting::ModelNormalizationLimits {
                    intake: qsl_foundation::IntakeLimits::default().with_input_bytes(35),
                    ..SpineLimits::default().model
                },
                ..SpineLimits::default()
            },
            library: LibraryLimits::default()
                .with_definitions(14)
                .with_dependency_edges(15)
                .with_artifact_bytes(16)
                .with_single_artifact_bytes(17),
            observation: ObservationLimits::default()
                .with_document_bytes(11)
                .with_objects_per_document(12)
                .with_values_per_document(13),
            explore: ExploreLimits::default()
                .with_max_states(25)
                .with_max_transitions(26),
            replay: ReplayLimits::default().with_input_bytes(18),
            ..CallerLimits::default()
        };
        assert_eq!(by_builder(built), mapped.bounds());
    }

    /// FR-255-AC-4's entry points: the settings operation takes every
    /// setting of the table, and a replay request's `stage_limits` entry
    /// takes each setting but `replay.input_bytes`, which a request does not
    /// carry and which `for_request` leaves at the library's value however
    /// the entries were built. The stage-driven half, an input that reached
    /// a limit and passes once it is raised, is
    /// `a_reached_limit_is_raised_by_the_settings_operation_and_by_a_request`.
    #[trace("TC-721", "FR-255-AC-4")]
    #[test]
    fn the_settings_operation_and_a_request_take_every_setting() {
        for setting in Setting::ALL {
            let operand = format!("{}=123456789", setting.name());
            let limits = CallerLimits::from_operands([operand.as_str()]).unwrap();
            assert_eq!(bound_of(&limits, *setting), 123_456_789, "{setting}");
            let entries: StageLimits = [(*setting, 987_654_321)].into_iter().collect();
            let replay = ReplayLimits::default();
            let requested =
                CallerLimits::for_request(&entries, AccountingLimits::default().0, replay);
            let expected = if *setting == Setting::ReplayInputBytes {
                bound_of(&CallerLimits::default(), *setting)
            } else {
                987_654_321
            };
            assert_eq!(bound_of(&requested, *setting), expected, "{setting}");
        }
    }

    /// FR-255-AC-5: a malformed operand is a usage refusal naming it.
    #[trace("TC-721", "FR-255-AC-5")]
    #[test]
    fn a_malformed_operand_is_a_usage_refusal_naming_it() {
        for (operands, operand, cause) in [
            (vec!["s9.nodes=1"], "s9.nodes=1", UsageCause::UnknownSetting),
            (
                vec!["s3.nodes=ten"],
                "s3.nodes=ten",
                UsageCause::NotAnInteger,
            ),
            (vec!["s3.nodes=-1"], "s3.nodes=-1", UsageCause::NotAnInteger),
            (
                vec!["s3.nodes=5", "s3.nodes=6"],
                "s3.nodes=6",
                UsageCause::Repeated,
            ),
        ] {
            let refusal = CallerLimits::from_operands(operands).unwrap_err();
            assert_eq!(refusal.operand, operand);
            assert_eq!(refusal.cause, cause);
        }
    }

    /// FR-255-AC-6: with nothing configured, each limit is at the default
    /// FR-255's table lists, read from the table itself.
    #[trace("TC-721", "FR-255-AC-6")]
    #[test]
    fn unconfigured_limits_are_at_the_published_defaults() {
        let defaults = CallerLimits::from_operands([]).unwrap();
        let table: Vec<Row> = spec_table();
        assert_eq!(table.len(), Setting::ALL.len());
        for row in table {
            let setting = Setting::from_name(&row.name).expect("a table name is a setting");
            assert_eq!(
                bound_of(&defaults, setting),
                default_of(&row.name, &row.default),
                "{}",
                row.name
            );
        }
        let accounting = accounting_table();
        assert_eq!(accounting.len(), AccountingSetting::ALL.len());
        for (name, default) in accounting {
            assert_eq!(
                bound_of_accounting(&defaults.accounting, &name),
                default_of(&name, &default),
                "{name}"
            );
        }
    }

    /// FR-255-AC-8: each accounting setting sets its counter alone, no
    /// operand gives the table's defaults, and `u64::MAX` is a bound.
    #[trace("TC-914", "FR-255-AC-8")]
    #[test]
    fn each_accounting_setting_sets_its_counter_alone() {
        let table = accounting_table();
        assert_eq!(table.len(), 10);
        let defaults = CallerLimits::from_operands([]).unwrap().accounting;
        for (name, default) in &table {
            assert_eq!(
                bound_of_accounting(&defaults, name),
                default_of(name, default),
                "{name}"
            );
        }
        for (name, _) in &table {
            let operand = format!("{name}=123456789");
            let set = CallerLimits::from_operands([operand.as_str()])
                .unwrap()
                .accounting;
            for (other, default) in &table {
                let expected = if other == name {
                    123_456_789
                } else {
                    default_of(other, default)
                };
                assert_eq!(
                    bound_of_accounting(&set, other),
                    expected,
                    "{name}: {other}"
                );
            }
        }
        let max = CallerLimits::from_operands(["work_units=18446744073709551615"]).unwrap();
        assert_eq!(max.accounting.0.work_units, u64::MAX);
    }

    fn bound_of_accounting(limits: &AccountingLimits, name: &str) -> u64 {
        limits
            .bounds()
            .into_iter()
            .find(|(reported, _)| reported.name() == name)
            .map(|(_, bound)| bound)
            .expect("the accounting limits report it")
    }

    /// FR-255-AC-9: the grammar, duplicate and overflow rules of the stage
    /// settings hold for the accounting names.
    #[trace("TC-914", "FR-255-AC-9")]
    #[test]
    fn a_malformed_accounting_operand_is_a_usage_refusal_naming_it() {
        for (operands, operand, cause) in [
            (vec!["depth=1"], "depth=1", UsageCause::UnknownSetting),
            (
                vec!["accounting.work_units=1"],
                "accounting.work_units=1",
                UsageCause::UnknownSetting,
            ),
            (
                vec!["work_units=ten"],
                "work_units=ten",
                UsageCause::NotAnInteger,
            ),
            (
                vec!["work_units=-1"],
                "work_units=-1",
                UsageCause::NotAnInteger,
            ),
            (
                vec!["work_units=18446744073709551616"],
                "work_units=18446744073709551616",
                UsageCause::NotAnInteger,
            ),
            (
                vec!["work_units=5", "work_units=6"],
                "work_units=6",
                UsageCause::Repeated,
            ),
        ] {
            let refusal = CallerLimits::from_operands(operands).unwrap_err();
            assert_eq!(refusal.operand, operand);
            assert_eq!(refusal.cause, cause);
        }
    }

    /// FR-255-AC-10: a `stage_limits` counter entry replaces that counter of
    /// the request's accounting limits and no other.
    #[trace("TC-914", "FR-255-AC-10")]
    #[test]
    fn a_stage_limits_counter_replaces_only_that_accounting_counter() {
        let base = crate::spine::default_accounting(77);
        let effective = AccountingLimits::with_entries(base, [(AccountingSetting::WorkUnits, 5)]);
        assert_eq!(
            effective,
            ScalarLimits {
                work_units: 5,
                ..base
            }
        );
    }
}

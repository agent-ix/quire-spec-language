// SPDX-License-Identifier: AGPL-3.0-or-later
//! The caller limits and FR-255's settings operation.
//!
//! [`CallerLimits`] holds every limits type of the stages QSL runs, each at
//! its published default. [`CallerLimits::from_operands`] is the settings
//! operation the driver CLI calls with its `--limit <name>=<value>`
//! operands, and a replay request's `stage_limits` entries are applied by the
//! same [`SettingLimits::set_bound`], so one name raises one limit at every
//! entry point.

use qsl_eval::simulation::model::ExpansionLimits;
use qsl_eval::simulation::Limits as ExploreLimits;
use qsl_foundation::{AccountingSetting, Setting, SettingLimits, SettingName, UsageRefusal};
use qsl_semantics::library::bundle::PackageLimits as LibraryLimits;
use qsl_semantics::model::observation::ObservationLimits;
use qsl_semantics::model::population::PopulationAdmissionLimits;
use quire_exact::{LimitKind as Counter, ScalarLimits};

use crate::bounds::ReplayLimits;
use crate::model_check::limits::ModelCheckLimits;
use crate::spine::{default_accounting, SpineLimits, DEFAULT_WORK_UNITS};

/// The call's accounting limits (`quire.value.accounting/v1`): each of the
/// ten counters is an [`AccountingSetting`] named `accounting.<counter>` (FR-255's
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

/// FR-298: the output-byte budget of `render`, the FR-255 setting
/// `render.output_bytes`. The default is 16777216 bytes (QSpec FR-461
/// rule 2); the bound is used as given, including zero.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RenderLimits {
    /// Inclusive count of every byte `render` returns, newlines included.
    pub output_bytes: usize,
}

impl Default for RenderLimits {
    fn default() -> Self {
        Self {
            output_bytes: 16_777_216,
        }
    }
}

impl SettingLimits for RenderLimits {
    fn bounds(&self) -> Vec<(Setting, u64)> {
        vec![(
            Setting::RenderOutputBytes,
            u64::try_from(self.output_bytes).unwrap_or(u64::MAX),
        )]
    }

    fn set_bound(&mut self, setting: Setting, bound: u64) -> bool {
        if setting != Setting::RenderOutputBytes {
            return false;
        }
        self.output_bytes = usize::try_from(bound).unwrap_or(usize::MAX);
        true
    }
}

/// FR-294: the translation work and generated code size one execution
/// backend `prepare` may use, the FR-255 settings `prepare.compile_work_units`
/// and `prepare.code_bytes`. Each bound is used as given, including zero.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PrepareLimits {
    /// Compile work units one `prepare` may charge.
    pub compile_work_units: u64,
    /// Generated code bytes one `prepare` may keep.
    pub code_bytes: u64,
}

impl Default for PrepareLimits {
    fn default() -> Self {
        Self {
            compile_work_units: 16_777_216,
            code_bytes: 67_108_864,
        }
    }
}

impl SettingLimits for PrepareLimits {
    fn bounds(&self) -> Vec<(Setting, u64)> {
        vec![
            (Setting::PrepareCompileWorkUnits, self.compile_work_units),
            (Setting::PrepareCodeBytes, self.code_bytes),
        ]
    }

    fn set_bound(&mut self, setting: Setting, bound: u64) -> bool {
        match setting {
            Setting::PrepareCompileWorkUnits => self.compile_work_units = bound,
            Setting::PrepareCodeBytes => self.code_bytes = bound,
            _ => return false,
        }
        true
    }
}

/// The FR-255 engine-table limits accepted by the settings operation.
/// `expansion` is absent until a caller supplies its required
/// `expansion.max_candidates` bound; the model-check and protocol-method
/// limits use their published defaults.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EngineLimits {
    /// EN-1's search and product budgets.
    pub model_check: ModelCheckLimits,
    /// FR-120's caller-supplied candidate bound, absent by default.
    pub expansion: Option<ExpansionLimits>,
    /// EN-1's protocol-subject live-instance bound.
    pub max_live_instances: u64,
    /// EN-1's weak-memory TSO store-buffer bound.
    pub max_store_buffer: u64,
    /// EN-1's weak-memory RA live-message bound.
    pub max_messages: u64,
}

impl Default for EngineLimits {
    fn default() -> Self {
        Self {
            model_check: ModelCheckLimits::default(),
            expansion: None,
            max_live_instances: 3,
            max_store_buffer: 4,
            max_messages: 4,
        }
    }
}

impl SettingLimits for EngineLimits {
    fn bounds(&self) -> Vec<(Setting, u64)> {
        let model = &self.model_check;
        let mut bounds = vec![
            (
                Setting::ModelCheckMaxStates,
                u64::try_from(model.limits.max_states).unwrap_or(u64::MAX),
            ),
            (
                Setting::ModelCheckMaxTransitions,
                u64::try_from(model.limits.max_transitions).unwrap_or(u64::MAX),
            ),
            (
                Setting::ModelCheckMaxAutomatonStates,
                model.max_automaton_states,
            ),
            (Setting::ModelCheckWitnessSamples, model.witness_samples),
            (Setting::ModelCheckMaxWalkSteps, model.max_walk_steps),
            (Setting::ModelCheckMaxWitnessSet, model.max_witness_set),
            (
                Setting::ModelCheckMaxRelationTuples,
                model.max_relation_tuples,
            ),
            (Setting::ModelCheckMaxLiveInstances, self.max_live_instances),
            (Setting::ModelCheckMaxStoreBuffer, self.max_store_buffer),
            (Setting::ModelCheckMaxMessages, self.max_messages),
        ];
        if let Some(expansion) = self.expansion {
            bounds.push((Setting::ExpansionMaxCandidates, expansion.max_candidates));
        }
        bounds
    }

    fn set_bound(&mut self, setting: Setting, bound: u64) -> bool {
        let model = &mut self.model_check;
        match setting {
            Setting::ModelCheckMaxStates => {
                model.limits.max_states = usize::try_from(bound).unwrap_or(usize::MAX);
            }
            Setting::ModelCheckMaxTransitions => {
                model.limits.max_transitions = usize::try_from(bound).unwrap_or(usize::MAX);
            }
            Setting::ModelCheckMaxAutomatonStates => model.max_automaton_states = bound,
            Setting::ModelCheckWitnessSamples => model.witness_samples = bound,
            Setting::ModelCheckMaxWalkSteps => model.max_walk_steps = bound,
            Setting::ModelCheckMaxWitnessSet => model.max_witness_set = bound,
            Setting::ModelCheckMaxRelationTuples => model.max_relation_tuples = bound,
            Setting::ModelCheckMaxLiveInstances => self.max_live_instances = bound,
            Setting::ModelCheckMaxStoreBuffer => self.max_store_buffer = bound,
            Setting::ModelCheckMaxMessages => self.max_messages = bound,
            Setting::ExpansionMaxCandidates => {
                self.expansion = Some(ExpansionLimits {
                    max_candidates: bound,
                });
            }
            _ => return false,
        }
        true
    }
}

/// Every limit type a caller can configure, one field per stage or engine.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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
    /// Formatter output bytes, configured independently of replay stages.
    pub format: qsl_cst::format::FormatLimits,
    /// Rendered output bytes (FR-298).
    pub render: RenderLimits,
    /// Execution-backend prepare (FR-294).
    pub prepare: PrepareLimits,
    /// FR-255's analysis-engine limits.
    pub engines: EngineLimits,
    /// The typed-view entry limit consumed by `qsl-inspect`.
    pub inspect_max_view_entries: u64,
    /// The call's accounting budgets (`quire.value.accounting/v1`).
    pub accounting: AccountingLimits,
}

impl Default for CallerLimits {
    fn default() -> Self {
        Self {
            spine: SpineLimits::default(),
            library: LibraryLimits::default(),
            admission: PopulationAdmissionLimits::default(),
            observation: ObservationLimits::default(),
            explore: ExploreLimits::default(),
            replay: ReplayLimits::default(),
            format: qsl_cst::format::FormatLimits::default(),
            render: RenderLimits::default(),
            prepare: PrepareLimits::default(),
            engines: EngineLimits::default(),
            inspect_max_view_entries: 10_000_000,
            accounting: AccountingLimits::default(),
        }
    }
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
            // Requests carry neither their own envelope limit nor the
            // formatter's limit: replay performs neither operation.
            if !matches!(
                setting,
                Setting::ReplayInputBytes | Setting::FormatOutputBytes
            ) {
                limits.set_bound(setting, bound);
            }
        }
        limits
    }

    fn parts(&self) -> [&dyn SettingLimits; 10] {
        [
            &self.spine,
            &self.library,
            &self.admission,
            &self.observation,
            &self.explore,
            &self.replay,
            &self.format,
            &self.render,
            &self.prepare,
            &self.engines,
        ]
    }
}

impl SettingLimits for CallerLimits {
    fn bounds(&self) -> Vec<(Setting, u64)> {
        let mut bounds: Vec<_> = self.parts().iter().flat_map(|part| part.bounds()).collect();
        bounds.push((
            Setting::InspectMaxViewEntries,
            self.inspect_max_view_entries,
        ));
        bounds
    }

    fn set_bound(&mut self, setting: Setting, bound: u64) -> bool {
        let parts: [&mut dyn SettingLimits; 10] = [
            &mut self.spine,
            &mut self.library,
            &mut self.admission,
            &mut self.observation,
            &mut self.explore,
            &mut self.replay,
            &mut self.format,
            &mut self.render,
            &mut self.prepare,
            &mut self.engines,
        ];
        if setting == Setting::InspectMaxViewEntries {
            self.inspect_max_view_entries = bound;
            return true;
        }
        parts.into_iter().any(|part| part.set_bound(setting, bound))
    }
}

#[cfg(test)]
mod tests {
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
        mapped.set_bound(Setting::S3RationalBits, 37);
        mapped.set_bound(Setting::IntakeInputBytes, 35);
        mapped.set_bound(Setting::FormatOutputBytes, 36);
        mapped.set_bound(Setting::RenderOutputBytes, 38);
        mapped.set_bound(Setting::InspectMaxViewEntries, 41);
        mapped.set_bound(Setting::PrepareCompileWorkUnits, 39);
        mapped.set_bound(Setting::PrepareCodeBytes, 40);
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
                    .with_work_budget(34)
                    .with_rational_bits(37),
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
            format: qsl_cst::format::FormatLimits::default().with_output_bytes(36),
            render: RenderLimits { output_bytes: 38 },
            inspect_max_view_entries: 41,
            prepare: PrepareLimits {
                compile_work_units: 39,
                code_bytes: 40,
            },
            ..CallerLimits::default()
        };
        assert_eq!(by_builder(built), mapped.bounds());
    }

    /// FR-255-AC-4's entry points: the settings operation takes every
    /// setting of the table, and a replay request's `stage_limits` entry
    /// takes each setting but `replay.input_bytes` and `format.output_bytes`,
    /// which a request does not carry and which `for_request` leaves at their
    /// defaults however the entries were built. The stage-driven half, an input that reached
    /// a limit and passes once it is raised, is
    /// `a_reached_limit_is_raised_by_the_settings_operation_and_by_a_request`.
    #[trace("TC-721", "FR-255-AC-4", "FR-255-AC-12", "FR-255-AC-13")]
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
            let expected = if matches!(
                setting,
                Setting::ReplayInputBytes | Setting::FormatOutputBytes
            ) {
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

    /// FR-255-AC-8: each accounting setting sets its counter alone, no
    /// operand gives the defaults, and `u64::MAX` is a bound.
    #[trace("TC-921", "FR-255-AC-8")]
    #[test]
    fn each_accounting_setting_sets_its_counter_alone() {
        let defaults = CallerLimits::from_operands([]).unwrap().accounting;
        assert_eq!(defaults, AccountingLimits::default());
        for (setting, _) in defaults.bounds() {
            for bound in [0, 123_456_789] {
                let operand = format!("{}={bound}", setting.name());
                let set = CallerLimits::from_operands([operand.as_str()])
                    .unwrap()
                    .accounting;
                for ((other, expected), (_, actual)) in
                    defaults.bounds().into_iter().zip(set.bounds())
                {
                    let expected = if other == setting { bound } else { expected };
                    assert_eq!(actual, expected, "{operand}: {other:?}");
                }
            }
        }
        let max =
            CallerLimits::from_operands(["accounting.work_units=18446744073709551615"]).unwrap();
        assert_eq!(max.accounting.0.work_units, u64::MAX);
    }

    /// FR-255-AC-9: the grammar, duplicate and overflow rules of the stage
    /// settings hold for the accounting names.
    #[trace("TC-921", "FR-255-AC-9")]
    #[test]
    fn a_malformed_accounting_operand_is_a_usage_refusal_naming_it() {
        for (operands, operand, cause) in [
            (vec!["depth=1"], "depth=1", UsageCause::UnknownSetting),
            (
                vec!["work_units=1"],
                "work_units=1",
                UsageCause::UnknownSetting,
            ),
            (
                vec!["accounting=1"],
                "accounting=1",
                UsageCause::UnknownSetting,
            ),
            (
                vec!["accounting.work_units=ten"],
                "accounting.work_units=ten",
                UsageCause::NotAnInteger,
            ),
            (
                vec!["accounting.work_units=-1"],
                "accounting.work_units=-1",
                UsageCause::NotAnInteger,
            ),
            (
                vec!["accounting.work_units=18446744073709551616"],
                "accounting.work_units=18446744073709551616",
                UsageCause::NotAnInteger,
            ),
            (
                vec!["accounting.work_units=5", "accounting.work_units=6"],
                "accounting.work_units=6",
                UsageCause::Repeated,
            ),
        ] {
            let refusal = CallerLimits::from_operands(operands).unwrap_err();
            assert_eq!(refusal.operand, operand);
            assert_eq!(refusal.cause, cause);
        }
    }

    /// FR-255-AC-3, FR-255-AC-4: the supplied-value ceilings of FR-277 are no
    /// setting-table rows, so the settings operation refuses them as it
    /// refuses any name that is no setting; they travel in the typed
    /// `limits.supplied` member.
    #[trace("TC-721", "FR-255-AC-3", "FR-255-AC-4")]
    #[test]
    fn the_supplied_ceilings_are_not_settings() {
        for name in [
            "supplied.admission_work_units",
            "supplied.conversion_work_units",
        ] {
            let operand = format!("{name}=5");
            let refusal = CallerLimits::from_operands([operand.as_str()]).unwrap_err();
            assert_eq!(refusal.operand, operand);
            assert_eq!(refusal.cause, UsageCause::UnknownSetting);
        }
    }

    /// FR-255-AC-10: a `stage_limits` counter entry replaces that counter of
    /// the request's accounting limits and no other.
    #[trace("TC-921", "FR-255-AC-10")]
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

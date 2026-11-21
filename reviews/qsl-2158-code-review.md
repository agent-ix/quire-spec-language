---
id: SR-202611210022442313144108508
title: "QSL-2158 PR 3285 FR-1043 finite-horizon measure profile code and Rust review"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@986e56a8e157cc8cf694ff4daed7cd2dcdc144e0 (merge-base f3aa681d7ed753a2cdd998bc24842ef38dda3c86); PR 3285 Rust diff: fsl-eval/src/probabilistic.rs, fsl-eval/src/probabilistic/key.rs, fsl-eval/src/simulation/model.rs, fsl-eval/src/simulation/model/encode/bounded.rs, fsl-analyze/src/analyze/exact.rs, fsl-analyze/src/analyze/exact/measure.rs, fsl-analyze/src/analyze/exact/undefined.rs, fsl-analyze/src/exact_probabilistic.rs, fsl-analyze/src/exact_probabilistic/{build,finite_horizon,plan,witness}.rs, fsl-analyze/src/statistical/model.rs, fsl-replay/src/probability_certificate.rs, fsl-replay/src/probability_certificate/document.rs, fsl-replay/src/probability_certificate/document/{measure,measure/bounds,fairness,key,writer}.rs, fsl-replay/src/probability_certificate/witness.rs, fsl-replay/src/probability_certificate/witness/{measure,step,undefined,bounded}.rs, fsl-replay/src/{certificate,certificate/simulation,lib,proof_result}.rs, fsl-replay/src/model_check/{counterexample,replay}.rs, fsl-replay/src/outcome/counterexample.rs, tools/fsl-probability-check/src/main.rs, tests fsl-analyze/tests/it/exact_probabilistic_item/measure_profile.rs, fsl-analyze/tests/it/exact_probabilistic_item.rs, fsl-analyze/tests/it/state_probabilistic_replay.rs, tools/fsl-probability-check/tests/it/certificate_kinds.rs, Cargo.toml, Cargo.lock"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-spec-language/FR-1043", type: references}
  - {target: "ix://agent-ix/filament-spec-language/FR-1042", type: references}
  - {target: "ix://agent-ix/filament-spec-language/FR-202", type: references}
  - {target: "ix://agent-ix/filament-spec-language/FR-203", type: references}
---

## Summary

Ticket: QSL-2158, PR 3285. This review covers the implementation at frozen
head `986e56a8e1` against the accepted normative contracts. Spec text is not
re-reviewed. The amended FR-1043, FR-1042, FR-201, FR-202 and FR-203 text was
accepted by SR-202611210022442313144108507 (findings 001 to 013 fixed, plus
FractionValue AC-11 PASS; `/tmp/qsl2158-mean-spec-review/fraction-value/report.md`).
The Rust lane follows `rust-review` and `rust-style` and the repository's
`AGENTS.md`.

The core soundness boundaries hold. The checker rebuilds the complete
eight-member product with the shared `fsl-eval` control. It recomputes each
decided payoff from the key and Bellman-checks every action and post-state.
It rederives ranks, and it rejects at the canonical locus. The vacuity check
explores the full product and returns value-free `AcceptedVacuous` with
`Exhaustive`. Numeric offers for no-activation claims reject `InequalityFails`
at the initial key. Mean refutation applies strict comparisons, the
role-direction `Malformed{kind}` rule and no-fallback hints. Native
FractionValue replay re-executes the actual step and its rewards before any
outside-window read.

The defects found are three behavioral gaps against SHALL clauses, two
duplications of existing codecs, two places where tests do not assert their
AC, and eleven low-severity cleanups.

## Verdict

**CONDITIONAL**. There are no high findings. Medium and low findings must be
fixed in this PR, per the dispatch brief.

Gates: no cargo was run, per the brief. Remote focused acceptance (33 exact
tests listed in `/tmp/qsl2158-fr1043-focused-986e.txt`) is pending on draco
and is not claimed here. The most recent recorded compile (`ec6`) predates
this head. `rustfmt --edition 2021 --check` on every changed `.rs` file at
`986e56a8e1` shows no diffs (read-only). The `model.rs` import reordering fixes
fmt drift that was already present at base. The PR does not change
`.github/workflows`.

Verified clean, with no findings:
- The FR-1043-AC-1 to AC-11 and FR-202-AC-27 to AC-30 behavior paths are
  implemented.
- FR-203-AC-10 vacuity record: `Proved{Exhaustive, Certified}`, no
  `ExactValue`, document `{"mode":"vacuous"}` with empty values. The pinned
  FSpec `Result` schema conformance test is present.
- The P95 `24233/25000` and 2 ms one-path refutation are asserted.
- TC-2065 `quantile 0.4 <= 3` shifted `11/135` is accepted in a separate
  process, and level 0.5 is refuted.
- The removed `#[ignore]` markers are justified, and no tests were deleted or
  weakened.
- FSpec pin `d6d06609` appears only in `Cargo.toml`/`Cargo.lock`.
- No `unsafe` code, `todo!` or `unimplemented!` was introduced. No reachable
  panic was found.
- A quantile over a fraction is refused by S3 (`QuantileOverFraction`), so the
  fraction `activated=false` flag cannot fabricate vacuity.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | A partial `lower` mean refutation still enumerates the complete product before checking the named subset. FR-1043 requires expanding only the named states and their successors under the checker's limits. A product that exceeds `states`/`edges` but whose named lower subset fits returns `incomplete` instead of an accepted refutation. | fsl-replay/src/probability_certificate/document/measure/bounds.rs:306 |
| FND-002 | medium | An undefined reward on an operation with an object-typed parameter skips FR-187 admission (`let Some(domains) = domains else { continue }`). It is then flattened to an internal BROKEN expansion fault at runtime, not FR-202 `Undefined` evidence as FR-1043 requires ("an undefined reward ... SHALL settle by FR-202's Undefined evidence"). The coder's preflight-refusal grounding (frames.rs:92-116, symbolic.rs:409-428) covers only operations whose parameters all have finite domains. | fsl-eval/src/simulation/model/probabilistic.rs:567 |
| FND-003 | medium | The FR-1043 rule that workload ratios with both activating and never-activating initial keys follow FR-201's per-key vacuity is not implemented. The checker refuses any non-singleton initial set as `Unsupported`, and the producer returns `not_yet`. No test covers it. | fsl-replay/src/probability_certificate/document/measure.rs:466 |
| FND-004 | medium | The `fsl-eval` key encoder re-states the UnitId descriptor and decimal-integer wire grammar (`["declared",node]` / `["compound",[{unit,exponent}]]`, `unwrap_or_default`) that `fsl-replay` `writer::Text::unit`/`integer` already define. That leaves two owners of one encoding. | fsl-eval/src/probabilistic/key.rs:111 |
| FND-005 | medium | `bounds.rs` hand-implements TransitionIdentity choice decoding and encoding (`TransitionShape`, the `Transition` encoder, ad-hoc `starts_with('{')` checks) instead of reusing the `ModelTransition` canonical codec. The test helper `mean_bound` mirrors that hand form. | fsl-replay/src/probability_certificate/document/measure/bounds.rs:121 |
| FND-006 | medium | The vacuity test asserts only `ProductState(_)` for the numeric no-activation rejection, not the initial complete product key the AC requires. It does not assert the `inconclusive CertificateRejected` settlement. FR-203-AC-11's cancellation, caller-limit and engine-only no-acceptance cases for vacuity are not exercised. | fsl-analyze/tests/it/exact_probabilistic_item/measure_profile.rs:858 |
| FND-007 | medium | The FR-1043-AC-7 limits test asserts only checker `Err(Limit)` values at zero limits. It never asserts the settled `incomplete` record with the actual limit and no truth value. It does not exercise `scalar_bits`/RationalBits or a limit reached during expansion (`states: 0` stops at the read-time row-count check). | fsl-analyze/tests/it/exact_probabilistic_item/measure_profile.rs:459 |
| FND-008 | low | The same MeasureError-to-ProbabilityDocumentError mapping is written three times (`scalar_error`, `measure_error`, `scalar_failure`). The ExactProbLimits-to-ProbabilityDocumentLimits literal gains five more copies (7 to 12), and the measure-form `matches!` predicate is repeated across six dispatch sites. | fsl-replay/src/probability_certificate/document/measure.rs:117 |
| FND-009 | low | The checker's enumeration and the witness terminal-stutter replay hand-build zero-reward lists with different filters (subject-model filter versus all declarations, one entry per declaration). EN-5 uses `ModelSystem::zero_rewards()`, the owner of stutter rewards. | fsl-replay/src/probability_certificate/document/measure.rs:633 |
| FND-010 | low | The canonical full-product-action order is re-stated as a local tag table (`""`/`"identity"`/`"post-state"`/`"terminal-stutter"` plus raw bytes) instead of ordering by `ProductAction::to_bytes()`. | fsl-replay/src/probability_certificate/document/measure/bounds.rs:420 |
| FND-011 | low | The lower-rank check does a linear `document.ranking.iter().find` inside the per-value loop. That is quadratic work, uncharged to the caller's work meter, although `ranking` is sorted and `checked_values` uses `binary_search`. | fsl-replay/src/probability_certificate/document/measure/bounds.rs:381 |
| FND-012 | low | `weighted_value` inlines `validate_probability_mass`'s non-negative, unit-mass rule as a second implementation instead of reusing it. | fsl-analyze/src/exact_probabilistic/finite_horizon.rs:1147 |
| FND-013 | low | The public `measure_probability_product_key` path maps structural mismatches (Formula/Deadline control, a missing reward descriptor) to `Error::ReservationDenied{requested:0}`. A malformed caller input is therefore reported as a budget denial. | fsl-eval/src/probabilistic/key.rs:182 |
| FND-014 | low | New unchecked indexing: `state.choices[index]` in exact mean refutation, and `initial[0]`/`initial[1]` in the ratio check. Neither is reachable as a panic, but rust-style prefers `get`/destructuring, as the adjacent code uses. | fsl-replay/src/probability_certificate/document/measure.rs:2306 |
| FND-015 | low | Redundant full enumerations charged to the caller budget. The ratio check enumerates the product a third time only to take `graph.initial`. The producer's vacuity route builds a complete checker document only to clear its ranking in `into_vacuous`. | fsl-replay/src/probability_certificate/document/measure.rs:1829 |
| FND-016 | low | The producer's `Arithmetic::retain` charges storage for every intermediate `mass`/`term`/`sum` cumulatively and never releases it. Producer working storage is overstated relative to the checker, so EN-5 can settle `StorageBytes` incomplete on an item the checker accepts. | fsl-analyze/src/exact_probabilistic/finite_horizon.rs:699 |
| FND-017 | low | The P95 FR-1043-AC-4 test does not assert the AC's "denominator is 1 at the initial key" or "`threshold` 5 in its unit". | fsl-analyze/tests/it/exact_probabilistic_item/measure_profile.rs:579 |
| FND-018 | low | The public `ProbabilityCertificatePart::Post`/`Reset` variants are added with no producer or consumer in this PR (speculative API). In `fsl-eval/src/probabilistic.rs`, the public `MeasurePlan` derives no standard traits, and `mod tests` sits mid-file before `MeasureComparison`/`MeasurePayoff`. | fsl-replay/src/probability_certificate.rs:595 |

## Coverage

- Methods: `code-review` with its `rust-review` lane folded in, under the
  `rust-style` baseline (API, implementation, safety, wire and testing
  references). The duplication and vendoring check found no vendored files;
  the in-repo duplications are FND-004/005/008/009/010/012. The ceremony
  check found none.
- Checked and clean: identity refused before enumeration. Strict canonical
  decode with depth limit 1. Values/ranking exact coverage. `RankBroken` at the
  first differing key. Shifted range `[-level, 1-level]`. Tight-activation
  search. Ratio `ThresholdMissed`. Vacuity precedence. Mean strict refutation
  and equality. No-fallback hints. Witness-policy parity. Subsystem and paths
  mutually exclusive. Path set refused for a mean. Positive actual draw.
  Residual post-state policy required. FractionValue locus
  (`h+1`, actual post-state, pre-step control/sums). `ReplayParity` on forged
  locus, position, control, sums or cause. Cancellation polls on every loop.
- Not run: cargo build, tests, clippy and deny (per the brief). Runtime
  results are pending in the parent's remote focused batch.

## Disposition pass 1

Latest reviewed source: `1bef7ca58bebe9972e3fe74189d578e060034722`; actual merge-base `529282e9dc28cc6be8b382ae634636e3b4fe4a14`. One independent Codex reviewer (GPT-6 family as supplied by harness; no more specific model version exposed), user-directed replacement of initial reviewer. Initial findings/history above are retained verbatim.

**Latest verdict: FAIL; not mergeable**. New high FND-020 blocks compilation; new medium FND-019 is still-open. No focused test executed. Original FND-001 and003–018 fixed; FND-002 rejected with admission grounding.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-019 | medium | Lower enumeration now drops every unnamed frontier and consequently rejects a valid named descendant behind it as Malformed(values). FR-1042 permits initial plus any subset. Extend the existing partial-lower fixture (root1/3 plus success2/3 branch) with the reachable all-fail terminal (fraction collecting control advanced to remaining0/complete, count0/3, value0, rank0), omitting the intermediate all-fail rows. The added terminal satisfies its exact payoff/rank and changes no named Bellman equation, but it is never inserted in graph.states and bounds.rs returns MalformedMember("values"), mapped by native replay to CertificateRejected{rule:Malformed, at:Member("values")}. Enumerate/check every named state and its actual successors without requiring a named path from the initial. | fsl-replay/src/probability_certificate/document/measure.rs:581; fsl-replay/src/probability_certificate/document/measure/bounds.rs:269 |

| FND-020 | high | Exact-head focused build fails E0425: ProbabilityDocumentLimits::for_exact references ExactProbLimits without importing the type. The checker owning crate cannot compile, so the first29-profile command exits101 with zero tests and commands2–14 are held. Also remove the reported unused MeasureError import in measure.rs7. Parent raw log /private/tmp/qsl-3285-1bef-1.log. | fsl-replay/src/probability_certificate/document.rs:229 |

## Dispositions

Round 1. `fixed` records inspected source resolution; execution evidence remains separately qualified below.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Partial lower now stops at immediate unnamed frontier, removing full-product enumeration; the separate disconnected-row regression is FND-019. |
| FND-002 | rejected | The originally plausible path has no admitted runtime reproducer. Object-domain preflight skip is real, but mandatory Linked Definedness still checks every typed reward; literal mod0 fails Nonzero, ordinary reference.field fails Composite typing, and deref is forbidden StateReadInReward. The corrected remote10970 probe compiled/selected one test but failed at admission, so it is not BROKEN runtime evidence. Parameter identity equality is a defined positive control. This rejection is bounded to the alleged paths; it is not a universal soundness proof. |
| FND-003 | fixed | Each actual initial index is independently checked; zero/zero roots are omitted only alongside activation, all-zero numeric roots reject; sealed binding and exact entry matching protect settlement. |
| FND-004 | fixed | Evaluator and replay writer use the single foundation scalar_wire integer/unit owner. |
| FND-005 | fixed | Hint decoding and reconstruction use ModelTransition construction/canonical encoding, compared byte-exactly before matching schedules. |
| FND-006 | fixed | Numeric no-activation rejection checks the exact initial key; settlement units assert same-locus CertificateRejected and engine-only failure; actual vacuity cases assert typed cap/cancellation incomplete without evidence. |
| FND-007 | fixed | Nonzero expansion edge1 and RationalBits4 checker limits are exercised; analyzer settlement asserts exact state/rational limit cause and no evidence. |
| FND-008 | fixed | One From<MeasureError> owner, for_exact limit projection, and CheckedProbabilisticClaim::bounded_measure routing replace repeated rules. Specialized cyclic/timed projections retain their distinct meanings. |
| FND-009 | fixed | Checker enumeration and stutter replay call the existing zero_rewards owner. |
| FND-010 | fixed | Canonical choice selection sorts ProductAction::to_bytes octets. |
| FND-011 | fixed | Rank lookup uses binary_search and charges logarithmic GraphEdge work. |
| FND-012 | fixed | weighted_value and weighted_interval use validate_probability_mass; exact validation receives the actual Arithmetic meter. |
| FND-013 | fixed | Key structure is checked before encoding as Unsupported; encoder structural errors use Serialize. ReservationDenied remains a budget refusal. |
| FND-014 | fixed | Choice access uses get and the ratio parts use slice destructuring. |
| FND-015 | fixed | Ratio reuses numerator/denominator initials; unchecked vacuous factory computes the objective without discarded full graph/ranks. |
| FND-016 | fixed | Arithmetic uses checkpoint/release for scratch and retains selected final values; temporary topological storage is released before induction. |
| FND-017 | fixed | P95 checks threshold5 in the checked reward unit and initial denominator1, preserving package recheck. |
| FND-018 | fixed | Unused Post/Reset variants removed; MeasurePlan derives Clone/Debug/Eq/PartialEq and test module follows production definitions. |
| FND-020 | still-open | Actual exact-head remote compile fails E0425: ExactProbLimits is not imported for ProbabilityDocumentLimits::for_exact at document.rs229; no tests execute. |
| FND-019 | still-open | New acceptance regression: disconnected named rows behind an unnamed frontier are rejected despite being a valid subset; see new finding and static adverse case. |

### Round 1 source excerpts

FND-001 after_excerpt:

```rust
        if named.is_some_and(|named| !named.contains(&key)) {
            // A lower certificate assigns zero to this frontier key. Its
            // descendants and terminal payoff are outside the named proof.
            states.insert(
                key,
                State {
                    terminal: None,
                    rank: 0,
                    choices: Vec::new(),
                    actions: Vec::new(),
                    activated: false,
                },
            );
            continue;
        }
        let terminal = plan
            .payoff
```

FND-003 after_excerpt:

```rust
        let Some(coverage) = &self.initial_values else {
            return true;
        };
        let mut offered = entries.iter();
        for (initial, value) in &coverage.values {
            let Some(value) = value else {
                continue;
            };
            let Some(entry) = offered.next() else {
                return false;
            };
            if entry.initial != *initial
                || entry.binding != coverage.binding
                || entry.extremum.is_some()
```

FND-004 after_excerpt:

```rust
    }
    pub(super) fn unit<S: Sink + ?Sized>(
        &self,
        unit: Option<&filament_exact::UnitId>,
        writer: &mut Writer<'_, S>,
    ) -> Result<(), Error> {
        self.poll()?;
        fsl_foundation::scalar_wire::unit(unit, writer, |value, writer| self.integer(value, writer))
    }
```

FND-005 after_excerpt:

```rust
        let transition = filament_semantic_value::model_transition::ModelTransition::new(
            wire.operation,
            wire.receiver.map(|value| value.get().to_owned()),
            wire.arguments
                .into_iter()
                .map(|value| value.get().to_owned())
                .collect(),
            limits,
        )
        .map_err(|_| ProductActionError::Schedule)?;
        let canonical = transition
            .canonical_bytes(limits)
            .map_err(|_| ProductActionError::Schedule)?;
        if canonical != bytes {
```

FND-006 after_excerpt:

```rust
            let checked = check(&world, name, &numeric);
            assert!(matches!(&checked, Ok(ExactProbabilityCertificateCheck::Rejected(ProbabilityCertificateRejection {
                rule: ProbabilityCertificateRule::InequalityFails,
                at: ProbabilityCertificateLocus::ProductState(at)
            })) if at == &expected), "numeric no-activation {scheduler}/{name}: {checked:?}; expected initial {expected:?}");
```

FND-007 after_excerpt:

```rust
        )
        .remove(0);
        let terminal = record
            .terminal()
            .unwrap_or_else(|| panic!("{name}: {:?}", record.settlement));
        assert_eq!(
            terminal.value(),
            &TerminalValue::Incomplete(IncompleteCause::LimitReached { limit, value }),
            "{name}: {:?}",
            record.settlement
        );
        assert!(terminal.exact_certificate_document().is_none());
        assert!(
            record.exact_probabilistic.is_none() && record.probabilistic_counterexample.is_none()
        );
```

FND-008 after_excerpt:

```rust
    pub fn for_exact(limits: &ExactProbLimits, accounting: filament_exact::ScalarLimits) -> Self {
        Self {
            bytes: limits.document.bytes,
            storage_bytes: limits.document.storage_bytes,
            states: limits.model.limits.max_states,
            edges: limits.model.limits.max_transitions,
            scalar_bits: limits.max_rational_bits,
            accounting,
        }
    }
}
```

FND-009 after_excerpt:

```rust
                != system.state_key(&step.post).ok().map(|(bytes, _)| bytes)
        {
            return Err(ProbabilityWitnessError::Refused);
        }
        let rewards = system.zero_rewards();
        let Some(projected) = project(state, &rewards, meter, storage)? else {
            return Err(ProbabilityWitnessError::Refused);
        };
        *edges = edges
            .checked_add(1)
```

FND-010 after_excerpt:

```rust
                limits.storage_bytes,
                u64::try_from(capacity.saturating_mul(2)).unwrap_or(u64::MAX),
            )?;
            let bytes = action
                .as_ref()
                .map(crate::ProductAction::to_bytes)
                .transpose()
                .map_err(|_| ProbabilityDocumentError::Malformed)?
                .unwrap_or_default();
            choices.push((bytes, index));
        }
        choices.sort_by(|a, b| a.0.cmp(&b.0));
        let mut satisfying = None;
```

FND-011 after_excerpt:

```rust
        if lower
            && document
                .ranking
                .binary_search_by(|row| row.0.cmp(key))
                .ok()
                .and_then(|index| document.ranking.get(index))
                .map(|row| row.1)
                != Some(state.rank)
        {
```

FND-012 after_excerpt:

```rust
fn weighted_value(
    edges: &[&ProbEdge],
    values: &[Rational],
    operations: &mut Arithmetic<'_>,
) -> Result<Rational, FhError> {
    validate_probability_mass(edges, operations)?;
    let mut sum = rational_zero();
    for edge in edges {
        operations.visit()?;
        let value = values.get(edge.to.index()).ok_or(FhError::InvalidState)?;
        let term = operations.arithmetic(filament_exact::RationalArithmetic::Multiply(
            &edge.probability,
            value,
```

FND-013 after_excerpt:

```rust
            binding,
            model: model.root(),
            control,
            accumulator,
            pending: pending.as_ref().map(|pending| pending.root()),
            reward: reward.map(|reward| fsl_eval::probabilistic::key::RewardKey {
                model: &reward.model,
                name: &reward.name,
                unit: reward.unit.as_ref(),
            }),
            budget: &scope,
            max_bytes: identity.input_bytes,
        };
        key.validate().map_err(ModelKeyError::Unsupported)?;
```

FND-014 after_excerpt:

```rust
        }
        let [(numerator_graph, numerator_values), (denominator_graph, denominator_values)] =
            checked_parts.as_slice()
        else {
            return Err(ProbabilityDocumentError::Malformed);
        };
        if numerator_graph.initials != denominator_graph.initials {
            return Err(ProbabilityDocumentError::Malformed);
        }
```

FND-015 after_excerpt:

```rust
    Ok(MeasureProbabilityDocument {
        identity: item.clone(),
        payoff: MeasurePayoffKind::Satisfied,
        values: Vec::new(),
        ranking: Vec::new(),
        objective: selected.objective,
        conditional: Some(Box::new(Conditional::Vacuous)),
    })
```

FND-016 after_excerpt:

```rust
            let checkpoint = operations.storage_used();
            if let Some((_, current)) = &selected {
                operations.retain(current)?;
            }
            let value = weighted_value(edges, &values, operations)?;
            // Both candidate values remain live through selection.
            operations.retain(&value)?;
            let replace = match selected.as_ref() {
                None => true,
                Some((_, current)) => operations.prefer(objective, &value, current)?,
            };
            if replace {
```

FND-017 after_excerpt:

```rust
    let initial = denominator["values"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["state"] == initial["state"])
        .unwrap();
    assert_eq!(initial["value"], json!({"numerator":"1","denominator":"1"}));
```

FND-018 after_excerpt:

```rust
/// The checked component of a conditional probability certificate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProbabilityCertificatePart {
    /// The satisfied-and-activated numerator.
    Numerator,
    /// The activation denominator.
    Denominator,
}
impl ProbabilityCertificatePart {
    /// Canonical published part spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Numerator => "numerator",
            Self::Denominator => "denominator",
        }
```

### Round 1 scope and verification

No cargo, local tests, clippy, deny, baseline, Kani or sweep was run. Parent reports exact1bef7ca58bebe9972e3fe74189d578e060034722 first29-profile command COMPILE101/ZERO tests: E0425 document.rs229 missing ExactProbLimits import, plus unused MeasureError at document/measure.rs7. Raw /private/tmp/qsl-3285-1bef-1.log. Commands2–14 held/not run; no42-test acceptance. Original986e33 never ran/HOLD and is superseded. Upstream monitor_root23 is reader-only evidence, not acceptance. Remote10970 admission failure is only prior post-rebase compile/probe evidence.

The actual current PR diff preserves main exports, boxed task transitions, refinement guards/time invariants, protocol replay, call_failure propagation and unavailability removals. IR20a4dfbf/core-data424ba49 remain pinned; only authorized FSpecd6d066 replaces its prior pin. No workflow lane changes; git diff --check passes. No applicable AssuranceProfile was found. No new source edits, test deletion, vendoring or speculative owner API retained. FractionValue replays the actual post-step state at h+1 with PRE undecided control/accumulator before outside-window predicate access.

Scope entries below record criteria examined against implementation; AC12 is context_only at full generic scope.

```yaml
scope:
- id: FR-1042-AC-12
  path: spec/functional/FR-1042-certify-value-bounds-with-lower-and-upper-certificates.md
  role: context_only
  excerpt: For a bounded fraction mean over an initial terminal state with a false
    fraction predicate and window 2, the base's only continuation is terminal stutter
    and its value is 0. An `upper` minimum document with every value 0 and no policy
    rows, and a `lower` maximum document with every value 0, the rederived remaining-horizon
    ranks and no policy rows, are accepted. The checker retains the existing stutter
    action at each undecided key; no fabricated operation identity or new wire form
    is required.
- id: FR-1042-AC-13
  path: spec/functional/FR-1042-certify-value-bounds-with-lower-and-upper-certificates.md
  role: examined
  excerpt: At an intermediate state with two enabled post-state choices `a` and `b`,
    their successors reach event success with probability `1/4` and `3/4`, respectively.
    With correct successor values and ranks, a minimum `upper` initial bound `1/2`
    with no hint is accepted selecting `a`, and a maximum `lower` bound `1/2` with
    no hint is accepted selecting `b`. Initial bounds `1/8` for the upper and `7/8`
    for the lower reject `InequalityFails` there because no action satisfies. Post-state
    selection needs no fabricated TransitionIdentity row.
- id: FR-1042-AC-14
  path: spec/functional/FR-1042-certify-value-bounds-with-lower-and-upper-certificates.md
  role: examined
  excerpt: In AC-3, minimum `upper` `s` `1/4` with hint `go` rejects `InequalityFails`
    although `wait` satisfies, and the same document without that hint accepts selecting
    `wait`. A nonexistent hint rejects `InequalityFails` even when an enabled action
    satisfies. Maximum `lower` `s` 1 with hint `wait` rejects `RankBroken`; removing
    the hint does not accept it, since no action satisfies both inequality and progress.
    When several actions satisfy every condition, an omitted hint selects the first
    in canonical full-product-action order.
- id: FR-1043-AC-1
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: '`Retry`''s undelivered state `u` has `try`, delivering with `1/2` and
    otherwise staying; delivered is terminal. Under its workload, `quantile 0.5 of
    (steps from holds(true) until holds(delivered) within 4) <= 2` settles `proved`,
    `ExactValue{3/4}`, with a `conditional` `ratio` certificate whose numerator (payoff
    `satisfied`, `comparison` `"le"`, `threshold` 2) has values `3/4`, `1/2`, 0, 0
    at `u` with active amounts 0, 1, 2 and above the bound and ranks 4, 3, 2, 1, and
    whose denominator (payoff `activated`) is 1 there; `fsl-probability-check` accepts
    it, and rejects it `RankBroken` with the ran'
- id: FR-1043-AC-2
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: With `try` split into `fast` (`1/2`) and `slow` (`1/4`), under every scheduler
    `quantile 0.4 of (…) <= 2` settles `proved` with an `exact` `shifted` document
    (`level` `2/5`) whose values are `3/80`, `−3/20`, `−2/5`, `−2/5`; `quantile 0.5
    of (…) <= 2` settles `refuted` (minimum shifted value `−1/16`, `Pr(M <= 2) = 7/16`
    under `slow`); the level-0.4 document with `3/80` replaced by `fast`'s `9/40`
    is rejected `InequalityFails` at the initial key.
- id: FR-1043-AC-3
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: 'Over AC-2''s model under every scheduler, `quantile 0.6 of (…) >= 2` (claim
    `Pr(M < 2 | act) < 3/5`) settles `proved`: the `shifted` document (`comparison`
    `"lt"`) has maximum initial value `−1/10` (through `fast`) and `−3/5` at every
    other undecided state; `quantile 0.5 of (…) >= 2` settles `refuted`, its maximum
    0 attained by `fast` with an activated state reachable.'
- id: FR-1043-AC-4
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: '`P95` over FR-185-AC-1''s `Service` under `Steady` at `5 ms` (FR-741-AC-2,
    FR-203-AC-1) settles `proved`, `ExactValue{24233/25000}`, with a `conditional`
    `ratio` certificate whose parts are documents of this profile (`kind` `"accumulate"`,
    `reward` the checked `duration`, `threshold` 5 in its unit) and whose denominator
    is 1 at the initial key; `fsl-probability-check` accepts it from the emitted package
    and rejects it with one numerator value altered. At `2 ms` the item settles `refuted`
    with FR-202-AC-1''s one-path set.'
- id: FR-1043-AC-5
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: Under `Retry`'s workload, `mean of fraction holds(delivered) over [0, 2]
    >= rational(2,5)` settles `proved`, `ExactValue{5/12}`, with a `fraction` document;
    at `rational(1,2)` it settles `refuted` with FSpec FR-413 subsystem evidence containing
    an independently accepted `exact` `fraction` certificate of `5/12 < 1/2`, checked
    for refutation rather than the proof threshold. The comparison event `probability
    >= 0.7 [M <= 2]` over AC-1's measure settles `proved`, `ExactValue{3/4}`, with
    an `event` document.
- id: FR-1043-AC-6
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: FR-201-AC-51's finite-horizon quantile item over the cyclic `link` model
    under every scheduler (`Pr(M <= 3) = 13/27` for `steps from holds(true) until
    holds(m.delivered) within 4`), written at level 0.4, settles `proved` with shifted
    initial value `11/135` and a document `fsl-probability-check` accepts in a separate
    process; written at level 0.5 it settles `refuted`.
- id: FR-1043-AC-7
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: AC-1's numerator with `window`, `threshold`, `comparison`, `reward`, `unit`,
    `level` or `payoff` changed refuses objective mismatch; a key whose accumulator
    amount or phase is altered is not an alias and fails coverage; a `shifted` value
    below `−level` is rejected `Malformed` at `values`. Pre-cancellation and exhaustion
    of each caller limit during expansion, payoff evaluation or exact sums settle
    `incomplete` with the actual limit and no truth value.
- id: FR-1043-AC-8
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: With AC-1's `try` split into `fast` (`1/2`) and `slow` (`1/4`), `mean of
    fraction holds(delivered) over [0, 2]` has minimum `11/48` and maximum `5/12`.
    Under every scheduler, `>= 1/2` is refuted by an accepted exact minimum or FR-1042
    `upper` on the minimum with initial bound `1/4` and policy `slow`; `<= 1/3` is
    refuted by an accepted exact maximum or `lower` on the maximum with initial bound
    `2/5` and policy `fast`. Both reuse subsystem evidence and are independently accepted
    by FR-202's checked-subject mean-certificate replay entry.
- id: FR-1043-AC-9
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: AC-5's exact workload mean `5/12` at `>= 5/12` and `<= 5/12`, a valid workload
    `upper` bound `1/2` at `>= 1/2`, and a valid `lower` bound `2/5` at `<= 2/5` each
    reject `ThresholdMissed` at the initial key and settle `inconclusive`, `CertificateRejected`.
    In AC-8, a valid `lower` minimum document with initial bound `1/5` at `>= 1/2`
    and a valid `upper` maximum document with bound `1/2` at `<= 1/3` each reject
    `Malformed` at `kind` with the same settlement. Identity/base mismatch retains
    its typed refusal; an invalid value rejects `InequalityFails` with `CertificateRejected`;
    a disagreeing witne
- id: FR-1043-AC-10
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: FR-203-AC-10's no-activation quantile under a resolved workload and under
    every scheduler, with and without an admitted fairness set, settles certified
    `Exhaustive` with an independently accepted conditional `vacuous` certificate,
    no `ExactValue`, shifted-value proof or numeric entry. Offering an all-zero `exact`
    `shifted` document under every scheduler, or a workload `ratio` with denominator
    0, for that no-activation claim rejects `InequalityFails` at the initial complete
    product key and settles `inconclusive`, `CertificateRejected{rule, at}`, without
    numeric acceptance or `ExactValue`. A var
- id: FR-1043-AC-11
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: '`Coin` initially has bound object `c` with `c.v = 0`; its only operation
    `drop(b)` draws `b` uniformly from `{0,1}`, deleting `c` when `b = 0`, and declares
    reward `R = 0` for both supported draws. Under its resolved workload, `mean of
    fraction holds(c.v = 0) over [0,0] weighted by R >= 1/2` reaches undefined `division-by-zero`
    on the actual `drop(0)` step of probability `1/2`: the final tentative weighted
    sums are `(0,0)`, and the `FractionValue` location is position 1 with the actual
    post-state lacking `c`, pre-step undecided control `remaining = 1` and pre-step
    sums `(0,0)`. Native replay r'
- id: FR-202-AC-27
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: Through the checked-subject mean-certificate replay entry, FR-1043-AC-5's
    workload mean `5/12` refutes `>= 1/2` using subsystem evidence containing the
    independently accepted `exact` `fraction` certificate; the same document checks
    in its refutation role despite missing the proof threshold. An unchecked document,
    an unreplayed engine value or a path set yields no accepted mean-refutation evidence.
- id: FR-202-AC-28
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: FR-1043-AC-8's minimum `11/48` with an accepted `upper` bound `1/4` refutes
    every-scheduler `>= 1/2`; its maximum `5/12` with an accepted `lower` bound `2/5`
    refutes every-scheduler `<= 1/3`. A valid `lower` minimum document with initial
    bound `1/5` at `>= 1/2`, and a valid `upper` maximum document with initial bound
    `1/2` at `<= 1/3`, each reject `Malformed` at `kind` and settle `inconclusive`,
    `CertificateRejected`; their conditions hold but their refutation roles are wrong.
- id: FR-202-AC-29
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: AC-27's exact `5/12` at `>= 5/12` or `<= 5/12`, and a valid workload `upper`
    bound `1/2` at `>= 1/2`, each reject `ThresholdMissed` at the initial key and
    settle `inconclusive`, `CertificateRejected`. Changing identity or base payoff/optimization
    retains the typed refusal; invalid certificate values settle `inconclusive`, `CertificateRejected{InequalityFails,
    at}`; a disagreeing witness policy settles `inconclusive`, `ReplayParity`; cancellation
    or a caller limit preserves its typed incomplete result without accepted evidence.
- id: FR-202-AC-30
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: 'A mean minimum refutation with a slack `upper < θ` for which both the
    optimizer''s attaining action and a different canonical-first action satisfy every
    FR-1042 condition emits a witness matching the independently checked policy: with
    no hint it uses the canonical-first satisfying full-product action; with a representable
    valid hint it uses that hinted action. Both replay as refuted. An old optimizer
    witness disagreeing with the checked selection settles `inconclusive`, `ReplayParity`;
    implicit stutter or post-state selection uses the same rule without a fabricated
    policy identity.'
- id: FR-203-AC-10
  path: spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md
  role: examined
  excerpt: 'The untimed claim `quantile 0.5 of (steps from holds(false) until holds(true)
    within 4) <= 2`, checked separately under a resolved workload and under every
    scheduler (with and without an admitted fairness set) has no activating state
    in the complete reachable product. Native checking independently accepts its `Conditional`
    `Vacuous` certificate and settlement returns `proved`, `closed-scope`, `Certified`,
    `Exhaustive`, category success, carrying that certificate with empty `values`
    and `conditional` exactly `{"mode": "vacuous"}`. The emitted result passes FR-331''s
    contract with basis `{"type":'
- id: FR-203-AC-11
  path: spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md
  role: examined
  excerpt: Changing AC-10's activation predicate to `holds(true)` makes the offered
    vacuous certificate reject `InequalityFails` and settle `inconclusive`, `CertificateRejected`,
    without vacuity acceptance. Offering an otherwise well-shaped `exact`, `ratio`
    or `optimum` certificate on `exhaustive`, or generic `evidence` with or without
    its permitted vacuous certificate, refuses the FR-331 result contract. An `exact-value`
    basis missing its value, using scalar zero or denominator zero refuses; AC-1's
    ordinary exact result with a valid `Rational` value remains admitted. An engine-only
    assertion, partial ex
- id: FR-203-AC-12
  path: spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md
  role: examined
  excerpt: '`Retry` has `Msg.delivered`, terminal when delivered, and the resolved
    `Steady` workload whose `fast` step delivers with probability `1/2` and whose
    `slow` weight is 0. With request initial states ordered as index 0 `{delivered:
    true}` and index 1 `{delivered: false}`, the same admitted binding `m`, and `quantile
    0.5 of (steps from holds(not m.delivered) until holds(m.delivered) within 4) <=
    2`, the independent conditional certificate check accounts for both pairs: index
    0 has unreachable activation with numerator and denominator 0, while index 1 activates
    immediately with numerator `3/4`, den'
```

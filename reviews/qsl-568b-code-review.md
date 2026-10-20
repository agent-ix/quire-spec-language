---
id: SR-13595
title: "Code review of quire-spec-language PR #1116: S3 typing of probabilistic claims and their obligation identity (FR-186)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@20dafcee1dbfa157ea9f9572071e16f6a82d96ea; PR #1116 diff against origin/main (merge base 4e5825762): qsl-semantics/src/check/probabilistic/claim.rs (new), qsl-semantics/src/check/{probabilistic,probabilistic/model,temporal_clause,hyper_clause,mod,check,assemble}.rs, qsl-semantics/tests/it/probabilistic_declarations.rs, qsl-replay/src/spine.rs, qsl-cst/tests/it/probabilistic.rs; against FR-186, FR-185, TC-621, ADR-024 PM-7/PM-8/PF-10, ADR-013 R-04 and O-09"
review_set: subset
---
# Code review of quire-spec-language PR #1116

## Summary

Ticket: QSL-568 (E17a, reopened for the FR-186 half). Code review with the
Rust lane (dev-tools `rust-review`) folded in.

The PR replaces the blanket `not-yet-implemented` refusal of a
`probabilistic` claim with a real S3 check. `claim.rs` resolves the subject
(a `model` alias, or a refused trace subject), the profile, the scheduler
(a workload of the subject's model, or `every scheduler` with its fairness
set) and the `over` parameter. It then types the four FR-186 forms, records a
`CheckedProbabilisticClaim` on `CheckedProbabilisticModel::claims()`, and
builds a `ProbabilisticObligationIdentity` from length-prefixed normal-form
bytes. S4 now refuses a checked claim by the feature name
`probabilistic-claim`. `expected ... until`, `elapsed`, timed windows and
`on each` still refuse `not-yet-implemented`, each at its own construct.

What was checked:

- **Every gate, run on the head in this reviewer's own clone and target
  directory.**
  - `cargo fmt --all -- --check` passes.
  - `cargo clippy --workspace --all-targets -- -D warnings` passes, and so
    does the same with `--all-features`.
  - `cargo test` passes for qsl-eval, qsl-replay, qsl-analyze, qsl-semantics
    (1192 passed) and qsl-cst. The root `cargo test --test it` passes: 975
    passed, 6 ignored.
  - `make string-edge` passes, and so do `make arch-lint-api-surface-qsl`,
    `make arch-lint-canonical-encoder` and `make arch-lint-qualified-core`.
- **Mutation testing: 25 hand mutants, 21 killed and 4 survived.** Each was
  run against `cargo test -p qsl-semantics --test it -- probabilistic`, and
  the tree was clean afterwards.
  - Killed:
    - the unit-mismatch check (M01);
    - the fraction-threshold upper bound (M02);
    - the foreign-workload check (M03);
    - `θ + ι < 1` (M04);
    - the repeated-parameter and omitted-parameter partial-confidence checks
      (M05, M06);
    - dropping `alpha`, the workload, the profile, the fairness set, or the
      bound direction from the identity (M07, M08, M10, M23, M25);
    - the trace-subject branch (M13);
    - the unbounded-profile refusal (M14);
    - fairness de-duplication, and dropping the fairness set (M15, M16);
    - the ambiguous-reward check (M18);
    - the recorded horizon (M19);
    - the converted threshold value (M20);
    - the S4 feature name, and dropping claims from `first_declaration` (M21,
      M22);
    - mapping a missing interval to a plain temporal refusal (M24).
  - Survived:
    - M09: replacing `encode_predicate` with `Debug` bytes (FND-005);
    - M11: dropping the `over` binding from the identity (FND-005);
    - M12: dropping the subject from the identity (FND-005);
    - M17: dropping the subject-model filter on a reward name (FND-006).
- **Identity behaviour, probed with throwaway tests that were reverted.**
  - `0.005 s` and `5 ms` give equal identities.
  - A commuted equality inside `long_run_fraction holds(..)` gives an equal
    identity. The code is right; only a test is missing (FND-005).
  - Reordering a workload's weight lines changes the identity (FND-002).
  - Renaming the `over` parameter `s` to `t` changes the identity (noted
    under FND-001).
- **Diagnostics.** Every new cause has a `check_message` arm in
  `qsl-replay/src/spine.rs`. The trace-subject message ("window aggregates
  (FR-194)") and the qualitative-threshold message ("ADR-018 TP-2 claim",
  "possible property") are asserted at the CLI compile. So is the
  unit-mismatch message naming both dimensions.
- **The S1 refusal test** (`qsl-cst/tests/it/probabilistic.rs:236`). It
  refuses `under Steady fair weak Node::tick` `invalid_syntax` at `fair`, and
  admits the `every scheduler` form.
- **The visibility widenings are justified.**
  - The `pub(super)` items in `probabilistic/model.rs` (`Pass` and its
    fields, `Site`, `site_location`, `literal`, `unresolved`, `operation`,
    `typer`) reach only the sibling `claim` module.
  - The `pub(crate)` on `temporal_clause::{horizon, HorizonFault}` matches
    the existing `checked_operators`, `checked_constraint` and
    `build_formula`.
  - The `pub(super)` on `hyper_clause::{node_bytes, encode_predicate,
    encode_event}` is used only by `claim.rs`.
- **No compatibility layer or dead code.** The old `Pass::claim` and the
  blanket `NotYetImplemented { claim, span }` are deleted, not kept beside
  the new path. `formula_horizon` is now a thin locating wrapper over the
  shared `horizon`.
- **The author's judgement calls:**
  - `Workload(String)`, `ModelSubjectRef`, `DefinitionRef` as the profile,
    `OverBinding`, and the fairness `Vec` are reasonable. Each is a deviation
    from FR-186 "Outputs" that the spec should record (FND-007).
  - Literal-only thresholds: FND-012.
  - The new causes: FND-008.
  - The identity carrier: FND-001.

## Verdict

Not mergeable as is: changes requested.

- FND-002 and FND-003 are code fixes the coder can make in this PR.
- FND-001 needs a spec ruling before the identity is treated as the O-09
  obligation identity. A ruling, or a deferral to a spec ticket, unblocks the
  merge.
- The low findings are test gaps, spec-text follow-ups and small
  robustness nits.
- The ACs themselves hold: each of FR-186-AC-1 to AC-4 is asserted by a test
  that a mutant of the check it names makes fail. The exceptions are the
  identity members listed in FND-005.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The claim's "obligation identity" is not an identity of any kind ADR-013 registers, and it contradicts O-09. `ProbabilisticObligationIdentity` is raw length-prefixed bytes compared lexically. It has no digest domain and no RFC 8785 preimage schema, so it is none of R-04's four equality kinds, and its doc does not state that it is not an identity. O-09 defines the obligation identity as a digest, which CG computes, over the clause node id, the occurrence key, the kind and the arguments, "or two occurrences of one clause collide". This identity omits the occurrence key, and the test at probabilistic_declarations.rs:1883-1896 asserts that two distinct declarations `C` and `D` have equal identities. It also binds the `over` parameter's name, so alpha-renaming `s` to `t` makes it another obligation (probe). Recommendation, to be ruled by the spec owner rather than guessed: (a), preferred, amend FR-186 so that S3 records the claim's normal form (the content its S4 claim node will be keyed over) and states, per R-04, that it is not an identity; O-09's obligation identity then stays CG's digest over the S4 node and its occurrence key, and the type is renamed (for example `ProbabilisticClaimContent`). Or (b) add an "O-09 probabilistic preimage" subsection to ADR-013, like the parity preimage, defining the JCS members, and have QSL produce that preimage object. Either ruling must also say whether two declarations of equal content are one obligation, and whether a binder name is content. | qsl-semantics/src/check/probabilistic/claim.rs:274-288; qsl-semantics/src/check/probabilistic/claim.rs:1268-1336; qsl-semantics/tests/it/probabilistic_declarations.rs:1883-1896; spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:266-276 |
| FND-002 | medium | The identity depends on the source order of a workload's weights. `workload_bytes` pushes one `weight` part per entry of `workload.weights()`, which is in written order. So `workload Burst { weight Server::request = 3; weight Server::attempt = 1; ... }` and the same workload with the two lines swapped make one claim into two obligations: a throwaway probe showed the identities unequal. The fairness set and a reward's operations are sorted, but the weights are not. Fix: sort the weight parts (for example by operation node key) before encoding, and add an identity-equality case for reordered weights. | qsl-semantics/src/check/probabilistic/claim.rs:1338-1354; qsl-semantics/src/check/probabilistic/model.rs:992-1031 |
| FND-003 | medium | Identity bytes are produced with Rust `Debug` formatting. A fairness constraint's kind and granularity are encoded as `format!("{:?}", ...)`, and every `over` type other than a model object reference falls back to `format!("{value_type:?}")`. `Debug` output is not a contract: a derive change, a variant rename or a new field in `quire_semantic_value` or `quire_exact::ValueType` silently changes every identity. No test pins the bytes, so nothing would fail. Fix: encode each member through an explicit, exhaustive tag table (as `Bound::bytes` and `temporal_tag` already do), and refuse an `over` type that has no defined encoding rather than falling back to `Debug`. | qsl-semantics/src/check/probabilistic/claim.rs:1369-1370; qsl-semantics/src/check/probabilistic/claim.rs:1379-1391 |
| FND-004 | low | Several silent fallbacks replace a broken invariant with a value. In `fairness_bytes`, `.unwrap_or_default()` encodes an empty declaring key when the model lookup fails, so two different operations could encode alike. `workload_bytes` drops the model and the weights when `self.model.workload(name)` is `None`. `ClaimCx::location` and `exact_location` use `position.unwrap_or(0)`, which mislocates a diagnostic at site 0. The rest of the file reports such cases with `broken(...)`. Fix: return `broken("...")` refusals, making `obligation` fallible, or `expect` with the invariant named where it is genuinely unreachable. | qsl-semantics/src/check/probabilistic/claim.rs:1341; qsl-semantics/src/check/probabilistic/claim.rs:1358-1365; qsl-semantics/src/check/probabilistic/claim.rs:416-431 |
| FND-005 | low | The identity tests do not cover three members, because three mutants survive. M09: replacing `encode_predicate` with `Debug` bytes survives, because the commuted-equality case (probabilistic_declarations.rs:1865-1882) uses a formula event, which is encoded by `encode_event` and never reads `CheckedPredicate::normal_form`. No test checks normal-form equality for a fraction, long-run, `accumulate` or `steps` predicate. A probe showed that the code is correct there. M11 and M12: dropping the `over` binding or the subject from the identity survives, because no variant changes only the subject or only the `over` binding. Fix: add a commuted predicate inside `long_run_fraction` or `fraction`, and variants that differ only in the `over` type and only in the subject. | qsl-semantics/tests/it/probabilistic_declarations.rs:1763-1897; qsl-semantics/src/check/probabilistic/claim.rs:1146 |
| FND-006 | low | No test checks that a reward is resolved within the subject's model. Mutant M17 removes `&& reward.operation().model() == claim.subject.alias` from `reward_named` and survives: no test names a reward that only another model declares. FR-186 requires `R` to be "declared by at least one operation of the model". Fix: in the two-model fixture of `a_workload_of_another_model_is_foreign`, a claim on `Other` that accumulates `duration` (declared on `Service`) refuses `missing-name`. | qsl-semantics/src/check/probabilistic/claim.rs:1065-1082 |
| FND-007 | low | The recorded type deviates from FR-186 "Outputs" without a spec change. The FR has `ProbScheduler::Workload(WireNodeId)`, `profile: ProfileRef`, `over: Option<Binding>` and `obligation: ObligationIdentity`. The code has `Workload(String)`, `DefinitionRef`, `OverBinding` and `ProbabilisticObligationIdentity`, and adds `name`, `span` and `fairness`. The justifications hold: a workload has no checked node (FR-185 "Package emission"), and AC-3 needs the fairness set on the claim. The spec still describes another shape. Fix: a spec follow-up amending FR-186 "Outputs" to the recorded shape, including `fairness`, so the code stays unchanged. | qsl-semantics/src/check/probabilistic/claim.rs:75-85; qsl-semantics/src/check/probabilistic/claim.rs:290-316; spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md:51-61 |
| FND-008 | low | New diagnostic causes and a feature name are not in the spec. FR-186 "Behavior" names only `invalid-confidence` and `quantile-over-fraction` as QSL-proposed causes requested in STD-188. The PR emits three more `invalid_probabilistic_model` causes: `trace-subject`, `unbounded-event` and `not-temporal-profile`. S4 also refuses with the feature name `probabilistic-claim`, while FR-185 "Package emission" lists only `random-parameter`, `workload` and `reward`. Fix: a spec follow-up adding the three causes to FR-186 (and to the STD-188 request to QSpec) and `probabilistic-claim` to FR-185 "Package emission". | qsl-semantics/src/check/probabilistic.rs:551-553; qsl-semantics/src/check/probabilistic/model.rs:323; spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md:84-92 |
| FND-009 | low | A fraction comparison threshold outside `(0, 1)` gets the wrong advice. `rational(1, 1)`, `rational(0, 1)`, `rational(3, 2)` or a negative value in `fraction ... >= c` is refused `qualitative-threshold`. Its message says "a threshold of 0 or 1 is a qualitative claim, stated as an ADR-018 TP-2 claim or as a possible property", which does not fit a bound on a fraction inside an event, nor `3/2` or a negative value. Fix: a distinct message (or cause) for an event's fraction bound, or a message that does not presume a probability threshold. | qsl-semantics/src/check/probabilistic/claim.rs:1185-1201; qsl-replay/src/spine.rs:991-995 |
| FND-010 | low | The horizon-overflow refusal of an event is unlocated and untested. When a sum of interval bounds overflows `u64`, `claim.rs` calls `super::horizon_refusal(actual)`, which builds `Location::root` and has no region, so the `Overflow { span }` field is discarded. A temporal clause's overflow is located at the operator (`formula_horizon`). No test reaches either path for a claim. Fix: carry the span into the refusal's region, and add a test with two `[0, 2^63]` intervals. | qsl-semantics/src/check/probabilistic/claim.rs:922-929; qsl-semantics/src/check/probabilistic.rs:580-593 |
| FND-011 | low | The S4 refusal of a checked claim is tested only through `first_declaration()`. Before this PR an S3 refusal kept every claim from reaching S4. Now `qsl-package::emit` (emit.rs:1234) is the only thing that stops a checked claim from being emitted, and no emit-level test compiles a claim-only package and asserts `ProbabilisticModelNotEmitted` with feature `probabilistic-claim`. Fix: one `emit` test with a claim-only unit, so that a future change to the emitter's guard fails a test. | qsl-semantics/tests/it/probabilistic_declarations.rs:1096-1112; qsl-package/src/emit.rs:1234-1241 |
| FND-012 | low | A threshold that is not a literal is refused `ill_typed`/`type-mismatch`. `threshold` requires `literal(body.root())`, so a well-typed constant expression in the `expr` position is reported as a type mismatch, although its type matches. FR-186 does not say that a threshold must be a literal. Fix: either state the literal rule in FR-186 and refuse with a cause that names it, or fold constant expressions before the literal check. | qsl-semantics/src/check/probabilistic/claim.rs:1171-1178 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-013 | low | The new refusal `over-not-object-type` (`OverNotObjectType`, `invalid_probabilistic_model`) has no owning statement in the merged FR-186 (#1117). The FR's list of QSL-proposed causes requested in STD-188 does not name it, and the Normal form section names "the `over` type" as content without restricting it. The restriction itself is sound: FR-125 "Instancing" binds `over` to objects of the subject's universes, so a scalar `over` type has no instance, and refusing it is what lets `type_bytes` drop the `Debug` fallback (SR FND-003). Fix: a spec follow-up adding a "Kind and subject" statement ("an `over` parameter SHALL have an object type of the subject's model, or S3 refuses the claim [`over-not-object-type`]"), and adding the cause to the STD-188 list. The code stays as it is. | qsl-semantics/src/check/probabilistic/claim.rs:711-717; qsl-semantics/src/check/probabilistic.rs:439-449 |

## Dispositions

Round 1, reviewed at `ef714e5855d617ed3262a8a6d25dc659d05ef5cf`. The fix
commits are 96d0012d9 (normal form and review fixes), ae327168b (clippy) and
ef714e585 (`fraction-bound-out-of-range`). The merge commit 37a2a9f77 merged
main at 78b8f4e56.

The branch does **not** contain spec PR #1117 (d2e3c128b), which is on main.
`git merge-tree` shows that the merge is clean, and the dispositions below
are judged against FR-186 as #1117 left it on main.

Gates, all run on the head: fmt, clippy `-D warnings`, the qsl-eval,
qsl-replay (869 tests), qsl-analyze, qsl-semantics (1195) and qsl-cst test
suites, root `--test it` (975 passed, 6 ignored), `make string-edge`, and the
three arch-lint targets. All passed.

Mutants on the head: 14 run, 12 killed and 2 survived.
- Killed: the four survivors of the first round, M09, M11, M12 and M17, are
  now killed. So are unsorted weights, the binder normalisation, the
  object-type check, the fraction-bound cause, the threshold-not-literal
  cause, the horizon location, and the fairness kind and granularity tags.
- Survived: unsorted fairness constraints and unsorted reward operations
  (SR-13596 FND-007).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 96d0012d9 with spec #1117: `ClaimNormalForm`, documented as not an identity (R-04); O-09's identity stays CG's digest over the S4 node and its occurrence key; equal-content declarations having equal normal forms is now the specified behaviour; the binder name is not content |
| FND-002 | fixed | 96d0012d9: the weight parts are sorted, and `burst(BURST) == burst(BURST_REORDERED)` is tested (mutant N3 killed) |
| FND-003 | fixed | 96d0012d9: explicit `weak`/`strong` and `whole`/`each` tags (mutants N9 and N10 killed); `type_bytes` has no `Debug` fallback, and a non-object type is refused before it is encoded; claim.rs has no `format!("{:?}")` left |
| FND-004 | fixed | 96d0012d9: `broken(...)` refusals in `workload_bytes`, `fairness_bytes`, `type_bytes`, `location` and `exact_location`; `normal_form` is fallible |
| FND-005 | fixed | 96d0012d9: a commuted predicate in the long-run path, and variants that differ only in the subject and only in the `over` binding (M09, M11 and M12 killed) |
| FND-006 | fixed | 96d0012d9: `a_reward_declared_only_by_another_model_is_not_the_subjects` (M17 killed) |
| FND-007 | fixed | spec #1117: FR-186 "Outputs" now states the recorded shape (`name`, `span`, `DefinitionRef`, `Workload(String)`, `fairness`, `OverBinding`, `normal_form: ClaimNormalForm`) |
| FND-008 | fixed | spec #1117: `trace-subject`, `unbounded-event`, `not-temporal-profile` and `fraction-bound-out-of-range` are listed as QSL-proposed causes (STD-188), and `probabilistic-claim` is in FR-185 "Package emission" and FR-186 "Normal form"; `over-not-object-type` is new and is filed as FND-013 |
| FND-009 | fixed | ef714e585: `FractionBound` gives `fraction-bound-out-of-range`, and the message does not name the qualitative form; both are tested (mutant N6 killed) |
| FND-010 | fixed | 96d0012d9: the refusal is located at `Site::Operator`, and two `[0, 2^63]` intervals are tested at `eventually` (mutant N8 killed) |
| FND-011 | fixed | 96d0012d9: `qsl-package/src/emit/tests/probabilistic.rs` `a_checked_claim_refuses_emission` |
| FND-012 | fixed | 96d0012d9 with spec #1117: FR-186 states the literal rule, and `ThresholdNotLiteral` refuses `ill_typed`/`type-mismatch` (mutant N7 killed) |

Round 2, reviewed at `20a7023c0556c145e71c9bb46e3f029d21b914e8`. That is
merge commit 20a7023c0, which brings in main at 47ac2d4fd (spec #1117 and
QSL-883 `s3.rational_bits`) together with the fix round.

The resolution was reviewed with `git show --remerge-diff`. It has one
conflict, in `probabilistic_declarations.rs`, where both sides are kept: main's
two `s3.rational_bits` tests and the branch's `OVER` const. Its `claim.rs`
changes are:
- the configured `rational_bits()` ceiling is passed to `exact::value`,
  `exact_refusal` and `fraction_type`;
- `AmbiguousLiteral` maps to `type-mismatch` for a `rational(n, d)` threshold
  of a non-fraction measure (FR-186-AC-5).

Gates on the head: fmt, clippy `-D warnings`, the qsl-eval, qsl-replay,
qsl-analyze, qsl-semantics (1199) and qsl-cst suites, root `--test it` (975
passed, 6 ignored), `make string-edge`, and the three arch-lint targets. All
passed.

`quire matrix`: FR-186-AC-1 to AC-6 are tagged.

Mutants re-run: N1 (fairness unsorted) and N2 (reward operations unsorted)
are now killed by `the_normal_form_ignores_declaration_order_and_binds_the_over_type`.
M09, M11, M12, M17, N3, N4 and N7 stay killed.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-013 | deferred | The coder keeps `over-not-object-type` and lists it as a spec follow-up: an FR-186 "Kind and subject" statement and the STD-188 cause list. The restriction is consistent with FR-125 "Instancing". No ticket id was given, so the code lead should file it in Linear before merge. |

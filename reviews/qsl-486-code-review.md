---
id: SR-1346
title: "Code review of quire-spec-language PR #647: B5 identity limits, FR-255 settings and identity.input_bytes threading"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@1db5326b01a40a18f17bd5d408736f92fcf7d83d; PR #647 diff against origin/main (merge base 03d3f899a), 96 files: qsl-foundation/src/{setting,identity_limits}.rs, qsl-foundation/src/diagnostic/{stage,locus}.rs, qsl-cst/src/lexer.rs, qsl-semantics/src/check/{assemble,assemble/units,check,family,lowering,lowering/model,mod,node_key/mod,node_key/shape,refusal,settings}.rs, qsl-semantics/src/library/{bundle,package_identity}.rs, qsl-semantics/src/model/{observation,observation/document,observation/tree,observation/frame,accounting,population}.rs, qsl-semantics/src/value/{enumeration,semantic_node,unit}.rs, qsl-eval/src/simulation/{key,order,explore,sample,trace}.rs, qsl-package/src/{checked_v2,emit/extent_agreement}.rs, qsl-replay/src/{limits,bounds,request,execute,spine,spine/lifecycle,witness}.rs, qsl-bench/src/check.rs and their tests"
review_set: subset
---
## Summary

Ticket: QSL-486. PR: quire-spec-language#647 (B5a and B5b). Code review with
the rust-review lane folded in. Focused build run through
`locked-build.sh`: see Verdict for the command and result (exit 0).

- `Setting` (qsl-foundation/src/setting.rs) is one macro-generated table of
  name, stage and kind. `LimitExceeded` now takes a `Setting` and derives its
  kind from it, so a producer cannot name a kind that disagrees with its
  setting. `SettingLimits::set_bound` is the one write path for the settings
  operation (`CallerLimits::from_operands`) and a request's `stage_limits`.
- `identity.input_bytes` has one setting owner: `AssemblyLimits.identity`,
  reported by `AssemblyLimits::bounds` inside `SpineLimits`, inside
  `CallerLimits`. The production compile path is
  `spine/lifecycle.rs:889` `assemble_with_cancel(limits.assembly)` ->
  `assemble.rs:1680` `package.identity = limits.identity` ->
  `PackageDeclarations::check` -> `Lowering::new(.., self.identity)`, which
  feeds `node_key`, `group_keys`, nominal `preimage_bytes` and
  `member_preimage_bytes`. Nominal admission (`admit_enum`,
  `declared_type_handle`, `units::assemble`, `admit_unit_graph`) takes
  `limits.identity` directly. `lifecycle.rs:234` maps a check-stage
  `IdentityRefusal::InputBytes` to `LimitExceeded` naming the setting.
  `link_bundle` and `explore_request` take an `IdentityLimits` argument and
  have no in-workspace production caller.
- Every remaining `IdentityLimits::default()` outside tests is the
  `AssemblyLimits` default, the `PackageDeclarations::new` default (see
  FND-002), or a `#[cfg(test)]` helper (`application_node_key`, bundle and
  semantic_node test modules). No `IDENTITY_LIMITS` constant is left on a
  production path.
- Deep reads: the observation reader (`observation/tree.rs`) walks
  `quire_canonical::NodeRef` from an explicit heap stack into an arena
  `SnapshotValue`. Its `PartialEq` is iterative and its drop is a flat
  `Vec`, so no recursion follows the document's depth. Behaviour matches the
  removed `serde_json` reader form for form. `package_identity.rs` moves to
  the shared reader with the same member checks.
- The `cargo fmt` commit (dca34670e) touches 25 files. Every one is already
  in the PR for the identity threading, so nothing outside the PR's purpose
  was reformatted.

## Verdict

Request changes (medium findings). The threading is correct where I traced
it, and `AssemblyLimits.identity` is the single setting owner. But no test
shows the caller's value reaches the check stage (FND-001), and FND-003 maps
an allocation failure to a malformed value, which FR-259 Behavior 6 forbids.

The four listed exceptions are accepted:

1. `mint_variant_id` under the unbounded `VARIANT_ID_LIMITS`: accepted. The
   same `quire.enum-member-node/v1` preimage bytes are already encoded under
   the caller's limit by `admit_enum` (`nominal_key(&member, identity)`) and
   by lowering's `member_preimage_bytes(.., self.identity)`
   (lowering.rs:2083), so the caller bound has already been applied to
   identical bytes. The `.expect` can only fire on an allocation failure.
2. qsl-bench/src/model.rs and qsl-semantics/tests/it/model_intake.rs using
   qsv's `IDENTITY_LIMITS`: accepted. Both encode a fixture document with no
   caller.
3. The emitter's `EMIT_LIMITS = u64::MAX`: accepted. It projects a graph
   whose every preimage was already encoded under the caller limit.
4. Accounting counters without FR-255 rows: accepted. FR-255's prose now
   says they are set in a request's accounting limits, not through
   `--limit`/`CallerLimits`, and have no row.

Build: `locked-build.sh <wt>/target cargo test -p qsl-foundation -p
qsl-replay -p qsl-eval --lib -- setting limits:: simulation::key`: exit 0,
12 passed (2 qsl-eval key, 4 qsl-foundation, 6 qsl-replay bounds and
limits), 0 failed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test shows the caller's `identity.input_bytes` reaching the check stage. The only enforcement tests call `canonical_bytes` (node_key/tests.rs:969) and `state_key` (key.rs:184) directly with a hand-built `IdentityLimits`. No test sets `AssemblyLimits.identity`, or a request's `identity.input_bytes` entry, below a package's preimage size and observes the refusal from assemble, check, `link_bundle` or the replay spine. If `package.identity = limits.identity` (assemble.rs:1680) were deleted, if `Lowering::new` got `IdentityLimits::default()`, or if `lifecycle.rs:234`'s mapping were removed, every test would still pass. Fix: one spine or assemble-plus-check test at a bound one below the largest preimage that asserts `Setting::IdentityInputBytes`, the bound and the count, and that passes at bound + 1. | qsl-semantics/src/check/assemble.rs:1680; qsl-semantics/src/check/lowering.rs:1553; qsl-replay/src/spine/lifecycle.rs:234 |
| FND-002 | low | `PackageDeclarations.identity` is a second, public copy of `identity.input_bytes`. `PackageDeclarations::new` sets it to `IdentityLimits::default()` (check.rs:289), and the check stage reads this copy, not `AssemblyLimits`. A package built through `new` plus public field updates and then `check(CheckingLimits)` is checked under the hidden default, and `check` has no parameter that could give it the caller's value. This is not on the spine path, which assembles. Fix: carry the identity limit in the check call's limits, or make the field private and set only by assembly. | qsl-semantics/src/check/check.rs:254-289 |
| FND-003 | medium | `EncodingRefusal::of` maps every `quire_canonical` error other than `Limit` to `NoEncoding` (key.rs:64), and that includes `Error::Allocation`. FR-259 Behavior 6 says a failed reservation SHALL be refused as `resource_exhausted`/`allocation-failed` carrying `requested`, never as a malformed value. `semantic_node.rs` already gives it its own arm (`NominalRefusal::Allocation`). Fix: add an `Allocation { requested }` variant to `EncodingRefusal`. | qsl-eval/src/simulation/key.rs:55-66 |
| FND-004 | low | `the_settings_operation_and_a_request_raise_every_setting` loops over every `Setting`, `ReplayInputBytes` included, and asserts that `CallerLimits::for_request` takes a `stage_limits` entry for it (limits.rs:243-246). That is the opposite of the test's own doc comment ("each setting but `replay.input_bytes`"), of `for_request`'s doc, and of FR-255's prose. Wire decode refuses the entry, but the public `FromIterator for StageLimits` (request.rs:94) skips that check, so `for_request` overrides the library's `replay` limit. Fix: have `for_request` skip `ReplayInputBytes` (or `StageLimits` refuse it at every constructor), and assert that in the test. | qsl-replay/src/limits.rs:60-73; qsl-replay/src/limits.rs:236-248; qsl-replay/src/request.rs:94 |
| FND-005 | low | `plain_digest`'s doc says the draw preimage "never reaches [`LIMITS`]' 16777216-byte bound", but `LIMITS` was removed and the draw now encodes under the unbounded `DRAW_LIMITS`. The text is stale and the intra-doc link is broken. | qsl-eval/src/simulation/key.rs:106-113 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | medium | `ModelRefusalCause::AncestorSteps { from, limit }` is produced both by model normalization (`model.ancestor_steps`) and by population admission (`admission.ancestor_steps`, per `PopulationAdmissionLimits`' doc at population.rs:191). It carries neither a setting nor the count reached, and `FamilySteps { original, limit }` (`model.family_steps`) is the same. So an admission ancestor-steps refusal cannot name `admission.ancestor_steps`. Only the compile spine names a setting for these causes, and it maps every `AncestorSteps` to `model.ancestor_steps` (lifecycle.rs Intake arm), so the cause alone cannot say which limit was hit. This breaks FR-255 Behavior 1 for two rows, and naming the setting on every limit outcome is B5's own deliverable ("`LimitExceeded` names the limit setting that was hit"). Fix in this PR: give both causes the `Setting` (and the count reached) at each producer, and drop the two partial rows from FR-255's Status. | qsl-semantics/src/model/refusal.rs:72-77; qsl-semantics/src/model/refusal.rs:171-176; qsl-semantics/src/model/population.rs:191 |
| FND-007 | high | An `identity.input_bytes` limit reached in assembly is refused as a plain assembly error, not as a limit outcome. In assembly, `declared_type_handle` returns `AssemblyCause::Handle(Identity(InputBytes { bound, required }))`, and `lifecycle.rs`'s `CompileRefusal::Assembly` arm maps only `TypeLimit`, `DecimalScaleLimit` and `IdentityLimit` to `LimitExceeded`. So `check` returns `Refused(Assembly { .. Handle(Identity(InputBytes { bound: 122, required: 123 })) .. })` with no `identity.input_bytes` setting, and `AssemblyCause::Handle` reports `runtime_invariant`/`established-invariant-broken` (assemble.rs:383, 439), so a caller limit is reported as an internal fault. That is against FR-255 Behavior 1 and FR-259 Behavior 4. The two new tests catch this, and both fail at 0d47d0c1: `check_names_the_identity_limit_it_reached` and `a_reached_limit_is_raised_by_the_settings_operation_and_by_a_request` (lifecycle/tests.rs:953, "expected an output or a limit, got Refused(Assembly ..."). The head is red. Fix: map the handle's identity byte refusal to `AssemblyCause::IdentityLimit(identity_limit(bound, required))` (or have the spine arm accept it), and re-run both tests. | qsl-replay/src/spine/lifecycle.rs (CompileRefusal::Assembly arm); qsl-semantics/src/check/assemble.rs:1487; qsl-replay/src/spine/lifecycle/tests.rs:1179-1260 |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | medium | Observation admission's type-membership check (FR-106 check 6.1) calls `model_index().conforms(&object_key, member, 64, Setting::ModelAncestorSteps).unwrap_or(false)`. That is a hidden, fixed 64-edge ancestor ceiling. It is not a caller limit, has no FR-255 row, and is not the `model.ancestor_steps` value its refusal now names. If an object's type is more than 64 ancestor edges from a population member type, the `AncestorSteps` refusal is swallowed by `unwrap_or(false)` and read as "does not conform", so a valid object is refused `invalid_runtime_input`/`wrong-role-mapping`. That is a wrong verdict, not a limit outcome. The literal is from main, but this PR reworks the file and, in round 2, labelled the refusal with a setting whose value it does not use. Fix in this PR: take the caller's `admission.ancestor_steps` (or the observation stage's own setting) and return its refusal as the limit outcome instead of `false`. | qsl-semantics/src/model/observation/document.rs:1192-1209 |

## Dispositions

Round 1, reviewed at 0d47d0c1865d0243f4cceca4b0b246af6325e115 (`git diff 1db5326b..0d47d0c1`: 58fc0fd01, 535697faa, b08397caf, 4d93d589e; 0d47d0c18 only adds the review files). Focused run at the head: `cargo test -p qsl-replay -p qsl-eval -p qsl-semantics -- limits:: lifecycle::tests:: simulation::key ...`. qsl-replay's lib has 30 passed and 2 failed, the two new identity tests (FND-007). A second `--no-fail-fast` run of the qsl-eval and qsl-semantics tests the fixes added or touched has 16 passed and 0 failed. I also ran a mutation (`PackageDeclarations::new(source, IdentityLimits::default())` at assemble.rs:1679) in a throwaway worktree. The two tests failed there too, but they already fail at the head, so the mutation shows nothing about their oracle until FND-007 is fixed. The many test call-site edits only add the new `IdentityLimits` argument to `PackageDeclarations::new` and its reformatting. No assertion is removed or weakened: the removed `limits.rs` assertions are replaced by table-driven ones, and the removed lines elsewhere are the three FR-255-AC-1 tags.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | still-open | The tests were added (58fc0fd01) but fail at the head because of FND-007, so the threading is still not shown by a passing test. Re-check by mutation once FND-007 is fixed. |
| FND-002 | fixed | 58fc0fd01 |
| FND-003 | fixed | 58fc0fd01 |
| FND-004 | fixed | 58fc0fd01 |
| FND-005 | fixed | 58fc0fd01 |

Round 2, reviewed at eab14d4b093a6fc1ebf93def26ee8946380bbd01 (`git diff 0d47d0c1..eab14d4b`: 155aa2e78, 1d399712a, eab14d4b0).
- The head is green on the focused set. `locked-build.sh … cargo test --no-fail-fast -p qsl-replay -p qsl-semantics` over the two identity spine tests, `limits::`, `lifecycle::tests::`, `model_conformance::`, `model_dispatch::`, `model_population::`, the library limit test and `checked_dispatch` passed 138 with 0 failed.
- FND-007: `AssemblyCause::handle` maps the handle's identity byte refusal to `IdentityLimit` and an allocation failure to `IdentityAllocation`.
- FND-006: both step causes now carry `setting` and `reached`. `ModelRefusalCause::limit_exceeded` builds the outcome, and every `conforms` caller passes the setting it is bounded by (admission walks pass `AdmissionAncestorSteps`).
- FND-001: mutation re-checked at this head in a throwaway worktree, with `PackageDeclarations::new(source, IdentityLimits::default())` in place of `limits.identity` at assemble.rs. Both identity spine tests still pass (2 passed, 0 failed), so the mutation survives. This fixture has no enums or units, so the only assembly-side identity encoding is the declared type handle (about 123 bytes). With lowering on the default limit, `assert_field`'s search finds the handle's size as the counter and stays consistent. The tests prove the spine maps an assembly identity refusal, not that the caller's value reaches lowering. Their doc comment says they would fail "if the assembly stops carrying `AssemblyLimits.identity` into the checked package, if lowering falls back to a default", and that is not true. Fix: after the search, assert that the counter equals the largest node preimage the checked package holds under default limits (or use a fixture whose largest preimage is a lowered node and assert counter > the handle preimage size).

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | still-open | The mutation (lowering identity defaulted at assemble.rs) survives both identity spine tests at eab14d4b; the counter they find is the declared type handle's size, so lowering threading is still unproven and the test doc's claim is false |
| FND-006 | fixed | 155aa2e78 |
| FND-007 | fixed | 155aa2e78 |

Round 3, reviewed at 8b33cf8b8ed0a7fbfbf31fab35a98fbbfdcf7ec1 (`git diff eab14d4b..8b33cf8b`: b3ba680af, a6660a7f5, 8b33cf8b8).
- The head is green on the focused set. `locked-build.sh … cargo test --no-fail-fast -p qsl-replay -p qsl-semantics` over the two identity spine tests, `lifecycle::tests::`, `limits::`, `state_clauses::` and `model_population::` passed 194 with 0 failed.
- FND-001: the identity tests now compile a unit with no record, enum or unit (a 40-term function). They assert that its largest node preimage is over 200 bytes and that the counter the search finds equals that preimage. The AC-4 identity leg uses the same unit (a6660a7f5). Mutation re-run in a throwaway worktree with its own, isolated target dir (every qsl-* crate recompiled from the mutated sources), with `PackageDeclarations::new(source, IdentityLimits::default())` in place of `limits.identity` at assemble.rs. Both tests fail (0 passed, 2 failed) at lifecycle/tests.rs:995, "`IdentityInputBytes` is not reached by the input". The mutant is killed.
- Round 2's mutation shared the reviewed worktree's target dir, so its "survives" result may have been a false pass of the same kind the coder hit. The finding also stood on the fixture analysis (only a handle-sized preimage at assembly), and the strengthened test makes the point moot.
- FND-008: the observation membership walk now takes the caller's `model_limits.ancestor_steps`, bounded and named as `model.ancestor_steps`, and returns a reached ceiling as the limit outcome instead of reading it as "does not conform". `a_deep_hierarchy_is_judged_by_the_callers_ancestor_steps_not_a_fixed_ceiling` admits a 70-edge hierarchy under the default (the fixed 64 refused it) and refuses at bound 60 naming the setting.
- No new findings this round.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | b3ba680af |
| FND-008 | fixed | b3ba680af |

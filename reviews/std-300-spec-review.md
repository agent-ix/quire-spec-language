---
id: SR-15570
title: "spec review of STD-300 and STD-312 evaluator differential corpus and T12-A corpus producer (QSL PR #1229)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@37afaae534e1617d2ec2fd5736f5c6dcd6d71909; spec/functional/FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md, spec/functional/FR-060-check-qsl-api-surface-boundary.md, spec/test-cases/TC-1112-evaluator-differential-corpus-and-harness.md, spec/test-cases/TC-1113-t12-a-evaluator-corpus-producer.md, spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md, spec/assurance/AD-001-shared-evaluation-core-interface.md, spec/spec.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-402
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-060
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1112
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1113
    type: reviews
---
# SR-15570: spec review of STD-300 and STD-312 evaluator differential corpus and T12-A corpus producer (QSL PR #1229)

## Summary

Tickets: STD-300 and STD-312. The diff is `origin/main...37afaae53` (main 828ddb6b9, which is an ancestor of the head). It touches 7 files and is spec only. It adds FR-402 with AC-1 to AC-8, TC-1112, FR-060's "T12-A: the evaluator corpus producer" section, an amended T12-A row, FR-060-AC-8 and AC-9, TC-1113, one sentence on ADR-011's `qsl-replay` crate-map row, one AD-001 paragraph and the spec.md index entries.

The reviewer checked the diff against QSL main and against the code it names: `tools/arch-lint/api_surface.rs` (the T12-A rule data), `qsl-replay/src/compile.rs` (`compile_package`), `qsl-foundation/src/source.rs` (`SourceIdentity`), `qsl-package/src/emit.rs`, `qsl-eval/src/value/expression/{evaluate,mod}.rs` (`Evaluation`, `refusal_record`, `CallFailure`), `qsl-foundation/src/diagnostic.rs` (`RefusalRecord`), and `quire-exact` at the locked commit 1c6cbcb (`Meter`, `Charge`, `AdmittedCharge`, `LimitKind`, `CHARGE_LOG_CAPACITY`, and the `test-support` feature). It also read FR-262, FR-098, FR-071, AD-001, IR-845, and CG `origin/main` 534729a (FR-044, `Cargo.toml`, and the `qsl_replay` references in `src/`).

Units examined:

- FR-402 statement, case record, compared record, corpus interface, loader refusals, corpus, harness and CI placement (examined)
- FR-402-AC-1 to FR-402-AC-8 (examined)
- FR-060 T12-A row and "T12-A: the evaluator corpus producer" (examined)
- FR-060-AC-8 and FR-060-AC-9 (examined)
- TC-1112 and TC-1113 (examined)
- ADR-011 `qsl-replay` row sentence, AD-001 S7 paragraph and spec.md entries (examined)
- FR-262-AC-1 to AC-8, FR-098 environment rule and IR-845 decision (context only)

These claims hold:

- The T12-A rule on main matches only `qsl_replay::` (`call_patterns: &["qsl_replay::"]`) and treats only `qsl_replay::spine` as a forbidden module. A CG reference to `qsl_eval::` therefore passes today. The new bullet that refuses every other QSL workspace crate is a real tightening, and FR-060-AC-9 tests it with concrete vectors.
- `qsl_replay::compile_package` exists, returns `CompiledPackage { bytes, package_id }`, and refuses `Recompile`, `DependencyInput` and `LimitAboveReader`.
- `function.call`, `work_units` and `LimitKind::WorkUnits` are real `quire.value.accounting/v1` names. `count(3)` makes four `function.call` charges (3, 2, 1, 0). `Evaluation { outcome, location, losses }` and `Evaluation::refusal_record` exist.
- `ObjectEnvironment::with_unresolved_references` exists, and FR-098 runs a request without `object_environment` over it.
- The ids FR-402, TC-1112, TC-1113, FR-060-AC-8 and FR-060-AC-9 appear on no other open PR head (20 checked) and not on main. The diff has no status wording and adds no compatibility layer. `quire validate` over the changed FR and TC files reports no errors.

Lead defaults (none is an owner answer):

1. Facade entry `qsl_replay::corpus::reference_record`: the alternatives are worse. A CG edge to `qsl-eval` breaks FR-262 ("RT and CG SHALL consume this interface, without depending on evaluator internals") and the new crate rule. `quire-eval` has no implementation and no checker, so CG could not build a checked closure with it. The owner still has to confirm this, because it reverses IR-845's "no callable application API through qsl-replay". FND-003 proposes a narrower entry that encodes and does not evaluate.
2. `quire.evaluator-case/v1`: QSL owns the encoding, so QSL should own the name. CG FR-044 on main conflicts with it (FND-010).
3. Source compiled through `compile_package` and bound by `package_id`: sound in principle. The record still lacks the inputs `compile_package` needs (FND-001).
4. Interpreted-only runs: acceptable for CG's emitted bodies. It leaves FR-262's registered-body, stepping and cancel behaviour out of the differential suite (FND-008).
5. `case_id` set and the refusal codes: acceptable except `unsupported_member`, which cannot be reached (FND-007).
6. Generator: underspecified (FND-009).
7. `evaluator_corpus` in CG `src/`: sound.
8. Placement: acceptable. The ticket-keyed lifecycle wording and the build mechanism need work (FND-012, FND-013).
9. Zero tolerated divergences: sound.

The `code-review` method was skipped, because the diff contains no code.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | major: A case record cannot drive `compile_package`. `compile_package(source: SourceIdentity, path, bytes, packages, dependencies, ..)` requires FR-001's four `SourceIdentity` labels, which "QSL defaults none of". The package emitter writes `source.authority` into the package (`qsl-package/src/emit.rs:870`), so `package_id` depends on the labels. The `source` member holds only `path` and `text`, so the loader cannot reproduce `package_id`, and `package_mismatch` cannot be decided. CG's corpus cases also compile with a `SourceIdentity` and a non-empty `DependencyInput` (`src/replay/composite_builder.rs:344`; the bounded-Kani corpus carries a `qsl_dependency` library source). FR-402 forbids dependencies, so v1 cannot carry those cases, and that blocks the IR-591 export. <br>Fix: add `source.identity` with the four labels, and either add a `dependencies` member (library sources compiled as `DependencyInput`) or state that dependency-bearing cases are outside v1 and name them in the CG follow-up. | spec/functional/FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md:75-76, spec/functional/FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md:84-87 |
| FND-002 | high | major: The ordered `charges` list cannot be produced where FR-402 needs it. quire-exact keeps the charge log (`Meter::admitted_charges`) only under its `test-support` feature, and its Cargo.toml says "A normal or build dependency must never enable it". A production meter keeps counts only. `reference_record` is a public item of `qsl-replay` that CG calls from `src/` through a normal dependency, so in that build it has no log. The `no_std` implementation "built as it ships" has no log either. The log is also capped at `CHARGE_LOG_CAPACITY` = 4096 and sets `charge_log_truncated` past the cap. The FR-262-AC-1 program at its criterion's input (100,000 calls) would be compared on a truncated list, and FR-402 says nothing about truncation. <br>Fix (owner decision): (a) give quire-exact a charge-log feature that a normal dependency may enable, with no cap or with truncation treated as a divergence; or (b) compare `charges` only inside QSL's harness, built with `test-support`, and drop `charges` from the `expected` record that CG writes (see FND-003). Either way, state what a truncated log means. | spec/functional/FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md:121-125, spec/functional/FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md:140-144, spec/functional/FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md:211 |
| FND-003 | medium | minor: CG's `expected` adds no evidence, but it forces the IR-845 exception. FR-402-AC-8 builds CG against the QSL revision under test, so `reference_record` in CG and the harness's std run are the same code on the same input. The `expected/std` check over CG's bundle cannot fail except through nondeterminism. CG needs `reference_record`, and with it the IR-845 exception and FND-002's charge log in a normal build, only to fill `expected`. <br>Recommended (owner decision on lead default 1): CG writes cases with `expected: null`, using `compile_package` and an encoding-only facade entry (`CaseInput` to `CaseRecord::line()`, no evaluation). QSL's harness computes the reference. The T12-A amendment then allows only the encoding types, and IR-845 stands as decided. If the owner keeps `reference_record` for CG, record that it reverses IR-845. | spec/functional/FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md:140-151, spec/functional/FR-060-check-qsl-api-surface-boundary.md:220-233 |
| FND-004 | medium | minor: The compared record's JSON is named, not defined, yet `malformed_expected` and checked-in `expected` values depend on it. `RefusalRecord` has `code`, `locus` and `fields`, but no `cause`, and has no serializer. `Evaluation::refusal_record` returns `None` for codeless refusals and needs the checked graph to resolve the locus. `Evaluation` has no record function for its undefined arms. `CallFailure`'s `Fault`, `Cancelled`, `Storage`, `Supplied` and `UnsupportedHelper` arms have no record. No QSL spec defines a `quire.native.diagnostics/v1` loss shape or an SV `Location` shape. Judgement: this does not block CG's IR-591 producer, which never reads a record, but it blocks QSL's `corpus` implementation and any non-null checked-in `expected`. <br>Fix: define `record` (including `null` when there is no record), `location` and `losses` in FR-402, or require checked-in `expected` to be `null` until they are defined. | spec/functional/FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md:113-120 |
| FND-005 | medium | minor: FR-402 names RFC 8785 but not the one encoder, `quire-canonical` (ADR-013 §2, FR-259). That encoder refuses a JSON number above 2^53. `limits` and `consumed` are u64 values and a charge amount is a `quire_exact::Integer`, so a valid record can hold a number the encoder refuses. <br>Fix: encode with `quire-canonical`, and write those integers as canonical decimal strings, as the replay value wire does. | spec/functional/FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md:65, spec/functional/FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md:80, spec/functional/FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md:121-127 |
| FND-006 | medium | minor: A charge is a vector, not one amount. `quire_exact::Charge` holds sizes per `LimitKind` plus `work_units` and `result_units`, and `AdmittedCharge` records each counter's amount. With `{"point", "amount"}`, a per-charge divergence in a size counter or in `result_units` is invisible, and so is a reordering that leaves the totals in `consumed` unchanged. <br>Fix: make each entry `{"point", "amounts": {<counter>: <integer>}}`, with the members `AdmittedCharge` holds. | spec/functional/FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md:121-125 |
| FND-007 | medium | minor: `unsupported_member` cannot be reached. The table gives `object_environment` the content `null`, so any non-null value has the wrong JSON type and refuses `member_set`, which comes first. FR-402-AC-3 expects `unsupported_member`. Related gaps: AC-3 requires every refusal to name "line 1 and the refused member's pointer", but `missing_bundle`, `empty_bundle`, `malformed_line` and `noncanonical_line` have no member, and the first two have no line. A file holding only `\n` matches both `empty_bundle` and `malformed_line`. A negative or non-integer `limits` value and a `package_id` that is not a digest record have no stated code. <br>Fix: type `object_environment` as "null or object" under `member_set`; state which codes name no line or pointer; and assign the remaining cases to codes. | spec/functional/FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md:79, spec/functional/FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md:155-176, spec/functional/FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md:250 |
| FND-008 | medium | minor: The corpus contents cannot be met and nothing verifies them. A function-call case cannot express FR-262-AC-3 (registry results), AC-4, AC-6 and AC-8 (registered generated bodies and stepping) or AC-7 (cancellation): v1 has no registration and no cancel, and it runs everything interpreted. AC-2 is a value and state-key test. "The evaluation vectors ... that a `function` call can express" is a judgement with no list, and no AC checks what the checked-in bundle holds. As a result, the std and no_std scheduler behaviour that AD-001's RT acceptance names (child handoff, preserved locals, cancellation) is never compared. <br>Fix: list the required `case_id`s or programs in an AC, and state how FR-262-AC-3/4/6/7/8 are compared, for example by running their tests in both builds or by adding QSL-owned test host bodies. | spec/functional/FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md:188-193 |
| FND-009 | medium | minor: The generator does not guarantee compiling units, yet a refused unit fails the run. FR-402 does not state the generated functions' result types, whether calls among the three functions are acyclic or carry `decreases`, or what keeps arithmetic within declared ranges. A 32-bit seed must also be mapped to proptest's 32-byte RNG seed, and `proptest` is not a dependency anywhere in the QSL workspace (Cargo.lock has no entry). <br>Fix: state result types, an acyclic call order (each function calls only later-declared ones) and the seed mapping, or specify a QSL-owned deterministic generator. | spec/functional/FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md:194-202 |
| FND-010 | medium | minor: CG FR-044 on CG main conflicts with FR-402. It names the format `quire.generated-closure-evaluation/v1`, which the loader refuses as `unknown_format`. It also requires "the generated-body registration identity", object environments, source locations and use of "the public checked-closure and application interface", all of which FR-402 v1 omits or refuses. FR-402's Dependencies line states only that CG writes through `qsl_replay::corpus`. <br>Fix: name these conflicts in FR-402's FR-044 dependency, and file the CG FR-044 amendment in Linear before IR-591's producer starts. | spec/functional/FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md:268-269 |
| FND-011 | low | minor: The T12-A crate list is underspecified. A workspace manifest lists member paths (`"."`, `"tools/arch-lint"`), not crate names. Each name comes from the member's own `[package]` (or `[lib]`) name, with `-` mapped to `_`, so the root `"."` yields `quire_spec_language`. The bullet does not say whether `qsl_eval::` is found by parsing or by substring, and FR-060's "Scanning method and its stated limitations" section is not updated. FR-060-AC-9's `qsl-extra` fixture also needs a member manifest. | spec/functional/FR-060-check-qsl-api-surface-boundary.md:246-250, spec/functional/FR-060-check-qsl-api-surface-boundary.md:280 |
| FND-012 | low | minor: FR-402-AC-7, AC-8 and the CI placement are written as a change plan ("in the change that adds the `no_std` implementation", "passes in the IR-593 switch change"), not as system behaviour, and TC-1112 step 7 cannot run until that change exists. <br>Fix: AC-7 should say that `make ci` runs `evaluator-differential` and that the harness has no tolerance option (Inspection). Move the IR-590 and IR-593 timing to AD-001. | spec/functional/FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md:228-242, spec/functional/FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md:254-255 |
| FND-013 | low | minor: "Each built as it ships" in one harness run conflicts with Cargo feature unification. AD-001 describes IR-590 as converting `qsl-eval` itself to no_std, and `qsl-replay` depends on `qsl-eval` with `std`. One binary cannot hold both builds. The PR lists this as ungrounded, and deferring the mechanism to IR-590 is acceptable. FR-402 should still say that the two runs are separate builds that exchange compared records, so that AC-5 and AC-7 do not assume a single process. | spec/functional/FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md:209-213 |

## Verdict

Not mergeable as is. There are two major findings. FND-001: the case record cannot drive `compile_package` and cannot carry CG's dependency-bearing cases. FND-002: the ordered charge log does not exist in a normal or no_std build. Owner decisions:

- Lead default 1 against IR-845, with FND-003's encoding-only alternative.
- The charge-log route in FND-002.

The T12-A amendment is sound and well tested. It closes a real hole: today `qsl_eval::` passes the rule.

Not findings, for the lead:

- CG main already references `qsl_replay` from `src/kani`, `src/routed`, `src/oracle/equality` and `src/lib.rs`, outside `replay`. The amended rule, like today's, would report these. The PR did not cause them.
- IR-591 shows state Done in Linear, while STD-300 says it blocks IR-591.

## New findings (disposition pass 1)

Head 2664e72bc (main 3763e5b7d is an ancestor). This head reworks the PR to a relayed owner decision, which the lead has not verified: CG records expectations through `quire-eval`, `qsl_replay::corpus` is removed, and IR-845 stands. The corpus moves to `quire-eval-corpus` (ADR-011 layer EC). T12-A gains one appended section and FR-060-AC-8.

The reviewer checked the following and found them sound:

- The EC dependency list (E, SV, K, `quire-canonical`, `serde`, `serde_json`) agrees with FR-401's type table, which moves `CatalogCoded`, `UndefinedCoded` and `InternalFault` to SV and `Evaluation` and `CallFailure` to E. No QSL crate is needed.
- The compared-record members exist: `Refusal::code()`/`cause()` (`None` only for `CheckedInvariant`), `InputRefusal::code()`/`cause()`, `Origin`'s variants, `Location::path()`, `ValueLoss::{Decimal, IeeeExact, IeeeFlags}`, `AdmittedCharge`'s per-size fields and `Meter::consumed`.
- The integer wire `{"kind":"integer","value":"<string>"}` agrees with the decimal-string rule.
- CG's `Cargo.toml` declares only `qsl-replay` from QSL.
- TC-069's fourteen vectors exist (`tests/runtime_evaluation_cases/scalars.rs`).
- Ids and layer EC are unique across open PRs.
- Load codes 1-11 and 13 and admission codes 1-5 are reachable in the stated order.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-014 | high | major: The claim that "the four labels enter the emitted package" is false, so FR-402-AC-2 cannot pass. The package's source reference is `RawSourceRef`/`CheckedSourceRef`: `authority`, `identity` and the source-bytes digest (`qsl-foundation/src/source/provenance.rs:66`, `qsl-package/src/emit.rs:866-874`). `revision_namespace` and `revision` are dropped. AC-2's mutation "a changed `source.identity.revision` and the old `package_id` refuses `package_mismatch`" therefore admits. <br>Fix: mutate `authority` or `identity` (or one byte of `text`) in AC-2 and TC-1112 step 2, and say which labels bind the `package_id`. | spec/functional/FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md:137-138, spec/functional/FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md:384 |
| FND-015 | medium | minor: `invalid_package_id` cannot be reached with AC-4's vector. A `package_id` is the object `{"algorithm": ..., "digest": ...}`, so `"x"` has a JSON type other than its row states. That refuses `member_set` (order 6) before `invalid_package_id` (order 12). FND-007 found the same pattern for `object_environment`. <br>Fix: use an object vector with a malformed digest (for example `{"algorithm": "sha256", "digest": "zz"}`), or state that `member_set` checks only the JSON types the table names explicitly. | spec/functional/FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md:163, spec/functional/FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md:169, spec/functional/FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md:386 |
| FND-016 | medium | minor: The comparator pairs records by `case_id`, and admission refuses the first case "in `case_id` order", but a `case_id` is unique only within one bundle. One run holds the checked-in bundle, the generated cases and an `EVALUATOR_CORPUS_DIR` bundle. The generated cases have no `case_id` scheme at all. <br>Fix: give generated cases ids (for example `gen/000` to `gen/255`), and either require ids to be unique across the run, with a collision failing it, or pair and order by (bundle, `case_id`). | spec/functional/FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md:289-310, spec/functional/FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md:327-334 |
| FND-017 | medium | minor: No admission is specified for the `no_std` build. "Each build admits every case", but only the std admission is stated, and it uses `qsl_replay::compile_package`. `qsl-replay` depends on `qsl-eval` with default features (`std` on until S9), which is the unification FR-402 cites as the reason for two builds. The `no_std` runner therefore cannot use the same admission, and FR-402-AC-9 requires both builds to admit. <br>Fix: state that the `no_std` build's `CaseAdmission` is supplied with IR-590 and refuses with the same five codes and pointers. Alternatively, specify a std-side compile step that hands the checked input to the `no_std` runner. | spec/functional/FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md:189-196, spec/functional/FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md:327-335 |
| FND-018 | low | minor: The harness is "the `xtask` command `evaluator-differential`", but "both builds are test builds of the runner with `charge-log` enabled", and `charge-log` may be enabled only by a dev-dependency. An xtask binary is not a test build, so a direct run would have `charges` `null` throughout, and AC-6's and AC-7's `/charges` assertions could not hold. <br>Fix: say that the xtask command drives the two runner test builds (for example through `cargo test` targets) and does not evaluate cases itself. | spec/functional/FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md:327-335, spec/functional/FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md:353-355 |
| FND-019 | low | minor: The new T12-A section states that `qsl-replay` is "CG's one dependency on a crate of the QSL repository", but no check holds that. FR-059 accepts CG's normal edge into QSL at repository granularity, and T12-A matches only `qsl_replay::`. Without the dropped rule, a CG manifest that adds `qsl-eval` and uses `qsl_eval::` passes both checks. This is the state on main, not a regression. The first half of FR-060-AC-8 cannot fail under today's rule data and only pins it. Owner awareness: decide whether that hole is accepted. | spec/functional/FR-060-check-qsl-api-surface-boundary.md:223-238, spec/functional/FR-060-check-qsl-api-surface-boundary.md:260 |
| FND-020 | low | minor: The relayed owner direction assumes an implementation CG can run will exist in `agent-ix/quire-eval`. AD-001's S6 bullet says IR-590 "converts `qsl-eval`", which lives in the QSL repository, while the RT acceptance asks for "the shared evaluator ... without a QSL-local evaluator dependency". Until the owner confirms where IR-590's implementation lands, CG's non-null `expected` path may never open. The direction is labelled relayed and unverified, which is correct, and needs Peter's confirmation. | spec/assurance/AD-001-shared-evaluation-core-interface.md:199-200 |
| FND-021 | low | minor: Wording nits. TC-069's fourteen vectors are listed in `tests/runtime_evaluation_cases/scalars.rs`, not in TC-069, so name that file. `ModelOperationClause.kind` is an enum with no stated encoding (snake case?). "`f0` to `fn`" reads as a keyword; write "`f0` to `f2`". | spec/functional/FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md:235-237, spec/functional/FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md:285-286, spec/functional/FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md:289-293 |

## New findings (disposition pass 2)

Head 9e808875f (main 3763e5b7d is an ancestor). The changes since 2664e72bc fix FND-014 to FND-021. They also place IR-590's `no_std` implementation in `quire-eval`, consumed by `qsl-eval` (a planner answer, labelled a lead default rather than an owner answer). AD-001's S7 paragraph is updated to match; its S6 bullet is not.

The reviewer checked the following against the code and found them sound:

- `RawSourceRef` and `CheckedSourceRef` carry `authority`, `identity` and the digest, so AC-2's mutations behave as stated: a changed authority or identity refuses `package_mismatch`, and a changed revision admits.
- `quire-exact` is `#![no_std]` with `alloc`, and its `test-support` log is an `alloc` `Vec`, so the `no_std` test build can keep the charge log that AC-9 requires.
- The new `package_id` vector is an object, so it reaches `invalid_package_id`.
- The key, the `gen/NNN` ids and the std-only admission are consistent across FR-402, AC-5, AC-7, AC-8, AC-9 and TC-1112.
- FR-060-AC-8 can fail, since a `compile_package` call outside `replay` fails today's rule data.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-022 | low | minor: "its patterns match only `qsl_replay`" is not exact. T12-A's rule data also carries the forbidden pattern `quire_spec_language::replay::` (`tools/arch-lint/api_surface.rs`, the T12-A `forbidden_patterns`). <br>Fix: "its call patterns name only `qsl_replay`". | spec/functional/FR-060-check-qsl-api-surface-boundary.md:225-226 |
| FND-023 | medium | minor: AD-001 now contradicts itself. Its S6 bullet says "IR-590 (S6) converts `qsl-eval` to no_std + alloc behind the interface", while the new S7 paragraph says the `no_std` implementation "lives in `quire-eval` ... and `qsl-eval` consumes it". NFR-015 also places `qsl-eval` at S6. The PR leaves the S6 bullet to "STD-299's S4/S6 author", but STD-299 is Done, so no open ticket owns the amendment. <br>Fix: amend the S6 bullet in this PR, or file an open Linear ticket for AD-001 S6 and NFR-015's `qsl-eval` row and name it in the PR body. | spec/assurance/AD-001-shared-evaluation-core-interface.md:177, spec/assurance/AD-001-shared-evaluation-core-interface.md:199-205 |

## Dispositions

Round 1, reviewed at 2664e72bce43f4f2b8036a5f651db296cc70820e (after 51b3fc048 and the rework).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Source objects carry the four FR-001 labels, and a `dependencies` array of library identity plus source object, built as FR-098 builds them. The binding claim about the labels is wrong: see FND-014. |
| FND-002 | fixed | `charges` is the test-support log or `null` (no log, or truncated at 4096). `charge-log` forwards `quire-exact/test-support` and only a dev-dependency enables it. `consumed` is always compared. |
| FND-003 | fixed | `qsl_replay::corpus` is removed and IR-845 is untouched. CG writes `expected: null` and takes `package_id` from `compile_package` in `replay`. Expectations come through `quire-eval` later (relayed owner direction, FND-020). |
| FND-004 | fixed | Every result, outcome, `CallFailure`, location, loss, `consumed` and `charges` form is defined from public members. v1 encodes only Boolean and Integer values, other kinds refuse `unencodable_value`, and losses are compared by kind and location. |
| FND-005 | fixed | A JSON encoding section requires `quire-canonical` and writes every u64, usize and `Integer` as a canonical decimal string. |
| FND-006 | fixed | Each charge entry is `{"point", "sizes": {<counter>: <amount>}}`, taken from `AdmittedCharge`'s recorded sizes. |
| FND-007 | fixed | `object_environment` is "`null` or a JSON object", so `true` refuses `member_set` and `{}` refuses `unsupported_member`. Bundle, line and member refusals state what they name. A file holding only `\n` refuses `malformed_line`. `invalid_limit` covers keys and values. The same reachability defect recurs for `package_id`: see FND-015. |
| FND-008 | fixed | The checked-in bundle has an exact `case_id` list. FR-262-AC-2, AC-3, AC-4, AC-6, AC-7 and AC-8 run as their own tests against both builds (AC-9), and AC-8 checks the bundle's contents. |
| FND-009 | fixed | A QSL-owned SplitMix64 generator seeded with `0x5153_4C37 + i`, acyclic calls to higher-numbered functions, nonzero literal divisors, stated result types and source labels, and AC-8 checks for no new RNG or property-testing crate. |
| FND-010 | fixed | FR-402's Dependencies list FR-044's format, object-environment and std-reference conflicts. The PR body lists the CG follow-up for the lead to file in Linear. |
| FND-011 | fixed | The workspace-manifest rule is removed and T12-A's rule data is back to main's. The text that was underspecified is gone. The hole it closed is recorded as FND-019. |
| FND-012 | fixed | AC-9 states `make ci` behaviour while QSL ships both implementations, plus an inspection for a tolerance setting. The IR-590 and IR-593 timing moved to AD-001. |
| FND-013 | fixed | The std and `no_std` runs are two separate builds that pair their records by `case_id`. The new admission gap is FND-017, and the pairing gap is FND-016. |

Verdict after round 1: not mergeable. One major finding is open: FND-014 (AC-2 contradicts the emitter). FND-015 to FND-021 are minor.

Round 2, reviewed at 9e808875f2e24a120c277cb1fcf60a97e0c266c3.

| FND | outcome | reason |
| --- | --- | --- |
| FND-014 | fixed | FR-402 says the source references carry `authority`, `identity` and the digest, and that `revision_namespace` and `revision` do not bind `package_id`. In AC-2, a changed authority or identity refuses `package_mismatch` and a changed revision admits. TC-1112 step 2 matches. |
| FND-015 | fixed | AC-4's vector is `{"algorithm": "sha256", "digest": "zz"}`, an object, so it reaches `invalid_package_id`. |
| FND-016 | fixed | Generated ids are `gen/000` to `gen/255`. A case's key is (bundle, `case_id`) over the bundles `checked-in`, `generated` and `external`. Admission, pairing and reports use the key. AC-8 runs an external bundle that reuses `fr402/count-3`. |
| FND-017 | fixed | Admission runs once, in the std build. The `no_std` build compiles nothing and evaluates the admitted cases through its own checked-input contract. A case it cannot take is a divergence. The handoff is IR-590's contract (AD-001 S7). AC-9 tests a `no_std` record set that lacks a case. |
| FND-018 | fixed | The runner is a Cargo test target whose dev-dependency on `quire-eval-corpus` enables `charge-log`. The xtask command evaluates nothing itself. AC-9 requires non-null `charges` for `fr402/count-3` in both builds. |
| FND-019 | accepted-no-change | This is main's state (FR-059 at repository granularity, T12-A matching `qsl_replay::`). The section no longer claims that `qsl-replay` is CG's only QSL dependency; it now states what T12-A matches. The owner can open a ticket if the hole should be closed. |
| FND-020 | fixed | FR-402 and AD-001 S7 place the `no_std` implementation in `quire-eval`, consumed by `qsl-eval`. The PR body labels this a lead default from the planner's answer, not an owner answer. The S6 bullet left behind is FND-023. |
| FND-021 | fixed | FR-402 names `tests/runtime_evaluation_cases/scalars.rs`, encodes `OperationClauseKind` in snake case (`requires`, `ensures`) and names the functions `f0`, `f1` and `f2`. |

Verdict after round 2: no major finding is open. FND-022 (low) and FND-023 (medium) are new minor findings. The PR is mergeable once both are fixed, or once FND-023 is deferred to an open Linear ticket.

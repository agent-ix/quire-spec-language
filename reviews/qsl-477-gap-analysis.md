---
id: SR-1259
title: "A5 gap analysis of PR #614 (FR-059, TC-156, FR-051-AC-6, TC-139; deletion of the FR-061 code and FR-059-AC-7's freshness check)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@b9a376cdf5173a9fcc17ca6a0ebf8b5feed5303c; PR #614 diff against origin/main (merge base 8d1deba4); spec/functional/FR-059-check-backend-dependency-direction.md; spec/test-cases/TC-156-check-backend-dependency-direction.md; spec/functional/FR-051-publish-checked-native-handoffs.md (FR-051-AC-6); spec/test-cases/TC-139-publish-checked-native-handoffs.md; spec/native-temporal/tests.md; spec/tests.md; tools/arch-lint/{graph,metadata,main,error}.rs; tools/arch-lint/duplicate_revisions.rs at origin/main (deleted); Makefile"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-059
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-051
    type: reviews
---
# A5 gap analysis of PR #614

## Summary

Ticket: QSL-477 (implementation slice A5). PR: quire-spec-language#614. No plan
bundle names this slice, so this is a manual AC-to-test check using the quoin
gap-analysis method. I also ran `quire coverage --scope . --format tsv` at the
head.

Trace, per unit:

- **FR-059-AC-1 to AC-5, AC-8, AC-9 (TC-156).** These are unchanged by the PR,
  and each is backed by the `graph` and `metadata` tests that still pass
  (make ci log). Coverage reports no unbacked row for any FR-059 AC.
- **FR-059-AC-6 (TC-156).** `tc_arch_lint_metadata_006` resolves QSL's own
  manifest through `edges_for_manifest`, now with no `offline` argument, and
  asserts the QSL -> IR edge on `quire-contract-model`. The retraced
  `tc_arch_lint_direction_009` asserts that `classify` maps `quire-contract-model`
  from IR's git source to `Repo::Ir`. That is the "classified as QSL -> IR"
  half of the AC's second sentence. Both bindings are correct.
- **FR-059 Behavior and Status.** The remaining direction code does what FR-059
  asks: it resolves four manifests, applies the shared-leaf exemption, and
  produces the FB-05 and FB-11 reports with a pass/fail result. The Status
  section no longer lists remaining work, and none remains for FR-059.
  "`graph::classify` classifies a QSL-sourced shared leaf as QSL" is still true
  and still tested, through xtask TC-747.
- **Deleted code against the spec.** At the head, no spec, TC, index row,
  README, AGENTS.md or Makefile help text names `duplicate-revisions`, FR-061,
  TC-158, FR-059-AC-7, `--offline` for arch-lint, `Stale` or exit 4. The
  FR-061 hits in `src/linking/composed/definition_source.rs`,
  `tests/it/composed_definition_source.rs` and FR-048 are QSpec's
  `FR-061-report-orthogonal-results.md`, a different document in another repo.
- **FR-051-AC-6 (TC-139).** This one is now a gap (FND-001). The deleted
  `tc_arch_lint_duplicate_revisions_006` carried
  `#[trace("TC-139", "FR-051-AC-6", "TC-158", "FR-061-AC-4")]` and was the only
  test tracing FR-051-AC-6. At the head, `grep` finds no `.rs` file that
  traces FR-051-AC-6, and `quire coverage` reports
  `unbacked-row FR-051-AC-6`. `arch-lint direction` is the verifier that
  FR-051-AC-6 and TC-139 name. It needs IR/RT/CG checkouts and is not in
  `make ci`, so after this PR nothing in the gate checks that no dependency of
  QSL depends back on QSL. The PR body says TC-139 and FR-051-AC-6 "already
  name `arch-lint direction` as their verifier". That is true, but the PR does
  not say that their only automated backing is deleted.
- **Underspecified code.** None added. Every remaining arch-lint subcommand
  traces to FR-059, FR-060, FR-284 or ADR-013 §2.

## Verdict

Changes requested. FND-001 (medium) leaves a live AC whose verification is
`Test (TC-139)` with no automated test. The fix is one assertion and one trace
tag on an existing real-data test.

The fix is not the narrowed one-copy check that the lead ruled out. It is
FR-051-AC-6's own `arch-lint direction` rule, FB-05/FB-11 through
`graph::check`, run over the graph that `direction` already resolves for
`--qsl`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-051-AC-6 ("QSL's resolved dependency graph is cycle-free, with no dependency of QSL depending back on QSL; `arch-lint direction` (FR-059) verifies it", `Test (TC-139)`) loses its only automated test. The deleted `tc_arch_lint_duplicate_revisions_006` traced TC-139/FR-051-AC-6. At the head no test traces it, `quire coverage` reports `unbacked-row FR-051-AC-6`, and `arch-lint direction` is not in `make ci`. Fix: in `tc_arch_lint_metadata_006`, or a new test beside it, run `graph::check(&edges)` over QSL's own resolved edges. Assert `is_clean()` and that no edge has `to == Repo::Qsl`, and add `#[trace("TC-139", "FR-051-AC-6")]`. Today's lock has no git-sourced QSL package, so it passes. graph tests 002 and 006 are the negative controls. Then mark the FR-051-AC-6 rows in spec/native-temporal/tests.md as passing. | spec/functional/FR-051-publish-checked-native-handoffs.md:128; tools/arch-lint/metadata.rs:208-229; spec/native-temporal/tests.md:102, :133 |

## New findings (disposition pass 1)

Round 1 also reviews the commits added after b9a376cdf that answer no
finding: 977891a6c, 4950778c8, ae3373a40, 2af8ca465, ea51436ef and a7f8047f5.
I ran `quire coverage --scope . --format tsv` again at this head (ea51436ef's
tree; a7f8047f5 changes only ci.yml).

- **Dangling references (2af8ca465, ea51436ef, a7f8047f5).**
  - Outside review records, a `grep` over spec, code, docs, README, AGENTS.md,
    the Makefile, Cargo.toml and `.github/` finds no reference to QSL FR-012,
    IT-004, TC-001 to TC-009, FR-017-AC-4, `fixture-audit`, `fixture_audit`,
    `audit-error-codes` or Plan-001.
  - The FR-012 hits that remain are QSpec FR-012 (operation anchors): ADR-017
    front matter, ADR-017:192 and :613, FR-305:88, and FR-008 front matter.
  - Two plan files keep history lines: Plan-002 plan.md:29 ("LR02 Plan-001
    remains done") and the Plan-005/006/007 logs.
- **Nothing still in use was deleted.**
  - The tool's only caller in the build or tests was the `ci-clean-build`
    step. Its only other caller was the ci.yml step, which a7f8047f5 removes.
  - Every test deleted with the tool traced only deleted ids, plus
    NFR-005-M-1, now an inspection metric.
  - NFR-002's Verification still carries the `workflow_dispatch`-only rule
    that AGENTS.md now cites.
- **spec/tests.md FR-059 rows.** These are true. Coverage shows every FR-059
  AC backed by TC-156's tests, and TC-156 is a row of this root matrix.
  - The archetype does force them. A copy of tests.md without them fails
    `quire validate`: "`functional_coverage` ... requires at least 1" row.
  - They are still not ceremony. They are per-AC status rows for a test case
    this matrix owns, and `quire coverage` checks those statuses. They carry
    no file, version or count tracking.
- **FR-017.** AC-4 and the audit behaviour are removed, and the rest of FR-017
  stands on its own. The coverage rows for FR-017-AC-2 (`Inspection`, no
  symbol) are not new.
- **NFR-005 metric.** It is true at this head. The only external commands
  Rust code launches are `cargo`, `cargo-deny`, `make`, `rustc` and the
  `quire-spec` binary, and no Rust target launches python, node or a shell.
  But the metric is scoped so that it cannot see the case NFR-005 exists to
  catch (FND-003).
- **deny.toml quire-canonical entry (4950778c8, ae3373a40).** It works: at this
  head, `cargo deny --workspace check bans` gives `bans ok`. But no
  requirement owns it, and the file's own header still describes only
  FR-080-AC-2 (FND-002).
- **FR-051-AC-6** no longer appears as an unbacked row in coverage. The total
  goes from 1298 unbacked rows to 1294.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | low | The new `{ crate = "quire-canonical", deny-multiple-versions = true }` entry traces to no requirement, and no automated test backs it. The coder's throwaway proof is not in any log. deny.toml's header (lines 2-12) still says the file is FR-080-AC-2 / TC-205's ban on `inventory`, `linkme` and `ctor`. The Makefile comment on `cargo-deny-bans` says the same and omits `--workspace`. Fix: in deny.toml's entry comment, cite the rule it enforces, ADR-013 §2's one RFC 8785 encoder (`quire-canonical`). Name the entry, and `--workspace`, in the deny.toml header and the Makefile comment. | deny.toml:2-12, :23-24; Makefile:41-58 |
| FND-003 | low | NFR-005's replacement metric counts "Non-Rust verification logic in the Rust test targets, `xtask` and `tools/arch-lint`". Those three are Rust by construction. A non-Rust fixture or qualification check added anywhere else, such as a script under `tools/` run from `make ci`, is not counted, so the metric reads 0 in exactly the state the Statement forbids. The Verification text ("each runs as a Rust test or a Rust tool") would catch it, but the metric would not. Fix: measure the governed set, for example "Fixture or qualification checks whose verification logic is not Rust \| 0 \| 0 \| inspection". | spec/non-functional/NFR-005-rust-verification-paths.md:11, :15-17, :23, :27-28 |

## Dispositions

Round 1, reviewed at a7f8047f5de649bff7e245210f21f3520f5d2505
(`git log b9a376cdf..a7f8047f5`).

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 9ce5f677f |

Round 2, reviewed at 87dde2e41105a8c8c83a74010d91cdc66db7bc43
(`git log a7f8047f5..87dde2e41`). The round adds no new finding.

- **FND-002.** The deny entry's comment and the deny.toml header both cite
  ADR-013 §2. ADR-013:119 reads "One RFC 8785 JCS implementation produces
  every RFC 8785 encoding", so the cite is accurate. The header and the
  Makefile comment name the entry and `--workspace`.
- **FND-003.** The metric now counts the governed set, and Scope defines
  verification logic as "the code that compares a fixture or vector with its
  expected result". That definition makes the metric true. It does not define
  the problem away:
  - It is the ordinary meaning of a fixture assertion.
  - It keeps NFR-005's target case inside the count. A non-Rust script
    anywhere in the repository that compares a fixture or vector with its
    expected output reads 1.
  - It leaves out only code that compares nothing. The `conformance` grep
    checks that each `--exact` Rust test printed its summary line, so that a
    filter matching nothing fails. I read Makefile:262-317: every grep matches
    a `^conformance: ...` line that the Rust test prints after it has done the
    comparing.
  - At 87dde2e41 the only non-Rust files tracked are
    `tools/check-no-committed-binaries.sh` (byte and size checks on tracked
    files) and `tools/check-index-completeness.sh` (ID presence in the spec
    index). Neither compares a fixture or vector, so 0 is true.
  - The commit does not name the two scripts in Scope, although the round-2
    brief said it did. The definition already excludes them, so that is not a
    defect.
  - NFR-005 at this head passes `quire validate`.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-002 | fixed | b9f190dc5 |
| FND-003 | fixed | 341233b3d |

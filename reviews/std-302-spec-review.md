---
id: SR-15620
title: "spec review of STD-302 S9 switch and retirement of the std reference evaluator (QSL PR #1237)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@4e8346155083e1344c75afbc50057ced6615f5fb; spec/functional/FR-405-retire-the-std-reference-evaluator-for-quire-eval.md, spec/test-cases/TC-1124-quire-eval-is-the-only-evaluator-after-the-switch.md, spec/assurance/AD-001-shared-evaluation-core-interface.md, spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md, spec/functional/FR-402-differentially-test-the-no-std-evaluator-against-the-std-reference.md, spec/functional/FR-403-implement-the-no-std-evaluator-in-quire-eval.md, spec/spec.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-405
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1124
    type: reviews
---
# SR-15620: spec review of STD-302 S9 switch and retirement of the std reference evaluator (QSL PR #1237)

## Summary

Ticket: STD-302 (S9 swap IR-593; umbrella IR-584). Diff `origin/main...4e8346155` (main 48108c9c1 is an ancestor of the head; `git ls-remote` confirms the head). Seven files, spec only. The diff adds FR-405 with AC-1 to AC-10 and TC-1124, and makes small edits to AD-001 (retire wording and a Decisions bullet), ADR-011 (row X-17), FR-402 (a pointer paragraph), FR-403-AC-9 ("Until FR-405's switch") and spec.md.

The reviewer checked the claims against QSL `origin/main` 48108c9c1:

- The items FR-405 deletes: `Machine` is `pub(crate) struct Machine` at `qsl-eval/src/value/expression/evaluate.rs:578`, with submodules `evaluate/invariants.rs` and `evaluate/union_case.rs`. The FR-401 std reference adapter and FR-403's separate `quire-eval` entry are specified but not yet implemented. Neither `qsl-eval` nor any other workspace crate names `quire_eval`. So AC-4's absence checks are about a future state, which is what the spec intends.
- The clause families run on `Machine`. `Machine` evaluates `NodeKind::Pre` (evaluate.rs:1398), `History` (1845), `PopulationObjects` (2046) and `NamedPopulation` (2102). The clause entries reach it through `Machine::with_pre` in `s6a/protocol_clause.rs:192`, which yields the FR-265 trail, and in `value_invariants.rs:355`. `family.rs:218` uses `Machine::new`. The author's claim is right: retiring `Machine` needs a `quire-eval` clause entry.
- What else `qsl-eval` holds that the switch must handle: the clause orchestration and the ADR-031 decision-path walk `resolve_quantifier` (`value/expression/mod.rs:932-985`, which matches `NodeKind::Query/Let/If/Not/Connective`), and the FR-380-AC-7 bound helpers `authored_bound_refusal` and `refused_evaluation` (evaluate.rs:333, 343), which `evaluate_clause` calls. It also holds `Evaluation`, defined in evaluate.rs and moved by X-14; the clause and refinement causes; and the automaton, fairness, hyper, simulation, temporal and model-subject engines.
- Other evaluators in the workspace: the root crate's `src/state/evaluation.rs` (FR-046/047/049 exact evaluation over admitted v1/v2 artifact value graphs) and `src/temporal.rs`. They evaluate artifact IR, not C's checked terms, and ADR-011 M-6c schedules their deletion.

Units examined: the FR-405 statement and its sections "The switch criterion", "One switch change", "The state after the switch", "Clause evaluation", "The corpus after the switch" and "The umbrella's completion"; FR-405-AC-1 to AC-10; TC-1124. Context only: FR-262-AC-9, FR-402-AC-6 to AC-10, FR-403's scope, its "What qsl-eval keeps" section and AC-9, AD-001's "RT and CG acceptance" section, and NFR-015's crate table.

These claims hold:

- AC-6 is a strong vector. The pre observation (`a`, `b`) and the post observation (`a`, `b`, `c`) differ, so a clause entry that reads the wrong observation fails the test. AC-7 and AC-8 have concrete, falsifiable vectors. AC-8 and AC-9 cover the `missing_expected` rule.
- Retiring the std path rather than demoting it is consistent with AD-001's purpose and the repo's no-compatibility-layer rule.
- No status wording appears beyond FND-007.

Lead defaults (none is an owner answer):

1. Retire, not demote: sound, but the grounding needs a label (FND-009).
2. Clause families move to a separate `quire-eval` clause entry, in one switch change: the direction is sound and follows from default 1. The entry itself is unspecified (FND-001).
3. RT acceptance is outside the switch criterion: this conflicts with AD-001's RT acceptance bullet (SR-15621 FND-003).
4. `evaluator-regression`, with a null `expected` failing, CG included: sound. CG FR-044 must record non-null expectations before AC-9 can pass (planner).
5. Generated cases are checked in and the generator stays: sound.
6. "`qsl-eval` names no `NodeKind` variant" as the inspection proxy: unsound as written (FND-002).
7. The switch base is the parent of the switch change: sound. The base evidence needs a record (FND-005).
8. The ids skip FR-404 and TC-1122/1123: fine. The ids are unique (SR-15621).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | major: The "Clause evaluation" section puts SHALL obligations on a `quire-eval` clause entry and a clause extension of the environment contract that no requirement defines. Undefined are the extension's reads (pre, current and per-read observations, history fields, refinement population objects, named populations), the events reported for the FR-265 trail, the FR-106 check 6.5 provenance, the ADR-031 witness and the stop report, and the entry's signature. Only AC-6 tests the entry, with one vector. The section also fails to place the causes. FR-403's table keeps `HistoryUnresolved`, `NamedPopulationIncomplete`, `ModelQueryRefusal`, `ProtocolClauseSnapshot`, `ProtocolClauseUnsupported` and `ProtocolClauseFrameRefusal` (which carries `qsl-semantics`' `AdmissionRecord`) in `qsl-eval`. Item 3 has `quire-eval` raise these refusals "with the same catalog codes", and item 2 lets it name no QSL type. Judgement: FR-405 can merge with the obligation stated, and no split is needed. Criterion 5 is testable at the switch head through AC-4 (once FND-002 is fixed) and AC-5. Before merge, though, the section must change from an unreviewed design to a dependency. <br>Fix: name the clause-entry requirement as a prerequisite, with its own STD ticket and spec cycle before IR-593 starts. Keep only the constraints (one evaluator; the `Value`-family Begin keeps its fault; no QSL type in `quire-eval`). State that the placement of the clause causes is decided by that requirement. | spec/functional/FR-405-retire-the-std-reference-evaluator-for-quire-eval.md:137-163 |
| FND-002 | high | major: AC-4's proxy "`qsl-eval`'s shipped source names no `NodeKind` variant" conflicts with code that FR-405 says `qsl-eval` keeps. `resolve_quantifier` (`qsl-eval/src/value/expression/mod.rs:932-985`) is the ADR-031 SW-2 decision-path walk used by separation, and it matches `NodeKind::Query`, `Let`, `If`, `Not` and `Connective`. It does structural navigation, not evaluation. `qsl-replay/src/witness/derivation.rs:70-93` does the same walk in layer 6. The AC would therefore fail, or force a move that no spec places. This is what the PR lists as "not verified against code". <br>Fix: narrow the proxy to "no evaluation dispatch on a node kind" (for example, no `NodeKind` match that computes a value), or name the decision-path walk as allowed. Alternatively, state where it moves (C, or shared with the `qsl-replay` derivation). | spec/functional/FR-405-retire-the-std-reference-evaluator-for-quire-eval.md:134-135, spec/functional/FR-405-retire-the-std-reference-evaluator-for-quire-eval.md:217 |
| FND-003 | medium | minor: AC-5's "no edit to any expected value" does not limit test deletion. The switch has to delete or rewrite tests whose subject it deletes: FR-402's std and `no_std` pairing tests (FR-402-AC-6, AC-7 and AC-9 name the std implementation), FR-403's "each evaluator" comparisons, and the FR-090/FR-096 probes inside the deleted module (`evaluate/invariants.rs`). Deleting a test removes its expected values, so as written AC-5 lets a failing test be deleted. <br>Fix: list the tests the switch may delete (those whose only subject is the std run, the pairing or the separate entry). Require every other test at the switch base, including the in-module probes once ported, to exist at the head with unchanged expected values. Make the check a base/head test-inventory diff in TC-1124 step 5. | spec/functional/FR-405-retire-the-std-reference-evaluator-for-quire-eval.md:218, spec/test-cases/TC-1124-quire-eval-is-the-only-evaluator-after-the-switch.md:23 |
| FND-004 | medium | minor: AC-10's "production path that evaluates on `quire-eval`" is undefined. FR-403 requires `qsl-eval` to ship a public entry that runs a call through `quire-eval`, and that entry sits on `main` next to `Machine` before the switch. Read literally, AC-10 is already violated by FR-403's own design. <br>Fix: define the production path as `call`, `evaluate`, `call_supplied`, `evaluate_closed` and the clause, clause-set, frame, separation and value-invariant entries. State that FR-403's differential entry is not one. | spec/functional/FR-405-retire-the-std-reference-evaluator-for-quire-eval.md:96-99, spec/functional/FR-405-retire-the-std-reference-evaluator-for-quire-eval.md:223 |
| FND-005 | medium | minor: AC-1 and AC-2 are measured at the switch base and verified as "Test (TC-1124)". The same applies to AC-6's and AC-7's "the switch base's reference recorded". After the switch the std side no longer exists, so these tests cannot run again, and no AC says where the base evidence is kept. The AC-6 charge expectation also has to be recorded as data at the base. <br>Fix: require the switch change to record the base sha and the AC-1 and AC-2 run results (for example in the PR description or a checked-in evidence file), and change AC-1 and AC-2 to Inspection of that record. Require AC-6's expected charges to be checked in as a fixture produced at the base. | spec/functional/FR-405-retire-the-std-reference-evaluator-for-quire-eval.md:214-215, spec/functional/FR-405-retire-the-std-reference-evaluator-for-quire-eval.md:219 |
| FND-006 | low | minor: The claim that `quire-eval` "is the only code in the QSL workspace that computes the value of a checked term" does not mention the root crate's native-lane evaluators (`src/state/evaluation.rs` for FR-046/047/049 and `src/temporal.rs`). Those evaluate artifact value graphs, not C's checked terms. A reader checking the "one evaluator" purpose will find them. <br>Fix: add one sentence placing them outside FR-405 and pointing to their retirement under ADR-011 M-6c. | spec/functional/FR-405-retire-the-std-reference-evaluator-for-quire-eval.md:103-109 |
| FND-007 | low | minor: Time-relative and changelog wording: "the protocol-clause refusals that clause evaluation raises today" and "the name `evaluator-differential` no longer exists". <br>Fix: "the refusals FR-107 and FR-218 define, with their catalog codes, fields and locations"; "no `xtask` command or `make` target named `evaluator-differential` exists". | spec/functional/FR-405-retire-the-std-reference-evaluator-for-quire-eval.md:157, spec/functional/FR-405-retire-the-std-reference-evaluator-for-quire-eval.md:181 |
| FND-008 | low | minor: AC-4 names the module `qsl_eval::value::expression::evaluate`, but that path is private (`qsl-eval/src/value/mod.rs:13` has `mod expression;` and `expression/mod.rs:16` has `mod evaluate;`). The module also holds clause helpers that must survive: `authored_bound_refusal` and `refused_evaluation` (evaluate.rs:333, 343), which `evaluate_clause` calls for FR-380-AC-7. The list of what `qsl-eval` keeps does not mention them. <br>Fix: name the file `qsl-eval/src/value/expression/evaluate.rs` and its submodules, and add the FR-380-AC-7 bound check to what `qsl-eval` keeps. | spec/functional/FR-405-retire-the-std-reference-evaluator-for-quire-eval.md:110-114, spec/functional/FR-405-retire-the-std-reference-evaluator-for-quire-eval.md:217 |
| FND-009 | low | minor: Lead default 1 is "grounded in Peter's Q-1 ('QSL switches its workspace to it', RT deletes its port with no shim)" and "unified core, not multiple implementations". These quotes come from memory or relay, not from a recorded owner ruling in the spec or the ticket. The AD-001 Decisions bullet now records "retires; it does not demote" as settled, which closes AD-001's earlier open "retire or demote". <br>Fix: in the PR body, label the quotes as recalled from memory or relay and unverified, and ask the owner to confirm the retire decision before merge. | spec/assurance/AD-001-shared-evaluation-core-interface.md:298-305 |

## Verdict

Not mergeable as is. Two major findings: FND-001 (the clause entry is unspecified and must be recast as a named prerequisite; no split needed) and FND-002 (AC-4's `NodeKind` proxy conflicts with retained separation code). FND-003 to FND-009 are minor. The integrity analysis (SR-15621) adds two more major findings.

Owner and planner decisions:

- Confirm retire rather than demote (lead default 1; the quotes are from memory).
- Open the STD ticket for the `quire-eval` clause-entry requirement (FND-001), and decide where the clause causes live.
- Decide whether RT's differential acceptance has to run before S9 (SR-15621 FND-003).
- CG FR-044 must record non-null expectations through `quire-eval` before FR-405-AC-9 can pass; CG owns that.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-010 | low | minor: FR-405-AC-4 is narrower than the Behavior text it checks. Behavior allows any "structural walk over checked terms that computes no value". AC-4 and TC-1124 expected result 4 instead require that "the `NodeKind` matches left in `qsl-eval` are the ADR-031 SW-2 decision-path walk". The clause orchestration that the switch rewrites (for example a clause-set or frame walk, or `composite_site`-style local matching) could legitimately add another structural match, and that would fail AC-4 while meeting the Behavior. <br>Fix: make AC-4 say "every `NodeKind` match left in `qsl-eval` is a structural walk that returns no value, such as the ADR-031 SW-2 decision-path walk". | spec/functional/FR-405-retire-the-std-reference-evaluator-for-quire-eval.md:187-195, spec/functional/FR-405-retire-the-std-reference-evaluator-for-quire-eval.md:252 |

## Dispositions

Round 1, reviewed at 591355c8cfc0678ec05bbcbb7cfdb555ef4e7081.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | "Clause evaluation: a prerequisite" names a separate requirement tracked by STD-318 (it exists in Linear, Backlog). That requirement owns the entry, the extension's reads and events and the placement of the clause causes, all listed. FR-405 keeps three constraints only. |
| FND-002 | fixed | The proxy is now no task or value stack and no `Value`/`Outcome`/`Evaluation` except through a `quire-eval` session, with the SW-2 walk named as allowed. The residual narrowness in AC-4 is FND-010. |
| FND-003 | fixed | "Tests at the switch" limits deletion to three classes and requires porting the in-module probes. AC-5 is a base/head test-inventory diff, and TC-1124 step 5 builds it. |
| FND-004 | fixed | "One switch change" lists the production evaluation paths and excludes FR-403's separate entry, and AC-10 repeats this. |
| FND-005 | fixed | The switch change's description reports the switch base and the item 1 and 2 results. AC-1 and AC-2 add Inspection of that report. AC-6 compares against a checked-in fixture produced at the base. |
| FND-006 | fixed | State item 1 places `src/state/evaluation.rs` and `src/temporal.rs` outside FR-405, under ADR-011 M-6c. |
| FND-007 | fixed | "today" and "no longer" are gone; the added lines of the diff carry no time-relative wording. |
| FND-008 | fixed | AC-4 and state item 2 name the file `qsl-eval/src/value/expression/evaluate.rs` and its submodules. The FR-380-AC-7 bound check moves into the clause orchestration and is in the list of what `qsl-eval` keeps. |
| FND-009 | fixed | The PR body labels the Q-1 quotes as the planner's relay of Peter's earlier ruling, unverified by the lead. The owner still has to confirm retire rather than demote. |

Verdict after round 1: no major finding is open. FND-010 (low) is a new minor finding.

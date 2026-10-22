---
id: SR-12317
title: "Code review (Rust lane) of QSL-570 part E: twelve capability kinds, descriptor evidence kinds, EN-4 manifest, reference-support refusal (PR #1204)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@65a31e407; qsl-route/src/{lib.rs,request.rs}, qsl-semantics/src/check/{capability.rs,probabilistic/model.rs}, src/linking/composed/requests.rs, qsl-analyze/src/{statistical.rs,statistical/manifest.rs,statistical/tests.rs}, qsl-route/tests/it/*, qsl-analyze/tests/it/{statistical_manifest.rs,statistical_model.rs}, qsl-semantics/tests/it/probabilistic_declarations.rs, tests/it/*"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-280
    type: reviews
---
# SR-12317: Code review (Rust lane) of QSL-570 part E: twelve capability kinds, descriptor evidence kinds, EN-4 manifest, reference-support refusal (PR #1204)

## Summary

Ticket: QSL-570. PR quire-spec-language#1204, branch qsl-570e-impl, head 65a31e407, two commits. The scope is `git diff origin/main...HEAD`: 26 files, +1189/-40. The rust-review lane is folded into this file. The review gave priority to the two code changes and to the registry's admission and routing semantics.

Gates run on the PR head, with the build in the review worktree's own `target/`:

- `make string-edge`: pass.
- `make route-lint`: pass, no static items in `qsl-route/src`.
- `make arch-lint-api-surface-qsl`: pass, T12-B to T12-F and FR-100-AC-8.
- `make seam-probe`: **fails**, exit 2: `checked-in list and the probe build's E0004 locations differ -- unexpected-but-present: [SeamLocation { file: "qsl-route/src/lib.rs", item: "BackendDescriptor::serves_evidence" }]` (FND-003). `seam-probe` is in `CI_CHECKS`, so `make ci` is red on this head.
- The focused new tests: 14 passed.
- Mutations: see SR-12318's Summary. Every new registry behaviour is caught by a test except the three FR-057 rules this PR adds. The reference-support guard is caught by no test.

The lead's questions:

- **(a) Mutations.** Each of these is caught by a test: unknown-evidence admission; the duplicate conflict over an evidence difference; the rule that `candidates_for_evidence` never substitutes `exact` for `statistical`. The reference-support guard is caught by none (SR-12318 FND-001). The guard cannot fire today. S3 has no reference-literal form, and `literal()` returns only a `NodeKind::Literal`, which the check typer never builds holding a `Value::Reference`. The guard is a correct rule waiting for that form, so it may stay. What must change is the claim in the test and in the PR body.
- **(b) `BackendDescriptor::admit`.** The code is right and the spec is stale (SR-12319 FND-001, with every occurrence listed).
- **(c) Public API with no production caller.** `ItemEvidence`, `Registry::candidates_for_evidence`, `CandidateSet::evidence` and `qsl_analyze::statistical::manifest::descriptor` have no caller outside tests. They pass the value test. Each one is the library surface that FR-075-AC-13 and FR-280-AC-4 state behaviour for, and tests pin it (mutation M5, M6 and M10 are caught). The missing production path is a real, cited gap: `requests::families` gives `&[]` for the probabilistic kind, and no requirement record carries an evidence kind. The PR records that gap as Remaining work under QSL-570. This is wiring in advance of a cited consumer, not dead code.
- **(d) Changelog.** The PR body lists the new variants, the new enums, the signature changes and the new methods. It leaves out one behaviour change (FND-002).
- **(e) `Quantity<ms>`.** See SR-12318 FND-003. The Remaining-work reason is stale; the real blocker is the sampler's lack of a quantity parameter encoding.

Rust lane notes:

- `serves_evidence` and `same_kind` match exhaustively, with no wildcard. `serves_evidence` is a new S7 seam location, and it is missing from the checked-in list (FND-003).
- `fr290_family` is generated from the same macro row as `to_wire` and carries the seam-probe arm.
- `EvidenceKind::from_wire` derives from `ALL` and `to_wire`.
- Refusals carry the received bytes.
- No new `unwrap` or `panic` in production code.
- The new `pub` items are documented.

## Verdict

**Not mergeable: one major finding (FND-003). `make seam-probe` is red, so `make ci` fails on this head.** Once FND-003 is fixed, the remaining findings are mediums and lows, all to be fixed in this PR.

The registry semantics are correct against FR-075-AC-10 to AC-14 and QSpec FR-290 "Evidence kinds":

- Evidence is checked only for a backend advertising `probabilistic-satisfaction`.
- An absent set, an empty set, or an unknown label refuses `unknown-evidence-kind`, keyed by identity, before insertion.
- An evidence difference conflicts through the derived equality.
- Candidacy requires the exact kind.
- Absent or unknown item evidence matches nothing and is kept on the set.

The EN-4 manifest is the FR-280 data, owned by `qsl-analyze`, and it is converted by FR-288's one conversion. The required fixes are SR-12318 FND-001 to FND-004 and SR-12319 FND-001 and FND-002. This file adds FND-003 (high) and two low findings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `candidates_for_evidence` accepts any `kind`, but it always passes empty fairness and memory-model sets to `compute`. For a `temporal-satisfaction` item, calling it silently skips FR-368's fairness filter and FR-439's memory-model filter, which `candidates_for_fairness` and `candidates_for_models` apply. The doc says "A kind other than `probabilistic-satisfaction` matches by kind alone", which states the behaviour but invites the misuse. Either take the probabilistic kind only (drop the `kind` parameter), or say in the doc that temporal items must use the fairness and model entry points. | qsl-route/src/lib.rs:1101-1110 |
| FND-002 | low | The changelog leaves out a behaviour change. `Registry::candidates`, `candidates_for_fairness` and the memory-model entry now give an empty set for `probabilistic-satisfaction`, because they pass `ItemEvidence::Absent`. Before this PR they matched by kind alone. A probabilistic descriptor's equality, and so its duplicate-backend conflict, now also includes the evidence set. Add both to the changelog line, with `EvidenceKind::{ALL, to_wire, from_wire}` and `ItemEvidence::from_label`. | qsl-route/src/lib.rs:1057-1093,1161-1166 |
| FND-003 | high | `make seam-probe`, a `make ci` gate, fails on this head. The new closed `match kind` in `BackendDescriptor::serves_evidence` fails to compile under `--cfg seam_probe`, as intended: it has no wildcard, so a new `Capability` variant breaks it. But `xtask/src/seam_probe.rs` `checked_in_locations()` does not list it, so the probe reports `unexpected-but-present: BackendDescriptor::serves_evidence` and exits 2. Two fixes work. Either add `SeamLocation { file: "qsl-route/src/lib.rs", item: "BackendDescriptor::serves_evidence" }` beside the `same_kind` entry, which keeps the forcing match. Or write the test as `matches!(kind, Capability::ProbabilisticSatisfaction)`, with no closed match. Then rerun `make seam-probe`. | qsl-route/src/lib.rs:494-516; xtask/src/seam_probe.rs:179-196 |

## Dispositions

Round 1, fixed in f4af7d95e.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | f4af7d95e: doc says temporal items use candidates_for_models; no filters applied here |
| FND-002 | fixed | f4af7d95e: PR body changelog extended |
| FND-003 | fixed | f4af7d95e: serves_evidence is now a matches!, no closed match; make seam-probe green |

Round 1, checked by the reviewer at 369bd13b1 (fix f4af7d95e). Each row was measured against the code and replaces the coder's claim above.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | f4af7d95e: the `candidates_for_evidence` doc now says it applies no fairness or memory-model filter and sends a temporal item to `candidates_for_models` |
| FND-002 | fixed | f4af7d95e: the PR body changelog now lists the empty-set behaviour of the three other entry points, evidence in descriptor equality, `EvidenceKind::{ALL,to_wire,from_wire}` and `ItemEvidence::from_label` |
| FND-003 | fixed | f4af7d95e: `serves_evidence` now uses `matches!` with no closed match over `Capability`; the reviewer's own `make seam-probe` run at 369bd13b1 exits 0 |

---
id: SR-919
title: "QSL-339 code review (comment edits, rust-review lane) of PR 544"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@52a4c1ab7b8d7e218029b46c67eda7ea6ef6c498; every .rs, .sh, .toml and Makefile hunk in origin/main...HEAD (about 230 files, comment lines only); remaining QSL-NNN ids in code, string literals, allow/expect reasons and toml values at HEAD; non-spec markdown (qsl-bench/BASELINE.md, examples/, tests/fixtures/)"
review_set: subset
---
## Summary

Ticket: QSL-339. PR: quire-spec-language#544 at 52a4c1ab. Gate: make ci
exit 0 on 52a4c1ab (the coordinator's log; not re-run). No builds were run.

What was checked:

- **Only comments change.** Every added or removed line in a .rs, .sh, .toml
  or Makefile hunk starts with `//`, `#` or `*` after whitespace. No changed
  line is a `#[...]`/`#!` attribute, and no `/*` line changed. The toml and
  Makefile hunks were read in full: each one is a `#` comment. Behaviour does
  not change.
- **Rewrites keep their meaning.** Sampled: Cargo.toml X-2..X-9 labels,
  Makefile target comments, qsl-package checked.rs M-4/#242, xtask
  string_edge.rs X-2..X-10, and the seam_probe.rs #185 pointers. All are
  correct.
- **Ids kept on purpose.** QSL-42/43/238/265/285/290/130/133 are open
  tickets, so they stay. QSL-22 is the exemplar name. The QSL-25/26/158/277
  ruling citations stay. "ADR-013 ruling (QSL-278 r6)" in
  qsl-replay/src/spine/clause/tests.rs:1179 is accepted as a ruling citation.
- **String-literal edits are safe.** Each `fault_reason` string is compared
  only with itself (emit/tests.rs:2002, clause/tests.rs:3923), so changing
  its text is safe.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Three `#[allow(dead_code, reason = ...)]` strings say "ADR-011 §4's round trip (QSL-6 slice S3) wires this reader in". QSL-6 is Done, and these items are still uncalled, so the reason points at closed work as the future owner. That is false. Name the real remaining owner, or say plainly that nothing calls it yet. | qsl-package/src/checked_v2.rs:457; qsl-semantics/src/library/mod.rs:724; qsl-semantics/src/library/package_identity.rs:155 |
| FND-002 | low | The sweep missed toml comments that still carry Done ids: `(QSL-194)` twice and `# QSL-206:`. The same pattern was removed in qsl-package/Cargo.toml and qsl-eval/Cargo.toml. | qsl-semantics/Cargo.toml:33,48,58 |
| FND-003 | low | This is one of the "unsure" set. The comment says "the remaining real sites are tracked as O-18 debt on QSL-26" and "Recorded on QSL-26 for whoever can amend that decision". QSL-26 is Done, so this is a tracked-on pointer to closed work, which is ceremony. Delete it, or point at an open owner if the debt is real. | qsl-foundation/src/digest.rs:108,115 |
| FND-004 | low | This is one of the "unsure" set. "QSL-148's own ticket text asks to move ... termination" retells the scope history of a Done ticket. The rule it explains (termination stays the whole-package pass) is already in ADR-012 §14.1 and FR-065. Cite those instead. | qsl-semantics/src/check/family.rs:1026 |
| FND-005 | low | String literals and toml values still carry Done ids as history: "since QSL-200" (probe output), "the pre-QSL-194 encoder" (assert message), "QSL-313 fault injection" (two test strings), arch-lint reasons "QSL-251:" and lint output "QSL-194", the qsl-bench description "QSL-196:", and the quire-exact description "QSL#213 S-1". All are ceremony and safe to edit. | qsl-bench/src/bin/qsl-bench-probe.rs:111; qsl-semantics/tests/it/identity_golden_vectors.rs:111; qsl-package/src/emit/tests.rs:1985; qsl-replay/src/spine/clause/tests.rs:3907; tools/arch-lint/canonical_encoder.rs:232,243,1103; qsl-bench/Cargo.toml:8; quire-exact/Cargo.toml:8 |
| FND-006 | low | Non-spec markdown still carries Done ids as history: qsl-bench/BASELINE.md (46 ids, e.g. "QSL-200 has landed", "QSL-201's ... changed intake"), examples/config-version/README.md:23 "FR-108 (QSL-314)", and tests/fixtures/native-package/README.md:29 "QSL-233 (ADR-013 §7 S-4b) regenerated". The measurement file names BASELINE.md cites can stay. | qsl-bench/BASELINE.md; examples/config-version/README.md:23; tests/fixtures/native-package/README.md:29 |
| FND-007 | low | Bare "G2" replaced "QSL-248 (G2)" in code comments. It is undefined, and it collides with FR-092's golden vector G2, which qsl-semantics/src/check/lowering/tests.rs:465,483 uses. The spec side is SR-918 FND-009. | qsl-eval/Cargo.toml:35; qsl-eval/tests/it/dispatch_calls.rs:649; qsl-package/src/emit/tests.rs:1492; qsl-semantics/src/family/contract.rs:257; tools/arch-lint/api_surface.rs:269,1543,2204; xtask/src/typestate_scan.rs:977 |

## Verdict

The code side of the diff changes comments only, and the rewrites keep their
meaning. One medium finding: three dead-code allow reasons name a Done ticket
as the future owner. The low findings are leftovers the sweep missed or kept
without cause. No behaviour changes, so gap analysis has nothing to trace.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | low | The three rewritten dead-code reasons say the ADR-011 §4 round trip is one "which no open ticket owns". QSL-347 ("ADR-011 §4 I2 round trip: wire the v2 reader into a real consumer, or delete the three unused readers", Backlog) was filed on 2026-10-01, so the reasons are now false. Name QSL-347: it is an open-work pointer, which the keep rule allows. | qsl-package/src/checked_v2.rs:457; qsl-semantics/src/library/mod.rs:724; qsl-semantics/src/library/package_identity.rs:155 |

## Dispositions

Round 1, reviewed at bd660f04c13517e9735b98f2941c90556f9f7036 (fix commit a0326826). Coordinator's capped make ci on a0326826: exit 0 (not re-run). Non-comment code changes in a0326826 are string-literal and toml `description` edits only. Each `fault_reason` is compared only with itself. The arch-lint `reason` strings are allow-list prose.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | a0326826: all three reasons now read "unused pending ADR-011 §4's round trip, which no open ticket owns". QSL-6 is no longer named as the owner. QSL-347 was filed afterwards; see FND-008 |
| FND-002 | fixed | a0326826: qsl-semantics/Cargo.toml has no QSL id left |
| FND-003 | fixed | a0326826: digest.rs no longer says "tracked/recorded on QSL-26" |
| FND-004 | fixed | a0326826: family.rs no longer retells QSL-148's ticket text. The only QSL-148 left is the testing-policy ruling link at :1897, which is kept as a ruling citation |
| FND-005 | fixed | a0326826: the probe output, assert message, fault strings, arch-lint reasons and output, and the qsl-bench and quire-exact descriptions carry no QSL-NNN id |
| FND-006 | fixed | a0326826: qsl-bench/BASELINE.md, examples/config-version/README.md and tests/fixtures/native-package/README.md carry no QSL-NNN id |
| FND-007 | fixed | a0326826: the code comments in qsl-eval/Cargo.toml, dispatch_calls.rs, emit/tests.rs, family/contract.rs, api_surface.rs and typestate_scan.rs no longer use a bare G2 |

Round 2, reviewed at dffebe8741536681fa0484ad4ea25e07b7533b4e (fix commit dffebe87). Coordinator's cargo check --workspace: clean (not re-run). The only code change is the three `reason` strings.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-008 | fixed | dffebe87: all three dead-code reasons name "ADR-011 §4's round trip (QSL-347)". QSL-347 is open (Backlog) and owns that round trip |

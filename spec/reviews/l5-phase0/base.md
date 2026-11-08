---
id: SR-2611081
title: "Base review for the L5 phase-0 batch: JIT, parity gate, lifecycle matrix, final-path qualification and clean-clone build"
type: SpecReview
analysis: base
scope: "Diff of spec/l5-phase0-batch against origin/main: FR-560 to FR-567, NFR-017, NFR-018, ADR-035, US-037, TC-1380 to TC-1399, and the edits to ADR-029, FR-299, US-029 and US-030"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-564
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-565
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-567
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-562
    type: reviews
  - target: ix://agent-ix/quire-spec-language/NFR-017
    type: reviews
  - target: ix://agent-ix/quire-spec-language/NFR-018
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-035
    type: reviews
---

## Summary

Base checklist run over the diff only, with the cited existing requirements
(FR-100, FR-185, FR-275 to FR-299, FR-350, FR-353, ADR-029) read to check
consistency. IDs are unique and sequential. Every new acceptance criterion has a
TC row and a TC whose scope names it. The FR-564 matrix arithmetic holds:
6 verbs times 8 kinds is 48 cells, of which 8 are n/a and 40 are cases, and the
per-verb counts in FR-564-AC-1 to AC-6 (6, 5, 7, 7, 7, 8) agree with the table.
The FR-562 boundary values are correct: 2^62 - 1 + 2^62 = 2^63 - 1,
3037000499^2 = 9223372030926249001 fits and 3037000500^2 does not, and
-2^63 div -1 = 2^63 overflows. The FR-185-AC-7 `package` unsupported cell is
consistent with FR-280 (S4 refuses to emit a random-parameter package while the
checked package remains). All agent-ix git sources in `Cargo.lock` are public
repositories, so NFR-017's source rule matches today's graph. Verification cells
written as `Test (TC-nnn)` follow the repository convention and are not findings.
No hash, digest, pin, ledger or tracking antipattern appears in the diff.

The defects are an incomplete matrix that claims to be total, a qualification
requirement that contradicts the unamended FR-353, a release-tag criterion that
cannot pass at the existing newest tag, and an FR-562 case the checker refuses.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-564 says the matrix covers "every outcome kind a user can meet", but its eight kinds leave out four FR-285 categories that the same verbs return: undefined (an `execute` `Undefined` such as FR-100-AC-10's `sum-out-of-domain`, exit 10; a `monitor` `InsufficientData`, FR-286-AC-12), pending (FR-283-AC-1's second half, exit 0), indeterminate (FR-501-AC-4 through `monitor`, exit 11) and inconclusive (FR-281-AC-4, AC-5 and AC-15 through `analyze` and `provider::run`, exit 22). n/a reason 4 for `execute`/violation also ignores `Undefined`. Fix: add the four kinds with their cells and n/a reasons, and update FR-565's 48/40/8 counts, FR-564-AC-1 to AC-7 and TC-1388 to TC-1394; or narrow the claim and say which categories the matrix leaves out and why. | FR-564, FR-565-AC-1, FR-285, FR-286 |
| FND-002 | high | FR-567 contradicts FR-353, which this PR does not amend. FR-353 says the gate reports qualified "if and only if every capability in that scope is `passed`", that exit 2 means "refused before any vector executes", and that "exit 2 does not replace a completed run's verdict". FR-567 adds two required parts and makes a parity `reference-fault`, raised after the corpus vectors have run, a refusal with exit 2, while saying "the exit contract stays FR-353's". Fix: amend FR-353's iff clause, exit contract and FR-352 report members in this PR, and either make `reference-fault` a failing finding (exit 1) or write the mid-run exception into FR-353. | FR-567, FR-567-AC-4, FR-353, FR-352 |
| FND-003 | high | NFR-018-AC-1 cannot pass at the repository's newest tag, `v0.2.0`. NFR-017's gate step 4 fixes today's commands (the `emit_checked_v2` example and the walkthrough test), and neither exists at `v0.2.0`, whose README documents `fixture-audit` commands, some needing a QSpec path. AC-1 also says the examples are "as NFR-017-AC-2 states for that commit's README", which conflicts with NFR-017's fixed list. Fix: have `release-check` run NFR-017 steps 1 to 3 and 5 (locked build, sources, cleanup) at a tag and not step 4, or read the example list from the tag's own gate; say that the gate script comes from the current checkout. | NFR-018-AC-1, NFR-017 step 4 |
| FND-004 | high | FR-562-AC-6 cannot be reached. `s(a: Int[0, 9], b: Int[0, 9]): Int[0, 9] { a + b }` is refused at `check` with `unproved-range`, because linked checking proves result ranges (FR-100, "Linked checking proves..."). No entry exists for the JIT to prepare, so the AC and its TC-1384 step 5 cannot pass as written. Fix: check `s` under `CheckMode::Kernel`, as FR-100-AC-10 does, or use a construct whose range obligation is checked at run time, and state the interpreter's expected outcome and cause. | FR-562-AC-6, TC-1384, FR-100 |
| FND-005 | medium | The `prove` (QSL half)/unsupported cell, "FR-281-AC-3's unsupported item through `provider::run`", cannot be produced. `provider::run` takes `RoutedItem`s already routed to a QSL engine (FR-280), and an item settled `Unsupported` is never routed. Routed to a backend `engines` does not hold, it settles `invalid-request`/`unknown-backend`, not `unsupported`. Fix: use FR-281-AC-11's EN-1 `RequiresBound` case (an `Integer` field root over `Engines::qsl()`), which settles `unsupported` with `unsupported_projection`/`unbounded-extent`. | FR-564, FR-564-AC-6, TC-1393, FR-280 |
| FND-006 | medium | FR-565 defines `run_case` over `Engines::qsl()`, but the `prove`/refusal cell cites FR-281-AC-11's `Admission` refusal, which comes from a test engine that `Engines::qsl()` does not hold. The `execute`/tool-failure cell also needs an injected-fault backend that FR-565 does not place in `qsl-conformance`. Fix: use FR-280-AC-6's undeclared-population binding (`declined`, `invalid-input`, over `Engines::qsl()`) for the refusal cell, and state that the fault-injecting test backend ships in `qsl-conformance` as case data. | FR-564, FR-565, FR-281-AC-11, FR-280-AC-6 |
| FND-007 | medium | The `check`/unsupported cell cites "FR-285-AC-3's profile-gated source", but FR-285-AC-3 and TC-769 step 3 map a constructed `StageFailure` value, and no source exists. Fix: name a concrete source and the selected profile that gates it (exit 21 at `check`). | FR-564, FR-564-AC-1, FR-285-AC-3, TC-1388 |
| FND-008 | low | FR-562 binds "a JIT or AOT backend", but its mechanism (the `KernelTable` of FR-561 and the recording table its ACs use) belongs to the JIT only. AOT is `quire-aot` in the driver repository, generating through CG, and FR-562 has no Overlap for it. Fix: scope FR-562 to `qsl-jit`, or add an Overlap that says how `quire-aot` meets EB-4 rule 2. | FR-562, FR-561, ADR-029 EB-6 |
| FND-009 | low | FR-564's kind table and third Behavior bullet spell document causes as Rust variants (`Deadline`, `Requested`). FR-286 documents carry a `cancelled` diagnostic with cause `deadline` or `requested`, and an `analyze` item carries `cancelled{source: deadline}`. Read literally, the bullet fails the `analyze` and `execute` cells. Fix: state the wire spellings FR-286 and FR-281 give. | FR-564, FR-286-AC-6, FR-281-AC-12 |
| FND-010 | low | ADR-035 CG-1 item 1 lets a system-library candidate pass CG-1 behind a non-default feature, and CG-3 may then pick it as fastest, yet it "cannot be the default JIT". The record does not say what the default build gets in that case. Fix: have CG-3 compare only candidates eligible as default, or state the outcome (the default build supplies no JIT). | ADR-035 CG-1, CG-3 |
| FND-011 | low | FR-567 traces to US-035, but US-035 is not updated to exercise FR-567, and FR-352 (the report that gains three named parts) is not amended. Fix: add the US-035 relationship and amend FR-352, or reference FR-567 from it. | FR-567, US-035, FR-352 |
| FND-012 | low | FR-299's new reference line says FR-564 "extends this source to every outcome kind", but FR-564's cells use other fixtures and not FR-299's source. Fix: reword it to say the matrix extends the first slice's operations. | FR-299, FR-564 |

## Dispositions

Fix round 1, in the same PR:

- FND-001: fixed by narrowing. FR-564 now states the matrix covers the eight named kinds and that undefined, pending, indeterminate and inconclusive stay with FR-285 and FR-286; n/a reason 4 names execute's undefined category.
- FND-002: fixed. FR-353's "if and only if" names FR-567's two parts; `reference-fault` is now a parity finding (exit 1), not a refusal; FR-567 states FR-352's added `parts` member.
- FND-003: fixed. `release-check` runs NFR-017 steps 1 to 4 and 7, not the examples.
- FND-004: fixed. FR-562-AC-6 checks `s` under `CheckMode::Kernel` (FR-495).
- FND-005: fixed. The prove/unsupported cell uses FR-281-AC-11's EN-1 `RequiresBound` item.
- FND-006: fixed. The prove/refusal cell uses FR-280-AC-6's binding refusal; FR-564 states the fault-injecting backend ships in `qsl-conformance`.
- FND-007: fixed. check/unsupported uses FR-218-AC-5's `repeat` whose body is only a `check`.
- FND-008: fixed. FR-562 is scoped to `qsl-jit`, with an AOT Overlap.
- FND-009: fixed. The kinds table gives `CancelCause` for failures and FR-281/FR-291 spellings for items.
- FND-010: fixed. CG-3 compares only default-eligible candidates.
- FND-011: fixed. US-035 exercises FR-567; FR-567 depends on FR-352.
- FND-012: fixed. FR-299's reference line reworded.

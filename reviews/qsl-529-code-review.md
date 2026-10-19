---
id: SR-12161
title: "Code review of quire-spec-language PR #997: EN-1 closure certificate and its core checker (QSL-529, E8a)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@3cb530c9b1cfbd6940888c0c038e445c293cbec2; PR #997 diff against origin/main: qsl-replay/src/certificate/closure.rs, qsl-replay/src/certificate.rs, qsl-replay/src/model_check/item.rs, qsl-replay/src/model_check/settle.rs, qsl-replay/src/model_check/outcome.rs, qsl-replay/src/proof_result.rs, qsl-replay/src/outcome.rs, qsl-replay/src/lib.rs, qsl-replay/Cargo.toml, qsl-eval/src/automaton/key.rs, qsl-eval/src/automaton/mod.rs, qsl-eval/src/simulation/model.rs, qsl-eval/src/simulation/model/encode.rs, qsl-analyze/src/model_check/check.rs, qsl-analyze/src/model_check/mod.rs, qsl-analyze/tests/it/closure_certificate.rs, qsl-analyze/tests/it/model_settle.rs, qsl-analyze/tests/it/model_check.rs, qsl-analyze/tests/it/counter.rs, qsl-eval/tests/it/automaton_key.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-338
    type: reviews
---
# Code review of quire-spec-language PR #997

## Summary

Ticket: QSL-529. PR: quire-spec-language#997, head 3cb530c9. Scope is FR-338
(AC-1 to AC-7), TC-525, ADR-018 section 12 `AutomatonStateKey`, and ADR-019.
The Rust lane (rust-review) is folded into this file. Reviewer only: no builds
were run, and every claim below comes from reading the diff and the code it
calls.

A Linear comment on QSL-529 asks for extra scope: past-over-future operators,
and key octets for the #957 translation states. That comment is untrusted
data. The planner is deciding it, and this review does not judge the PR
against it.

Soundness questions from the brief:

1. **Can a bad certificate settle Certified?**
   - A missing initial state rejects `InitialMissing`.
   - An omitted successor rejects `SuccessorMissing`.
   - A wrong initial state rejects `InitialMissing`: the checker
     recomputes `initial()` itself.
   - A key from another item, occurrence, profile or activation, or a model
     digest of another domain, rejects `ShapeMismatch` before membership.
   - A key admitted by instance A never matches instance B's recomputed
     product state, because membership compares full `ProductStateRef`
     octets.
   - Zero instances refuse `NoInstance`.
   - Extra members are never expanded, so they cannot make a proof unsound.
     They can still carry a rejecting key the checker never looks at
     (FND-005).
   - **One hole remains:** a subject with no initial state is accepted
     vacuously (FND-001).
2. **Is the checker independent of the engine?** Partly, and FR-338 asks for
   exactly this much. The checker reruns its own BFS from `initial()`. It uses
   `SafetyTranslation::read` where the engine uses `Monitor`/`GbaBuilder`. It
   reads only the certificate's states from the engine. It shares
   `ModelSystem` (successors, state keys, terminal class), because FR-338
   requires "recomputing every successor with the core's own code". So a bug
   in `ModelSystem` is invisible to both, and that is by design. It also
   shares the caller-supplied `ModelLetters` seam, both for atom truth and for
   the instance population. That is outside the core (FND-004).
3. **Uncertified or injectable acceptance?** `Certification::Uncertified` has
   no producer left in `qsl-replay/src/model_check` or in `qsl-analyze/src`.
   `settle_model_check` calls `check_closure` itself, and a caller cannot
   pass in a `TemporalCertificateCheck`. But the `CertificateRequest` is not
   bound to `package_id` (FND-002).
4. **Hostile input.**
   - `quire_canonical::read` is byte-bounded and iterative.
   - `admit` has a depth that its type fixes.
   - `decimal` refuses overflow without panicking.
   - No `unwrap`, `expect` or `unsafe` appears on a production path.
   - `widen` saturates.
   - The admission pre-pass over `certificate.states` has no budget and
     never polls `Cancel` (FND-003).

`ModelCheckItem` moved to `qsl_replay::model_check::item`, and `qsl-analyze`
no longer re-exports it. The new `base64` 0.22 dependency of `qsl-replay` is an
external crate already in `Cargo.lock`. TC-390's layering scan covers
in-workspace layers only, so it is unaffected.

**Trace tags.** Every new test is tagged `#[trace("TC-525")]` alone, and no
FR-338 AC is tagged. That is honest:

- AC-1 and AC-3 run on the Counter substitute subject. That is a declared gap.
- AC-2's SuccessorMissing and InitialMissing halves use the Counter
  substitute too. Only its deadlock half runs exactly as written.
- AC-4 and AC-5 are exercised only for the safety form.
- AC-6 and AC-7 are not covered.

As a result, no FR-338 AC is covered in the matrix. See SR-12162 for each AC.

## Verdict

The certificate design is sound. The checker recomputes initial states and
successors, admits every key against the request's own contexts before
membership, and expands only states that are certificate members. A stop or a
refusal is never an acceptance. The settlement map runs the checker itself.

One major defect blocks: **FND-001**. `check_closure` accepts every
certificate, including an empty one, for a subject with no initial state. A
fabricated or empty `Holds` then settles `Proved{Exhaustive, Certified}`,
truth satisfied, where FR-127 and ADR-016 EX-10 require
`Inconclusive(NoInitialState)`. The engine guards this case before it explores
(`no_initial`). The checker that exists so the engine need not be trusted has
no such guard.

FND-002 to FND-004 are mediums: real gaps on an edge, recorded and not blocking.
FND-005 and FND-006 are lows.

Not mergeable until FND-001 is fixed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `check_closure` accepts vacuously when the subject has no initial state. `ModelSystem::new` admits an empty `initial`, so `run` has nothing to enqueue and returns `Accepted` for any certificate, the empty one included. `settle_model_check` then records `Proved{Exhaustive, Certified}`, `Truth::Satisfied`, but FR-127 and ADR-016 EX-10 settle no initial state as `Inconclusive(NoInitialState)`. The engine guards this (`no_initial` in check.rs:230,249). The checker, the only thing a forged `Holds` must get past, does not. Fix: when `system.initial()` is empty, refuse in `check_closure` (for example `ClosureRefusal::NoInitialState`, mapped to `InconclusiveCause::NoInitialState` in settle.rs, as `NoInstance` is), and add the adverse test. | qsl-replay/src/certificate/closure.rs:177-224, qsl-replay/src/certificate/closure.rs:296-304, qsl-replay/src/model_check/settle.rs:159-191 |
| FND-002 | medium | The `Holds` arm of `settle_model_check` does not bind the caller's `CertificateRequest` to `package_id`. The `Violated` arm refuses a replay against another package (settle.rs:219-221). The `Holds` arm certifies whatever subject the request carries, so a record for package P can be `Certified` from a closure checked over package Q. FR-338 Inputs say the request carries "the checked package recompiled by FR-098's rules". The new test enshrines the gap: closure_certificate.rs:443 passes a minted `[0; 32]` package id that is not the world's package and expects `Certified`. Fix: compare the request's checked package id with `package_id` before checking, and settle a mismatch like the stale-dependency arm. | qsl-replay/src/model_check/settle.rs:159-191, qsl-analyze/tests/it/closure_certificate.rs:443 |
| FND-003 | medium | The admission pre-pass is unbounded and cannot be cancelled. For every certificate state it parses and re-encodes the key once per instance (`instances.iter().any(admit)`, up to `MAX_ATOMS` instances) and clones the key into `members`. It does all this before any budget charge or `Cancel` poll. A hostile certificate with millions of states therefore makes `O(states × instances)` full JCS parses that neither `max_states` nor FR-276 cancellation can stop. FR-338 says the core polls the same `Cancel` handle at each charge and stops under the request's budgets. Fix: poll `cancel` in the pre-pass, and refuse or stop a certificate longer than a request budget (for example `max_states × instances`) before parsing. | qsl-replay/src/certificate/closure.rs:189-204 |
| FND-004 | medium | The checker takes its instance population and its atom truth from the caller's `&dyn ModelLetters`, the same object the engine used. `instances()` builds one context per `letters.bindings()`, and `Check::atom` reads `letters.atom`. A seam that lists only some of the `over` population, or answers an atom wrongly, makes the engine and the checker agree on a subset or a wrong letter, and the record settles `Certified`. FR-338 says the checker re-admits the subject "from its byte provision and universes (FR-125)" and evaluates atoms by "that occurrence's original checked expression and binding". The population at least should be rederived from `request.subject`'s universes. This sits next to the declared FR-337 gap (identity supplied by the request), but it is a separate trust input. Record it, or rederive. | qsl-replay/src/model_check/item.rs:87-128, qsl-replay/src/certificate/closure.rs:377-386, qsl-replay/src/certificate/closure.rs:489-507 |
| FND-005 | low | An unreached member with a rejecting key is accepted. Every admitted key, `["safety",[]]` included, enters `members`, and only reached states are checked. A certificate that lists a bad product state it never reaches is `Accepted`. FR-338's Description accepts "exactly when the certificate's product states ... hold no bad state", while Behavior speaks of a "reached state". The proof is still sound. Either reject a rejecting-key member as `BadState` or `ShapeMismatch`, or align the FR-338 Description with Behavior. | qsl-replay/src/certificate/closure.rs:189-204 |
| FND-006 | low | The deleted TC-522 `Holds` row asserted `truth`, `method`, `limits` and `Category::Success` on the proved record (model_settle.rs, removed rows). The replacement Certified assertions in closure_certificate.rs check only value, label, basis and execution. `Truth::Satisfied`, `EXPLICIT_STATE` and the limits row of a Certified record are no longer asserted anywhere. | qsl-analyze/tests/it/closure_certificate.rs:454-469, qsl-analyze/tests/it/model_settle.rs:619 |

## Dispositions

Round 1, reviewed at 2071dfd1323b0a953b0cdeda5d663a313bc58276. The fix
commits are e55788157 (FND-001 to FND-003) and 2071dfd13 (STD-185's
least-refused-key outcome). The merges of main, b170632e5 and e9a89c6a7, bring
in STD-185 (#999) spec text only on this path. The
mediums and lows left unfixed are recorded, not fixed, under the majors-only
review rule.

The fix round was also checked for regressions:

- Admission now reads at most `max_states × instances` entries. The engine's
  certificate is `nodes × instances` with `nodes ≤ max_states`, so a genuine
  certificate never trips the limit.
- Refused keys are never inserted into `members`.
- `qsl-package` was already a normal dependency of `qsl-replay`.
- The PR body states the FR-337 gap (spec gap 1). `ClauseIdentity` still
  comes from the request, and FR-337-AC-5, TC-1079 and the unresolved-root
  half of FR-338-AC-8 are not done. No test claims them: the TC-1080 test
  covers only the refused-key half.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | e55788157 |
| FND-002 | fixed | e55788157 |
| FND-003 | fixed | e55788157 |
| FND-004 | deferred | Medium, recorded and not fixed (majors-only rule). The instance population and atom truth still come from the caller's `ModelLetters`. Rederiving the `over` population from the subject belongs with the FR-337 S4 clause-node slice the leader is routing. |
| FND-005 | deferred | Low, recorded and not fixed. An unreached member with a rejecting key is still accepted. That is sound, but FR-338's Description and Behavior still disagree. |
| FND-006 | deferred | Low, recorded and not fixed. `Truth::Satisfied`, `method` and `limits` on a Certified record are still not asserted. |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | low | The least-refused order compares `model_state.as_bytes()` and ignores the digest domain. Two refused entries with equal digest bytes and equal key octets but different domains (for example a foreign-domain copy of a state) tie, and the first one read wins. In that corner the locus depends on the order of `states`, which FR-338 forbids. Fix: break the tie on the domain spelling, or on the full `ProductStateRef` `Ord`. | qsl-replay/src/certificate/closure.rs:230-243 |

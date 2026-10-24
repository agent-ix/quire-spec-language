---
id: SR-14850
title: "integrity review of STD-256 compensation attempt record, key retention and footprint (PR #1200, QSpec PR #353)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@6e35052ff7072d7486001ff9721d5e5a0bad0fe9; spec/decisions/ADR-027-protocol-transition-system-for-parallel.md, spec/functional/FR-205-define-the-protocol-subject-and-its-state-key.md, spec/functional/FR-207-run-compensation-templates-as-protocol-threads.md, spec/functional/FR-216-give-every-protocol-step-a-static-footprint.md, spec/test-cases/TC-650-protocol-subject-and-state-key.md, spec/test-cases/TC-652-compensation-templates.md, spec/test-cases/TC-661-protocol-step-footprints.md; agent-ix/quire-specification@21ff693e1113a6d69e501fab5300552a8ead515b; spec/functional/protocol/FR-425-define-protocol-subjects-and-state-keys.md, spec/functional/protocol/FR-426-apply-protocol-steps.md, spec/functional/protocol/FR-427-check-protocol-compensation.md, spec/test-cases/TC-375-protocol-state-key-and-coalescing.md, spec/test-cases/TC-380-compensation-worked-example.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-027
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-205
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-207
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-216
    type: reviews
---
# SR-14850: integrity review of STD-256 (PR #1200, QSpec PR #353)

## Summary

Ticket: STD-256. Diffs reviewed: QSL `git diff origin/main...6e35052ff`
(7 files, one commit) and QSpec `git diff origin/main...21ff693` (5 files,
one commit). Both PRs are based on `main` and are not stacked.

Checks:

- Faithfulness to the ticket. The ticket asks for three rules: what record a
  `cattempt` produces (ST-11, FR-207, QSpec FR-427), when the key keeps it
  (PS-8, FR-205, QSpec FR-425 and FR-427), and its footprint location (FT-1,
  FR-216). The lead's comment asks to restore the retry part of FR-205-AC-8,
  FR-425-AC-12 and TC-375 PK-16. All of these are done.
- STD-233 history. The text takes the lead defaults that SR-13990 proposed:
  FND-002 (enumerate the attempt-type records, filter by the retry relation,
  add the record to the TS-2 identity), FND-003 (keep the record only while a
  later `cattempt` can read it) and FND-004 (the record is part of
  `ctl(t)`, and the commit entry is part of `reg(c, ·)`). The PR bodies label
  them as lead defaults, not owner answers.
- PS-8 closing rule. The retention condition lists the four conditions under
  which ST-11 can still evaluate a retry relation over `earlier`: the thread
  rests at its retry point, the count is below `attempts n`, the relation
  reads `earlier`, and no commit step has occurred. The removal cases (count
  reaches `n`, the commit step, `cend`) are the steps that make one of those
  conditions false. The closing sentence now names attempt-record removal.
  This is consistent with ST-11, CP-3, CP-4 and CP-5. See FND-001 for the
  wording of the condition.
- Code. `Program::new` refuses `UnboundAttemptRecord` when
  `maximum_attempts > 1` and the relation reads local 0 or 1
  (qsl-eval/src/simulation/protocol/program.rs:401-402). `cattempt` at a retry
  evaluates the relation with both locals unbound and adds no record to
  `ProtocolTransition::cattempt` (protocol/compensation.rs:369-390). It does
  not evaluate the relation at the first attempt, which matches the new text.
  The new ACs describe behaviour the code refuses today, and the PR bodies list
  the code-lane remainder with paths that exist. No AC states a result the
  code already produces, so no probe was run.
- Fixtures. The `attempts 2` variants set `refund()`'s precondition to `true`,
  so a retry is enabled after the first refund sets `bal` to 0. §7.1 and
  TC-380 now say `RefundAttempt` has one value. So CP-01's nine states and
  eight transitions, CP-05's distances and FR-207-AC-1 to AC-3 still hold
  under per-record enumeration. Hand count of FR-207-AC-6 and FR-427-AC-7:
  at t4 there are two `cattempt` steps, one per `ok` value, with one post-state
  model. Under `not e.ok`, the retry point after `ok` false holds the record,
  and the relation is true for both `l`, which gives two retries plus `cend`.
  After `ok` true the relation is false, which leaves only `cend`. Under
  `l.ok`, no record is kept, the two first attempts meet in one state, and one
  retry (`l.ok` true) plus `cend` is enabled. This matches each AC.
- QSL and QSpec agree. PS-8 and the FR-425 table entry state the same
  retention condition. ST-11 and the FR-427 `cattempt` row state the same
  enumeration and the same first-attempt rule. TS-2 and the FR-426 `record`
  row agree. FR-205-AC-8 and FR-425-AC-12 agree word for word in the variant.
  FR-207-AC-6 and FR-427-AC-7 also agree. FR-466 names the retry binders
  `earlier` and `later`, as the new text does.
- Ids. FR-207-AC-6, FR-216-AC-7, FR-427-AC-7 and TC-380 CP-13 are new and
  unique. No other open PR in either repo touches these files.
- Stale text. A grep of `spec/` and `proposals/` in both repos, outside
  `reviews/`, finds no remaining "previous attempt's binder" and no statement
  that contradicts the new rule.
- Wording. The text describes behaviour only. The PR bodies do not paraphrase
  review findings.
- Gates. QSL `make check-spec-validation` exits 0, with no diagnostic on a
  touched file. `tools/check-index-completeness.sh` is not on main; STD-226
  (#1162, commit 0f34179e2) removed it. QSpec `quire validate --scope .
  spec/**/*.md` reports 134 failures, which is the baseline, and none of them
  is in a touched file. `cargo test --test checked_package_v2 --test
  backend_provider_v1` passes, with 14 and 117 tests. Tools: quire 0.36.2
  (engine 0.50.2) and quoin 0.28.3. These are newer than the 0.36.1/0.50.1
  and 0.28.1 releases that the rollout note names.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | PS-8, FR-205 and QSpec FR-425/FR-427 say the key keeps the record "exactly while a later `cattempt` of the thread can read it". The colon list that follows is syntactic, and FR-205-AC-8 and FR-425-AC-12 require the record to be kept after `ok` true under `not e.ok`, a state where no later `cattempt` can occur. An implementer who reads "can read it" semantically would drop the record there and fail those ACs. Fix: state the four conditions as the definition ("while all of these hold: ..."), without "exactly while ... can read it". | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:140; spec/functional/FR-205-define-the-protocol-subject-and-its-state-key.md:101; quire-specification spec/functional/protocol/FR-425-define-protocol-subjects-and-state-keys.md:201 |
| FND-002 | low | FT-1 and FR-216 say the commit-node step writes "the `ctl` of each compensation thread, in its instance, of a template that names the node, whose attempt record the step removes". The clause can be read as restrictive, which writes only threads that hold a record, or as explanatory, which writes every such thread. Under `attempts 1` or a retry relation that does not read `earlier`, no thread holds a record, so the two readings give different static footprints. FR-216 ACs pin footprints exactly ("and nothing else"). Fix: say which threads are written. | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:294; spec/functional/FR-216-give-every-protocol-step-a-static-footprint.md:99-101 |
| FND-003 | low | FR-216-AC-7 says the commit step writes the threads' `ctl`, "so it is dependent with that thread's `cend` and its `cattempt`". Without that write, the commit step already writes `reg(Refund, ·)`, which `cattempt` reads and which contains the `reg(Refund, r)` that `cend` reads and writes. So the dependence check passes whether or not the new write exists, and it does not test the new location. The footprint listing in the same AC does test it. Fix: drop the "so", or test a pair whose dependence comes only from `ctl`. | spec/functional/FR-216-give-every-protocol-step-a-static-footprint.md:155 |
| FND-004 | low | FR-216-AC-7 and TC-661 step 7 use "a `cattempt` whose attempt type holds a `Reference<Peer>`", but no fixture declares a `Peer` type or population or an attempt type with that field. FR-216 uses `Peer` nowhere else. The case can be built, but two test authors would build different models. Fix: name the model, for example an attempt type with one field `p: Reference<Peer>` over a `Peer` population in the FR-205-AC-8 variant. | spec/functional/FR-216-give-every-protocol-step-a-static-footprint.md:155; spec/test-cases/TC-661-protocol-step-footprints.md:44-47 |

## Verdict

Mergeable. There are four low findings and no major ones. Both halves are
faithful to the ticket and to the STD-233 lead defaults, they agree with each
other, they are consistent with the PS-8 closing rule, and every new AC can be
built. The minors can be fixed in this PR or deferred.

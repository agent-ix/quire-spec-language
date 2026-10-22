---
id: SR-13990
title: "integrity review of STD-233 protocol key members, ST-2, FR-221-AC-1 and FR-216-AC-4 (PR #1189, QSpec PR #329)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@e2239ccee1cd807c6f824fb928d90717f6798981; spec/decisions/ADR-025-weak-memory-models-for-parallel.md, spec/decisions/ADR-027-protocol-transition-system-for-parallel.md, spec/functional/FR-205-define-the-protocol-subject-and-its-state-key.md, spec/functional/FR-206-take-the-protocol-step-kinds-with-folded-structural-moves.md, spec/functional/FR-216-give-every-protocol-step-a-static-footprint.md, spec/functional/FR-221-explore-the-x86-tso-memory-model.md, spec/test-cases/TC-650-protocol-subject-and-state-key.md, spec/test-cases/TC-651-protocol-step-kinds.md, spec/test-cases/TC-666-x86-tso-memory-model.md; agent-ix/quire-specification@d2b6b37bea68292b7b74ee5208b4916b3598166f; spec/functional/protocol/FR-425-define-protocol-subjects-and-state-keys.md, spec/functional/protocol/FR-426-apply-protocol-steps.md, spec/test-cases/TC-375-protocol-state-key-and-coalescing.md, spec/test-cases/TC-376-two-branch-join-worked-example.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-027
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-025
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-205
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-206
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-216
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-221
    type: reviews
---
# SR-13990: integrity review of STD-233 (PR #1189, QSpec PR #329)

## Summary

Ticket: STD-233 (partial). Diffs reviewed: QSL `git diff origin/main...e2239cc`
(9 files, +32/-12) and QSpec `git diff origin/main...d2b6b37` (4 files,
+8/-3). Both PRs are based on `main` and are not stacked. The brief named the
QSL head `3dd16ec`, but that object does not exist in the clone. The live PR
head is `e2239cc`, and that is what this review covers.

Checks:

- Faithfulness to the ticket. Items 1, 3, 6a and 7 are edited. Items 4 and 5
  are already on main, which I confirmed: FT-1, FT-3 and FT-5 are in ADR-027
  line 294, and TC-661's Scope reads AC-1 to AC-5. Items 2 and 6b are left to
  the owner. The lead defaults I propose for them are below.
- Implemented behaviour. The `commit` control entry, with key
  `{"type":"commit","instance","node"}` and value
  `{"type":"commit-state","occurred":true}`, and the `registration` member of
  a compensation thread's `thread-state` match `key_members`
  (qsl-eval/src/simulation/protocol.rs:1059-1215). They also match the
  untagged test `a_compensation_thread_keys_its_registration`
  (qsl-analyze/tests/it/protocol_compensation.rs:990), which reads
  `value["registration"]["type"] == "registration"`. The two shapes are
  clean. The `attempt-record` entry has no code counterpart yet, because the
  code refuses `UnboundAttemptRecord` (program.rs:402). See FND-002 and
  FND-003.
- QSL and QSpec agree. The commit entry's lifetime is "absent once the
  instance finishes" in QSL and "until the instance is removed" in QSpec.
  These are the same thing under ADR-027 AE-3 and ST-9, which remove an
  instance in the step that finishes it. ST-2 and the FR-426 `event` row say
  the same thing.
- Ids. FR-205-AC-8, FR-206-AC-10, FR-425-AC-12, FR-426-AC-16, PK-16 and FL-10
  collide with no other open PR in either repo. QSpec PR #328 holds
  FR-426-AC-15. QSL PR #1196 (STD-248) edits FR-221-AC-1 and ADR-025 §10. See
  FND-001.
- Wording. No changelog, status or review language. The PR bodies do not
  paraphrase review findings.
- Gates. `make check-spec-validation` exits 0, with no warning on a touched
  file. `tools/check-index-completeness.sh` exits 0. The PR touches no
  `docs/` file and no vector table. QSpec `quire validate --scope .
  'spec/**/*.md'` reports 134 failures, the same as main, none of them in a
  touched file. `cargo test --test checked_package_v2 --test
  backend_provider_v1` passes, 14 and 117 tests. Tools: quire 0.36.2
  (engine 0.50.2) and quoin 0.28.3. These are newer than the 0.36.1/0.50.1
  and 0.28.1 releases the rollout note names.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Open PR #1196 (STD-248) rewrites the same ADR-025 §10 `tso` paragraph and FR-221-AC-1 the other way: it pins the canonical order `Sx`, `Sy`, `Lx`, `Ly`, which the code reports (STD-248, release_acquire.rs:217), and also fixes the `ra` table and FR-222-AC-1. #1189 instead loosens FR-221-AC-1 to order-independent facts. The two PRs conflict in text and in decision, and only one can merge. Drop item 6a from #1189 (the ADR-025, FR-221-AC-1 and TC-666 hunks) and let STD-248 own it. | spec/functional/FR-221-explore-the-x86-tso-memory-model.md:63; spec/decisions/ADR-025-weak-memory-models-for-parallel.md:413 |
| FND-002 | high | The new `attempt-record` key entry holds "the record of its previous attempt", but no rule says what attempt-type record a `cattempt` produces. ST-11 and FR-427 say only "with the template's attempt type as the record". ST-1's record is (receiver, arguments, result), and TS-2's `cattempt` identity carries no record. The code refuses `UnboundAttemptRecord` for exactly this reason: "no operation result binds here" (protocol.rs:110-112). So the retry clause of FR-205-AC-8 ("a retry relation over it evaluates"), FR-425-AC-12 and PK-16 cannot be built. Lead default: a `cattempt` takes one step per record of the attempt type `T` that satisfies the retry relation, enumerating them as ST-2 enumerates event records. Add that record to the TS-2 `cattempt` identity, and the record is what the entry keeps. | spec/functional/FR-205-define-the-protocol-subject-and-its-state-key.md:121; spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:172 |
| FND-003 | medium | PS-8 and FR-425 keep the `attempt-record` entry for every compensation thread from its first `cattempt` to its `cend`. That includes cases where no later step can read it: `attempts 1`, a retry relation that does not read `earlier`, a count that has reached `n`, or a commit step that has occurred (CP-4). The entry then splits states that no behaviour tells apart, which contradicts PS-8's closing rule that the key is a function of what the protocol can still do. The code conditions the `activation` observation the same way (`trigger_read`, protocol.rs:1165), and it refuses only when `maximum_attempts > 1` and the relation reads the records (program.rs:401). Hold the entry only while a later `cattempt` can read it. | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:140; spec/functional/protocol/FR-425-define-protocol-subjects-and-state-keys.md:201 |
| FND-004 | medium | FT-1 and FR-216 give the new key members no footprint location. `cattempt`'s `R` is "`ctl` of its thread, its count and the previous attempt's binder", but the record is now an `observations` `attempt-record` entry, not a binder. `R` also leaves out the commit occurrence that ST-11 reads, and FT-5's `ProtocolLocation` list has no location for it. The code treats the commit occurrence as part of `reg(c, ·)`, which a `cattempt` reads (footprint.rs:46-49, 830-836), and FR-216's enabling-footprint text already says "`reg(c, ·)` and the commit's occurrence for a `cattempt`". As written, a commit step and a `cattempt` read as independent although the commit step disables the `cattempt`. State that `reg(c, ·)` holds the commit entry of each template that names that node, add `reg(c, ·)` to `cattempt`'s `R`, and make the attempt record part of `ctl(t)`. | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:294; spec/functional/FR-216-give-every-protocol-step-a-static-footprint.md:97-98 |
| FND-005 | medium | FR-206-AC-10, FR-426-AC-16 and FL-10 write an unqualified `effect E of A` after the join, with `A` declared inside branch `left`. Under FR-113, resolution starts at the effect's own scope and moves outward. No enclosing scope declares `A`, because the branch declares it, so S3 refuses `missing_declaration`/`missing-name` before the step can be observed. The code's own test for this behaviour uses the qualified anchor `Main::P::a::C` (protocol_compensation.rs:1013-1017). Write the anchor qualified, for example `effect E of Main::P::left::A`. | spec/functional/FR-206-take-the-protocol-step-kinds-with-folded-structural-moves.md:103; spec/functional/protocol/FR-426-apply-protocol-steps.md:223 |
| FND-006 | medium | The FR-216-AC-4 oracle can be read more than one way. "Each location `l` of the instantiated universe" could mean model locations only, or protocol locations (`ctl`, `inst`, `reg`, `queue`) as well. "Any value the location's type admits" makes ill-formed pre-states for protocol locations, for example a `ctl` rest point that no settling reaches. "The corresponding successor" is not defined for a step with several successors of one identity, which TS-2 tells apart only by post-state digest. Lead default: `l` ranges over the model locations (object, field) of the subject's universe and over population membership, with values from the field's declared domain. Successors correspond by transition identity and by the delta outside `l`. | spec/functional/FR-216-give-every-protocol-step-a-static-footprint.md:135 |
| FND-007 | medium | The fixture for FR-205-AC-8 and FR-425-AC-12 in TC-650 step 8 and PK-16 contradicts CP-4. The fixture steps the commit node `Committed` and then takes "a `cattempt` and a retry". After a commit step no `cattempt` is enabled, and no new registration is made (CP-1). Neither AC names the protocol that lets `r1` and `r2` activate in either order (two triggers in sibling branches, CP-2). "A retry relation over it evaluates" states no expected result. Order the fixture as activate in both orders, then `cattempt`, retry and `cend`, with the commit step on a separate path. Name the sibling-trigger protocol, and state the retry's expected enabledness. | spec/test-cases/TC-375-protocol-state-key-and-coalescing.md:45; spec/test-cases/TC-650-protocol-subject-and-state-key.md:42-44 |
| FND-008 | low | Two phrases explain rather than state behaviour. FR-206-AC-10's clause "the condition that `A`'s binder is bound is decided from that environment at the step" is not observable; the enabledness and environment facts before it already assert it. ST-2 and FR-426's "not a rule on where the node stands" is a negative aside. Drop the FR-206-AC-10 clause, and keep only the positive condition in ST-2 and FR-426. | spec/functional/FR-206-take-the-protocol-step-kinds-with-folded-structural-moves.md:103; spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:163 |
| FND-009 | low | After #1189's edit, ADR-025 §10 still says "Under `sc` the same four steps give `c2.r = 1` at position 4" and opens the `ra` paragraph with "The same order". Both refer to an order the paragraph no longer fixes. FR-222-AC-1 and FR-227-AC-1 still pin §10's tables position by position. This becomes moot if FND-001 is resolved by dropping item 6a. | spec/decisions/ADR-025-weak-memory-models-for-parallel.md:428-433 |

## Verdict

Not mergeable as is: two high and five medium findings. The commit and
registration key shapes are clean and match the code. ST-2's run-time wording
is faithful to the code's ST-2 reading (footprint.rs:552). The QSL and QSpec
halves agree with each other.

Proposed lead defaults for the two items the author stopped on (proposals
only, not owner answers; nothing edited):

- Item 2 (ST-12 recovery view). Default: refuse. Over a protocol subject, a
  recovery predicate that reads its recovery binder settles
  `unsupported`/`RecoveryView`. This is the code's current refusal
  (program.rs:405, protocol.rs:113-115). The predicate may read only the
  captures and the thread's observation. ADR-027 §7.1 (`recover (r:
  M::RecoveryView) { r.bal = 0 }`), FR-207-AC-2, TC-652 and QSpec TC-380 are
  rewritten to a predicate over captures, and they assert the refusal for a
  binder read. Evidence:
  - QSpec FR-464 bars model state from the recovery slot.
  - FR-381 bars deriving the predicate from the attempt ("Operation success
    does not stand for recovery").
  - The choreography surface (proposals/quire-v1/choreography-surface.md:172)
    calls the view "supplied", and a model-check run has no supplier.
  Deriving the view from model objects needs a declared mapping, which is new
  surface and a choice for the owner. Refusing leaves no AC that cannot be
  built, once FR-207-AC-2 is rewritten.
- Item 6b (MA-1 readers). Default: over-approximate, and refuse rather than
  classify.
  - An `event`, `effect`, `commit`, `send`, `receive` or `check` whose
    constraint reads a model location inside a branch of a weak `parallel`
    counts as a reader of that location for MA-1. Sharing can then only grow.
  - When such a read hits a shared location, EN-1's pre-check settles V-8
    `unsupported`, cause `WeakAccessShape` naming the node, as MA-4 does for
    access shapes it cannot classify. MA-2 still classifies only attempts.
  - `finish`, and nodes outside every weak `parallel`, read the joined
    memory. Every joined state has no buffer (FR-221-AC-4) and one message
    per location (FR-222-AC-3), so they are not readers.
  - Separately from the memory model, FT-1's `R` for `event` and `finish`
    gains the model locations its constraint reads. This uses the
    field-over-universe over-approximation that PR #1187 adds for `retire`
    predicates. Today FT-1 and the code (footprint.rs:550-590) list only
    binders and membership for these steps, while QSpec FR-464 lets them
    read `over`.

---
id: SR-503
title: "Evidence analysis of ADR-010 to ADR-013 (ARCH-G1)"
type: SpecReview
analysis: evidence
scope: "spec/decisions/ADR-010-observed-architecture-baseline.md, spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md, spec/decisions/ADR-012-semantic-family-extension-contracts.md, spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
---
# SR-503: Evidence analysis of ADR-010 to ADR-013 (ARCH-G1)

## Summary

This review reads the four records together, as one subject, at
`task/212-arch-g1-gate` HEAD 457a131 plus the uncommitted ADR-011 edits of
this PR. Those edits include the #245 §2.3 rule 8 amendment on proof-gate
vacuity. Line anchors follow SR-498: `ADR-011:134` is line 134 of the ADR-011
file in this worktree.

The records are design records, not FRs. They have no acceptance-criterion
rows, and `quoin advise --json` returns no obligation for any of them. Every
method below is reviewer judgement, not a catalog verdict. The review asks one
question of each named piece of evidence: can it fail? It also asks whether
the four records name the same evidence for the same rule.

This review does not repeat SR-498. MD-1 (where the crossing replay test runs)
and MD-2 (a registered backend with no CG kind) stay as SR-498 states them.
FND-001 to FND-007 of SR-498 are not repeated.

What holds:

- ADR-010 is falsifiable throughout. Every positive cell is a
  `<prefix>:<path>:<line>` at a pinned sha, and every absence is a stated
  `git grep` pattern (ADR-010:60-85). Spot checks of the ADR-011 §6.1 kernel
  anchors hold in this worktree: `value/composite.rs:22-31,50-67,144-158` and
  `value/outcome.rs:10,13`.
- ADR-011 §4 and §5 name `compile_fail` tests and
  `clippy::wildcard_enum_match_arm` (ADR-011:433-435, 491-492). Both can fail.
- ADR-012 §5.3 seam probe compares rustc E0004 locations with a checked-in
  list, "no more and no fewer" (ADR-012:432-437). It fails in both directions.
- ADR-012 §7.4 names a fault-injection test with a missing tool and a
  mismatched pin (ADR-012:593-597).
- ADR-013 §4 names a test for each of C-01 to C-29 (ADR-013:675-705). C-13
  names adverse executor tests that separate a `package_id` refusal from a
  source-digest refusal (ADR-013:689).
- The #245 evidence is a measured mutation, not an assertion: the agent-ix/quire-contract-runtime#57
  harness stayed VERIFICATION SUCCESSFUL under a mutated `decode`, and the
  fixed harness failed (#245 body; ADR-011:375-379).

What does not hold:

- Rule 8 now forbids an oracle that shares a helper with the code under proof.
  ADR-013 O-13 makes CG oracles consumers of the kernel operations that the
  scenario 1 proof is about (FND-001).
- Rule 8 requires a mutation of "each shared helper", but no record says who
  lists the shared helpers or how the list is checked. The gate record says
  "the gate lists every helper" and cites rule 8 for it (FND-002).
- Several named checks have no pass criterion, no recorded artifact, or no
  owner that can see the violation (FND-004, FND-008, FND-010 to FND-012).

Verdict: **BLOCK.** FND-001 and FND-002 make SR-498 scenario 1 wrong as
written: its evidence cell (gate.md:73) states two things that the records do
not hold. Both are editorially fixable in this PR with the text in FND-001 and
FND-002. With that text applied, the verdict is ACCEPT WITH FINDINGS, and no
other finding is blocking.

## Findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-001 | high | **Blocking.** Rule 8 and ADR-013 O-13 disagree on the scenario 1 oracle. Rule 8 says the proof's expectation is derived independently of the function under proof (ADR-011:135-136, 375-376). ADR-013 O-13 lists "CG oracles" as consumers of the kernel `Value` "and their operations" (ADR-013:294, 296). ADR-011 §7.1 draws CG → `quire-exact` (ADR-011:676). After X-1 the RT exact kernel is replaced by `quire-exact` (ADR-011:730). So a CG oracle that computes its expectation with the kernel operation proves that operation against itself. That is the vacuous case of #245, for the whole function and not one helper. SR-498 scenario 1 says "The CG oracle's expectation is derived independently of the `quire-exact` operation under proof" (gate.md:73). ADR-013 as written does not allow that claim. **Fix (editorial, ADR-013:294):** replace "Consumers: QSL evaluator, RT host ABI, CG oracles." with "Consumers: QSL evaluator, RT host ABI, and CG oracles for kernel types only. A CG oracle never computes its expectation with the kernel operation under proof, or with a kernel helper that operation calls (ADR-011 §2.3 rule 8). The expectation for a kernel operation is derived from the QSpec operation-catalog law or vectors." | ADR-011:135-136, 375-379, 676, 730; ADR-013:294, 296, 299; gate.md:73 |
| FND-002 | high | **Blocking.** The shared-helper mutation has no declared input. Rule 8 and §2.3 item 3 require a run mutation of "each helper that the oracle or expectation shares with the code under proof" (ADR-011:138-139, 390-391). No record defines "shared" (direct or transitive, same crate or any crate), and no record requires the helper set to be published or checked. A gate that omits a helper runs no mutation for it and passes item 3. That is the #245 vacuity again, one level up. §2.3 already requires a checked-in claimed-module list that #219 checks (ADR-011:381, 394-395); the helper set has no such rule. SR-498 scenario 1 says "The gate lists every helper the oracle shares with the operation under proof" and cites rule 8 for it (gate.md:73). ADR-011 holds no such duty. **Fix (editorial, ADR-011 §2.3, after item 3 at :391):** "A helper is shared when the oracle or expectation and the function under proof both call it, directly or transitively, in any crate. Each counted gate publishes, beside its claimed-module list, a checked-in list of its shared helpers. #219 checks that list against the oracle and the code under proof, and a shared helper missing from the list fails the gate." | ADR-011:138-139, 381, 390-395; gate.md:73; #245 |
| FND-003 | low | The discharge floor has two readings. "Counts SUCCESS checks only; UNREACHABLE checks are subtracted" (ADR-011:139-141, 385-387) can mean the count of SUCCESS checks, or SUCCESS minus UNREACHABLE. The #245 table shows why it matters: the mutated run lost two checks (2491 to 2489). A #219 census written to one reading can pass a module the other fails. **Fix:** "The discharged count of a module is the number of its checks whose status is SUCCESS. A check with any other status, including UNREACHABLE, counts zero." | ADR-011:139-141, 385-387; #245 |
| FND-004 | medium | Mutation evidence is run but never recorded. Item 2 (a mutation control turns the gate red) and item 3 (each shared-helper mutation fails the proof) name no artifact that records the red run (ADR-011:388-391). #219 checks the claimed-module list against the transcript census (ADR-011:394-395), but it has no transcript of the mutated runs to check. A gate can claim both items with nothing to inspect. Also, a claimed-module list that shrinks between runs is only "reported by #219" (ADR-011:396). Dropping a vacuous module therefore keeps the gate green. **Fix:** state that each mutated run's transcript is kept with the gate's census, and that #219 fails the gate when the list shrinks without a change that removes the module. | ADR-011:388-398; FB-10 (ADR-011:423) |
| FND-005 | medium | The rule 8 amendment is not carried into the ticket that builds the CG gate. T-10 still reads "claimed-module list, `unreached` failure, mutation control" (ADR-011:926). agent-ix/quire-contract-codegen#88, which SR-498 cites as T-10 (gate.md:74), has the same three items and neither SUCCESS-only counting nor shared-helper mutation. The CG oracle is where the shared helpers of FND-001 and FND-002 arise. §9 agent-ix/quire-contract-runtime#53 and Consequences were updated (ADR-011:800, 942-946). **Fix (ADR-011:926):** "CG generated-harness gate: claimed-module list, `unreached` failure, SUCCESS-only discharge count, mutation control, shared-helper list, and a run mutation of each shared helper (§2.3)". Amending the agent-ix/quire-contract-codegen#88 body is an owner action. | ADR-011:926; agent-ix/quire-contract-codegen#88; gate.md:74 |
| FND-006 | low | The restatements of rule 8 differ in scope. Rule 8 and §2.3 say independent "of the function under proof" (ADR-011:135-136, 375-376), and so does the §9 agent-ix/quire-contract-runtime#53 row (ADR-011:800). Consequences says agent-ix/quire-contract-runtime#53 needs "a harness whose expectation is independent of `src/exact/`" (ADR-011:945-946), which is the whole module. Under rule 8, an RT harness may call `src/exact/` helpers when each one's mutation fails the proof. Under Consequences it may not. **Fix (ADR-011:945-946):** "a harness whose expectation is derived independently of the function under proof in `src/exact/`". | ADR-011:135-136, 375-376, 800, 945-946 |
| FND-007 | low | The counting rule uses Kani's vocabulary only: SUCCESS, UNREACHABLE and VERIFICATION SUCCESSFUL (ADR-011:139-141, 379, 385-387). ADR-012 admits other backends: a temporal backend for #188 and the quire-analyze implication backend (ADR-012:544-546), each with its own outcome enum at seam S8 (ADR-012:393). The rule states no discharged status for them, so their proof evidence cannot meet §2.3 as written. **Fix:** say that each backend's S8 outcome map names which check status counts as discharged, or confine §2.3 to Kani until T-11 lands. | ADR-011:379-387, 927; ADR-012:393, 544-546 |
| FND-008 | medium | Two kernel minting rules have no check that can see a violation outside QSL. ADR-013 O-04 says RT and CG hold no `NodeKey` (ADR-013:170). `NodeKey` and `EffectiveId` live in `quire-exact`, which RT and CG depend on (ADR-011:675-676). T-12 part (b) and part (c) name only the QSL callers: "only `check`" and "only `model`" (ADR-011:592-595, 928). The FB-05 direction check covers QSL types, not kernel types (ADR-011:418). A call to the kernel `NodeKey` constructor from RT or CG therefore passes every named check. **Fix:** state that the T-12 API-surface check scans every crate that depends on `quire-exact`, and fails any `NodeKey` or `EffectiveId` constructor call outside QSL `check` and `model`. | ADR-011:418, 592-595, 675-676, 928; ADR-013:170, 182 |
| FND-009 | medium | The R-06 static check contradicts R-06's own exception. ADR-013 names a #226 static check "that no post-check module calls a name-resolution function" (ADR-013:97-99). R-06 allows one post-check lookup: the replay executor resolves a `QualifiedName` against the recompiled package (ADR-013:89). ADR-011 places that lookup in the layer-6 `replay` facade (ADR-011:272-274, 584-589). The check as written fails on the first TK-01 landing, or it is written with an exemption no record names. **Fix (ADR-013:98-99):** "…that no post-check module calls a name-resolution function, except the `replay` facade's `QualifiedName` lookup (R-06)". | ADR-013:89, 97-99; ADR-011:272-274, 584-589 |
| FND-010 | medium | The witness admission tests have a stale owner. O-25 says IR PR #139, "before its sha is recorded", routes `Deserialize` through `parse`, makes `transcript` private and adds three refusal tests (ADR-013:579). #139 merged at 954c2f2 without them: at IR 954c2f2 and at IR `origin/main` 65aa282, `Witness` derives `Deserialize` and `transcript` is `pub` (`IR:src/kani/witness.rs:99,107`). §7 still makes #231 wait on "IR PR #139 merged at a recorded sha with admission through `parse`" (ADR-013:779). That condition can never be met. TK-04 lists packet members, the WP9 map and reader codes, and not admission (ADR-013:891). The admission ruling now lives in agent-ix/quire-contract-ir#144. **Fix:** in O-25 (ADR-013:579), §7 (:779) and TK-04 (:891), name agent-ix/quire-contract-ir#144 as the owner of admission through `parse` and of the three refusal tests. | ADR-013:579, 779, 891; IR 954c2f2 and 65aa282 `src/kani/witness.rs:99,107`; agent-ix/quire-contract-ir#144 |
| FND-011 | medium | Mapping-mutant evidence has no pass rule, and the records disagree on its scope. ADR-012 §5.3 says AD-016's `cargo mutants` requirement covers S1, S4 and S5 to S9 (ADR-012:445-446). S8 is the `KaniOutcomeKind` → FR-331 map, which is ADR-013 C-09. ADR-013 names `cargo mutants` for C-05 only (ADR-013:681), "mutation tests on each mapping" for O-10 (ADR-013:249), and an enumeration test with no mutants for C-06, C-09, C-15 and C-20 (ADR-013:682, 685, 691, 696). No record says whether a missed mutant fails the gate. A mutation run that reports missed mutants and passes is not evidence. **Fix:** state once that a missed mutant in a listed mapping fails the gate, and give ADR-013 C-06, C-09, C-15 and C-20 the same `cargo mutants` evidence as ADR-012 §5.3. | ADR-012:445-446; ADR-013:249, 681-696 |
| FND-012 | medium | The end-to-end disposition test has no home that can produce counted evidence. ADR-012 puts it in "the test harness downstream of CG" (ADR-012:454-457, 920-921). ADR-011 puts cross-repository end-to-end evidence in CG or in the QI `heads/` workspace (ADR-011:698-701). ADR-013 says the heads lane "never produces release evidence" (ADR-013:540-543). CG is not downstream of itself, and the driver that is downstream of CG has no placement yet (#225, ADR-011:525). This has the same shape as MD-1, but for the #185 exit corpus, not for replay. **Fix:** name the crate that runs the #185 exit corpus, or state that it runs in CG with a test-only registry value built from provider manifests. | ADR-012:454-457, 920-921; ADR-011:525, 698-701; ADR-013:540-543; SR-498 MD-1 |
| FND-013 | low | ADR-013 R-09 has no check until Layer 5. The rule that no new consumer uses a lane-private type is enforced by the #226 drift gate (ADR-013:92). #226 lands in Layer 5. ADR-011 §3 gives an interim inspection rule for its own table only (ADR-011:409-410). Between acceptance and #226, a new consumer of a §6 type passes every named check. **Fix:** add R-09 to the interim inspection walk that #216 and #219 already do. | ADR-013:92, 743-766; ADR-011:409-410 |
| FND-014 | low | Two named checks have no party that runs them. ADR-012 says the landing PRs of #187 and #218 "are checked to change only paths inside the modules their table names" (ADR-012:719-721). SR-498 cites that check as evidence for scenario 2 (gate.md:89). The `xtask string-edge` allow-list can grow with no review rule (ADR-012:655-657), so an entry can hide a string dispatch. **Fix:** name the owner of the landing-PR path check (for example the #219 or #224 gate walk), and state that an allow-list entry names the user value it compares and selects no behaviour. | ADR-012:655-657, 719-721; gate.md:89 |

## Evidence by record

This table is reviewer judgement. The catalog matched no rule, because none of
the four records has criterion rows.

| Record | Evidence that can fail | Evidence that cannot fail as written | Findings |
|---|---|---|---|
| ADR-010 | Pinned positive anchors and `absent:` `git grep` patterns | none found | none |
| ADR-011 | §4 `compile_fail`; §5 lint and #230; §2.3 claimed-module census with the `unreached` failure; the #245 mutation | §2.3 item 3 with no helper list; mutated runs with no record; list shrinkage only reported; T-12 scope | FND-001 to FND-008 |
| ADR-012 | Seam probe; registry permutation property test and static lints; §7.4 fault injection | mapping mutants with no pass rule; e2e disposition harness with no home; landing-PR path check with no runner | FND-007, FND-011, FND-012, FND-014 |
| ADR-013 | C-01 to C-29 tests; C-13 adverse executor tests; `compile_fail` for R-10 | R-06 static check that contradicts R-06; O-13 consumer rule against rule 8; O-25 admission tests with a merged owner | FND-001, FND-008 to FND-011, FND-013 |

## Method

- **Subject.** The four ADRs at `task/212-arch-g1-gate` HEAD 457a131, with the
  uncommitted ADR-011 diff (`git diff`) that carries #245 and the SR-498
  editorial fix. SR-498 (`gate.md`) and SR-470 (`spec/reviews/stage-dag/evidence.md`)
  were read for scope and style.
- **`quoin advise --json`.** Run in the worktree. It returned 461 obligations,
  none from ADR-010 to ADR-013.
- **QSL `src/` at HEAD.** `value/composite.rs` imports and variants
  (`:18-31,36-67,132-158`) and `value/outcome.rs:8-13`, to check the ADR-011
  §6.1 kernel anchors.
- **IR.** `src/kani/witness.rs` at 954c2f2 (IR PR #139) and at `origin/main`
  65aa282.
- **GitHub, read-only `gh`.** QSL #245; agent-ix/quire-contract-ir#144; agent-ix/quire-contract-codegen#88.

## Round 2

- FND-002 is resolved: ADR-011 §2.3 defines a shared helper and requires a
  checked-in shared-helper list beside the claimed-module list.
- FND-001 is resolved in SR-498: scenario 1 states rule 8 as the requirement
  on the oracle and no longer claims the records already guarantee it. The
  proposed O-13 text is SR-498 FND-008, Remaining work: #244.
- FND-003, FND-005 and FND-006 are resolved by the counting-sentence, T-10
  and Consequences edits.

Verdict after Round 2: ACCEPT WITH FINDINGS.

## Round 3

Dispositions after the #212 rulings, 2026-09-19 (issue #212, newest two comments).

- FND-001 is resolved (#212 rulings, 2026-09-19, SR-500 FND-001): ADR-013 O-13 and ADR-011 §2.3
  take the expectation for a kernel operation from the QSpec vectors or a
  specification model that calls no `quire-exact` operation.
- FND-002 is resolved (#212 rulings, 2026-09-19, FND-006): the shared-helper set is computed from
  the build, checked in, and compared by #219.
- FND-004, FND-007 and FND-011 are Remaining work: #219.
- FND-008 is Remaining work: #215.
- FND-009 is Remaining work: #247.
- FND-010 is Remaining work: agent-ix/quire-contract-ir#144 (ADR-013 TK-04).
- FND-012 is Remaining work: #225. The replay crossing test has its home in
  QSL (#212 rulings, 2026-09-19, MD-1).
- FND-013 is Remaining work: #216.
- FND-014 is Remaining work: #226.
- FND-003, FND-005 and FND-006 were resolved in Round 2.

Verdict after Round 3: ACCEPT WITH FINDINGS.

## Round 4

Dispositions after the #212 round-2 rulings, 2026-09-19 (issue #212, round-2
comment), and the #247 editorial alignments.

- FND-003 is resolved (#212 round-2 ruling): a check is discharged only when
  the backend reports it proved or SUCCESS. UNREACHABLE checks, and every
  status other than SUCCESS, are not counted as discharged.
- FND-004 is resolved (#212 round-2 ruling): every run mutation is checked in
  with the diff, the exact command and the failing check output. A mutant
  that does not fail the proof fails the gate, with no allow-list (ADR-011
  §2.3). #219 enforces it.
- FND-005 is resolved: agent-ix/quire-contract-codegen#88's scope is amended
  with the rule-8 items, and SR-498 scenario 1 cites it.
- FND-007 is resolved (#212 round-2 ruling): a backend with no per-check
  status counts one check per proved obligation.
- FND-009 is resolved (#247): the R-06 static check names the E9
  `QualifiedName` lookup as its one exception (ADR-013 §2).
- FND-012 is resolved (#212 round-2 ruling): the end-to-end disposition test
  runs in CG over wire and never calls `route` (ADR-012 §5.3).
- FND-013 is resolved (editorial): ADR-011 §3 and ADR-013 R-09 put R-09 in
  the #216 and #219 interim inspection walk until #226 lands.
- FND-008 is Remaining work: #215. FND-014 is Remaining work: #226. Each
  needs an owner ruling, which #212 reports.
- FND-010 is Remaining work: agent-ix/quire-contract-ir#144. FND-011 is
  Remaining work: #219.

Verdict after Round 4: ACCEPT WITH FINDINGS.

## Round 5

Dispositions after the #212 round-3 rulings, 2026-09-19 (issue #212, round-3
comment).

- FND-008 is resolved (#212 round-3 ruling): the ADR-011 T-12 API-surface
  check scans every crate that depends on `quire-exact`, and a `NodeKey` or
  `EffectiveId` constructor call outside QSL `check` and `model` fails it.
  #215 ships it as one reusable tool. RT runs it in its lint gate under
  agent-ix/quire-contract-runtime#56, and CG under
  agent-ix/quire-contract-codegen#89 (ADR-011 T-12, ADR-013 O-04, O-05).
- FND-014 is resolved (#212 round-3 ruling): the #219 and #224 gate walks
  check that the #187 and #218 landing PRs change only the paths their table
  names until #226 lands, and #226 enforces it after that. Each
  `xtask string-edge` allow-list entry names the user value it compares and
  selects no behaviour (ADR-012 §9, §12).
- FND-010 is Remaining work: agent-ix/quire-contract-ir#144. FND-011 is
  resolved (#212 round-4 ruling; see Round 6).

Verdict after Round 5: ACCEPT WITH FINDINGS.

## Round 6

Dispositions after the #212 round-4 rulings, 2026-09-19 (issue #212, round-4
comment).

- FND-011 is resolved (#212 round-4 ruling): a surviving `cargo mutants`
  mutant in any listed mapping fails the gate, with no allow-list. ADR-012
  §5.3 states the rule once, and ADR-013 §4 mirrors it. C-06, C-09, C-15 and
  C-20 carry the same `cargo mutants` evidence as C-05.
- FND-010 is Remaining work: agent-ix/quire-contract-ir#144.

Verdict after Round 6: ACCEPT WITH FINDINGS.

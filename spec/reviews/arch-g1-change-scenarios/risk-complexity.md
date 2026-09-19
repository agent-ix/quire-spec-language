---
id: SR-504
title: "Risk and complexity analysis of ADR-010 to ADR-013 (ARCH-G1)"
type: SpecReview
analysis: risk-complexity
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
# SR-504: Risk and complexity analysis of ADR-010 to ADR-013 (ARCH-G1)

## Summary

Reviewed state: branch `task/212-arch-g1-gate` at 457a131, plus the
uncommitted ADR-011 edits of this PR (#245 rule 8 and the O-03 alignment).
The four records are read together as one architecture. Line anchors follow
SR-498: `ADR-011:134` is line 134 of the ADR-011 file at this PR's head.

This analysis does not repeat SR-498's MD-1, MD-2 or FND-001 to FND-007.
It also does not repeat SR-471, the ADR-011-only risk review.

The stage DAG, the family contract and the ownership tables are low in
technical risk on their own. The risk sits where the three records meet:

- The Layer 2 gate #216 and the skeleton spine both sit at the end of one
  serial chain. The chain runs through #213 S-1, S-2 and S-3, and through
  QSpec changes QC-2, QC-3, QC-5, QC-10, QC-11 and QC-15. No record states
  that chain end to end (FND-001).
- The skeleton spine is to be built "in parallel with Layers 1 and 2". But it
  needs M-4, the `replay` facade over QC-1, and candidate sets from `route`.
  So it cannot run in parallel as written (FND-002).
- ADR-013 makes `quire-exact` the value kernel of the CG oracles. The new §2.3
  rule 3 requires the oracle to share no helper with the code under proof.
  For a kernel operation, those two rules conflict (FND-003).

Verdict: **ACCEPT WITH FINDINGS** (3 high, 4 medium, 3 low). No finding is
blocking. None makes SR-498 wrong: SR-498 already fails the gate on MD-1 and
MD-2, and it records scenario 1's independence rule as required evidence, not
as met. Every fix is local text in ADR-011, ADR-012 or ADR-013. None needs a
compatibility layer.

## Risk register

| Item | Tech risk | Volatility | Drivers | Mitigation named in the records? |
|---|---|---|---|---|
| #213 S-1 to S-3 before M-4, M-6a and #216 | High | High | Three serial slices. Each waits on QSpec PRs (ADR-013:791-796, 811-830). X-1 replaces the QSL `value` kernel (19,397 lines, ADR-010:373) and the RT `src/exact` kernel in one change per repository (ADR-011:730). | Partly. Each blocker is listed per slice, but not as the #216 critical path: FND-001 |
| Skeleton spine (§1.1, T-2) | High | Medium | It needs M-4, the `replay` facade, QC-1, E7 candidate sets and CG #86. | No: FND-002 |
| §2.3 proof acceptance with #245 rule 3 | High | Medium | CG oracles consume `quire-exact` (ADR-013:294). A shared helper that the list omits escapes the rule. Each gate needs one Kani run per shared helper. | Partly: FND-003, FND-004 |
| Shared-write cores: the `check` core, the `forms` entry table and the seam-probe list | Medium | High | Every family PR edits the same enums and lists (ADR-012:136-143, 386-394, 432-437). | No: FND-005 |
| Ticket roles for M-6c, M-6e and the replay widening | Medium | Medium | ADR-011 names design tickets as landing PRs, and ADR-012 names other ones. | No: FND-006 |
| Identity preimages (QC-14, QC-18, QC-15) | Medium | High | Node id, `package_id`, obligation id and the seed vectors all change with them. | Partly: FND-007 |
| External sinks: Contract IR #141 and Codegen #86 | Medium | High | Many roles on each, in repositories QSL does not control. OBS-212-1 already shows IR coverage red. | Partly: FND-008 |
| Replay recompile limits | Low | Low | The replay request carries no S1 to S4 limits. | No: FND-009 |
| SEAM-3 remainder | Low | Medium | `protocol_artifact` (22,959 lines) is cut up by four PRs. | No: FND-010 |

## Findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-001 | high | Gate #216 has an unstated serial chain of cross-repository blockers. M-6a lands "with #214 with M-4, before #216" (ADR-011:744). M-4 needs `EmittedPackage` and `VerifiedPackage`, which #213 S-3 builds (ADR-011:449; ADR-013:793). The v2 emitter also waits on QC-10 (ADR-013:820). S-3 waits on S-2. S-2 waits on S-1, #131, QC-15, QC-2, QC-3 and QC-5 (ADR-013:792). S-1 waits on TK-10 (QC-15) (ADR-013:791). Every S1 to S4 stage returns T-4 `StageFailure` (ADR-011:334-336), which S-5 builds after QC-11 (ADR-013:795). So #216 waits on five QSpec changes in three QSpec tickets (TK-07, TK-08, TK-10) and on three serial #213 slices. ADR-013:806-808 says #212 can pass with the QC items open. That is right for #212. But no record says these items are on the #216 critical path. A slip in any one QSpec PR stalls Layer 2. Fix: in ADR-013 §7, add one sentence naming the chain: "#216 transitively waits on S-1, S-2, S-3 and S-5, and on QC-2, QC-3, QC-5, QC-10, QC-11 and QC-15." In ADR-011 §7.3, add "after #213 S-3 and QC-10" to the M-4 order cell. | ADR-011:449, 734, 744; ADR-013:791-796, 806-808, 812-825 |
| FND-002 | high | The skeleton spine cannot be built in parallel with Layers 1 and 2 as written (ADR-011:215-216). (a) It lands the `replay` facade (ADR-011:918; ADR-013:888). `replay` recompiles digest-addressed source from the QC-1 byte provision (ADR-011:250, 264-271), and QC-1 is unmerged (ADR-013:811). (b) It moves CG's pin to a QSL revision with M-4 (ADR-011:745), so it waits on the whole FND-001 chain. (c) E7 takes the `route` candidate sets (ADR-011:248), and CG `negotiate_*` takes the candidate set only after Codegen #86 (ADR-012:881). `route` is #185, which waits on #213 S-6 (ADR-013:776). S-6 waits on #222 accepted (ADR-013:796). The skeleton is the only proof-and-replay evidence until #217 (ADR-011:947-948). If it slips to the end of Layer 2, the program's early end-to-end check comes last. Fix: in ADR-011 §1.1, state what the skeleton's one Boolean clause uses and what it does not. Proposed text: "The skeleton's clause has no dependency package and no domain package, so the §4 dependency binding and QC-10 are not exercised. It needs M-4 and the `replay` facade over QC-1. It reaches E7 with a candidate set of one Kani backend written by `route`. If #185 has not landed, the skeleton waits for it; it does not stub the candidate set (FB-07)." Or move the skeleton into Layer 2 after M-4 and say so. | ADR-011:215-216, 248, 250, 264-271, 745, 918, 947-948; ADR-012:881; ADR-013:776, 791-796, 811, 888 |
| FND-003 | high | The #245 independence rule conflicts with the kernel's consumers. ADR-011 Decision 8 and §2.3 rule 3 require the proof's expectation to be derived independently of the function under proof. A helper that the oracle shares with the code under proof must fail the proof when mutated (ADR-011:134-143, 390-391). ADR-013 O-13 makes `quire-exact` the value kernel of the CG oracles (ADR-013:294). The crate graph adds CG → `quire-exact` (ADR-011:676, 694). For a proof over a `quire-exact` operation, an oracle built on the kernel shares at least `Value` decode and normalization with the code under proof. Rule 3 then fails by construction, or the proof is vacuous: the #245 defect again. SR-498 scenario 1 records the independence rule as required evidence. No record says where an independent expectation comes from for a kernel operation. Fix: in ADR-013 O-13, after the Consumers sentence, add: "A CG oracle for a proof over a `quire-exact` operation derives its expectation without calling that operation or any kernel helper it uses. It takes the expectation from the QSpec operation vector or from its own arithmetic over the harness's primitive inputs. It uses `quire-exact` types only after the expectation is computed. ADR-011 §2.3 rule 3 applies to every helper that is still shared." | ADR-011:134-143, 381-391, 676, 694; ADR-013:294; SR-498 scenario 1 |
| FND-004 | medium | §2.3 rule 3 has no checked list and no cost bound. Each gate publishes a checked-in list of claimed modules, and #219 checks that list against the transcript census (ADR-011:381, 393-396). No record requires a list of shared helpers, or says who derives it and what checks it is complete. An omitted helper escapes rule 3 silently. Each listed helper costs one full Kani run per gate. With CG oracles over every family, that cost grows with families × helpers. T-10 (CG harness gate) and RT #53 grow with it (ADR-011:926). Fix: in ADR-011 §2.3, add: "Each counted gate also publishes a checked-in list of the helpers its oracle shares with the code under proof, derived from the harness crate's call graph. #219 checks the list against that call graph and fails on an unlisted shared helper." In T-10, add the shared-helper list and its mutation runs to the scope. | ADR-011:381-396, 926 |
| FND-005 | medium | The shared cores are write hotspots for parallel family work. The `check` core holds `CheckContext`, the family checker trait, `FamilyOutcome`, the checked node enum (S3), the canonical clause kind (S5), and the checked types that more than one family reads (ADR-011:518; ADR-012:136-143, 386-394). §12.1 and §12.2 each edit the `check` core, the `forms` core entry table and `token` (ADR-012:738, 742, 767, 770, 774). A clause-kind change forces arms in `TemporalTrace` and `Relation` (ADR-012:768). The seam-probe list must match the E0004 set "no more and no fewer", so every family PR edits it too (ADR-012:432-437). #218, #220, #187 and #188 can then run at the same time, and all write these files. #205 requires one writer for overlapping QSL modules. ADR-011 states that order only for `value` and `model` (ADR-011:725-726). Fix: in ADR-012 §12, state one single-writer order for the core enums, the entry table and the seam-probe list across the family tickets. Or state that each family PR rebases on the previous one's core edit before it merges. | ADR-011:518, 725-726; ADR-012:136-143, 386-394, 432-437, 738, 742, 767-770, 774 |
| FND-006 | medium | Some lane deletions are owned by design tickets. ADR-011 says #220 lands the state evaluator and #222 lands the temporal evaluator, and each deletes the old module in the same PR (ADR-011:646-647, 746). ADR-012 names #220 and #222 as design inputs, and #120, #121, #164, #188 and #189 as the implementing tickets (ADR-012:123, 125, 883, 885). M-6e names #221 and #223, which ADR-012 also treats as design tickets, with #187, #218, #191, #192 and #198 as implementers (ADR-011:748; ADR-012:124-127, 884-887). T-3 adds deletion exit criteria to #220, #222, #221 and #223 (ADR-011:919). Also, ADR-011 and ADR-013 give #214 the per-family widening of `replay` (ADR-011:78, 591; ADR-013:888). But #214 lands only function application, and the other families land later (ADR-012:122-127). If a deletion sits on a design ticket that ships no evaluator, the owner ruling "each old path is deleted in the PR that lands its spine replacement" (ADR-011:736, 898-901) has no PR to enforce it. Fix: in ADR-011 §6.2 (`state` and `temporal` rows), M-6c, M-6e and T-3, name the implementing tickets from ADR-012 §14.1: #120, #121, #164 for state; #188 and #189 for temporal; #187 for sum and case; #218, #191, #192 and #198 for protocol and relation. In ADR-011:78 and ADR-013:888, say that each family's implementing ticket widens `replay` for that family. | ADR-011:78, 84, 591, 646-647, 736, 746, 748, 898-901, 919; ADR-012:122-127, 883-887; ADR-013:888 |
| FND-007 | medium | Identity preimages are still changing, and every identity is built on them. QC-18 changes the node-identity preimage (ADR-013:828). QC-14 changes the obligation identity and regenerates the AD-016 seed vector (ADR-013:824; ADR-011:324). QC-15 adds six kernel identity types (ADR-013:825). A node id feeds `package_id` (O-02), the obligation identity (O-09), the packet and the replay check (O-26). So one late preimage change invalidates vectors in QSpec, QSL, CG and IR at once. The skeleton, #213 S-2 and CG TK-05 all produce such vectors. Fix: in ADR-013 §7, add: "QC-14, QC-15 and QC-18 merge before any slice that checks in identity vectors (#213 S-2, TK-05, and the skeleton spine)." | ADR-011:324; ADR-013:792, 824-828, 892 |
| FND-008 | medium | Two external tickets carry a large share of the combined architecture. Contract IR #141 holds v2 intake of value, expression and temporal nodes, the S6 enum decode, removal of 27 `as_str()` sites, the string-edge scan and the IR root → QSL edge removal (ADR-011:688; ADR-012:391, 410-411, 888). T-5 still asks whether #141 removes that edge (ADR-011:921). Codegen #86 holds the S9 enum, `negotiate_*` over candidate sets and extents, the solver-absence test, the S6 matches and the string-edge scan. #185's exit, #188, #189 and #217 wait on it, and it waits on #141 and QSpec #134 (ADR-012:881, 913-916). SR-498 OBS-212-1 shows IR coverage red today. The records name no slice plan for either ticket. Fix: in ADR-012 §14.2, split the #86 amendment into bounded children: S9 enum with candidate-set input; solver-absence test; S6 matches after #141. Ask the IR owner to split #141 the same way: v2 intake decode; predicate and temporal admission with the edge removal. | ADR-011:688, 921; ADR-012:391, 410-411, 881, 888, 913-916; SR-498 OBS-212-1 |
| FND-009 | low | Replay has no stated compile limits. Every stage entry takes explicit limits (ADR-011:356-362). The replay request carries only the `quire.value.accounting/v1` limits for S6a (ADR-013:635). `replay` also recompiles the proved package and each dependency through S1 to S4 (ADR-011:264-271). No record says which limits bound those compiles. If `replay` uses built-in defaults, a replay verdict depends on facade defaults that are not in the request identity. Fix: in ADR-013 O-26, state that the S1 to S4 limits for the recompile are request members, or are fixed constants of the executor's toolchain pin, which O-27 already records. | ADR-011:264-271, 356-362; ADR-013:635, 647 |
| FND-010 | low | SEAM-3 has no rule for its remainder. `protocol_artifact` (22,959 lines, ADR-010:369) is cut by M-6c (#220, #222) and M-6d (#218, #223 and the IR change) (ADR-011:609, 746-747). SEAM-2 has "the last one deletes the remainder" (ADR-011:748). SEAM-3 has none. Parts that no lane PR touches, such as the `value::containment` consumer (ADR-011:642), have no deleting PR. Fix: in the ADR-011 SEAM-3 row, add "the last of these PRs deletes the remainder of `protocol_artifact`". | ADR-010:369; ADR-011:609, 642, 746-748 |

## Volatility of the combined decisions

Low: the stage order and edge table (ADR-011 §1, §2.2), the family catalogue
and closed-seam rule (ADR-012 §1, §5.1), and the ownership rules R-01 to R-10
(ADR-013 §1). They follow from accepted AD-016 and owner rulings.

Medium: the family contract's Rust shape (ADR-012 §2) and the stage type names
(ADR-013 T-1). Both are design names that #213 and #214 may respell within
stated rules.

High: the QSpec changes on the Layer 2 critical path (FND-001), the identity
preimages (FND-007), #222's extent and mode vocabulary, which reaches #185
through #213 S-6 (FND-002), and the §2.3 proof acceptance with rule 3
(FND-003, FND-004).

## Top hazards for live review before planning

1. FND-001 and FND-002: the #216 and skeleton critical path through #213 and
   QSpec.
2. FND-003: where a kernel proof gets an independent expectation.
3. FND-006: lane deletions owned by design tickets.
4. FND-005: single writer for the `check` core and the seam-probe list.

## Failure-domain gaps

The failure-domain analysis of this gate is a sibling record in this
directory. The overlap with this analysis is FND-003 and FND-009. In both, a
proof or replay verdict depends on an input that no record pins: a shared
kernel helper, or an unstated compile limit.

## Round 3

Dispositions after the #212 rulings, 2026-09-19 (issue #212, newest two comments).

- FND-001 is Remaining work: #216.
- FND-002 is Remaining work: CG #87. #212 does not wait on it (#212 rulings, 2026-09-19,
  SR-502 FND-001).
- FND-003 is resolved (#212 rulings, 2026-09-19, SR-500 FND-001): ADR-013 O-13 forbids a CG oracle
  from computing its expectation with the kernel operation under proof.
- FND-004 is resolved (#212 rulings, 2026-09-19, FND-006): the shared-helper set is computed from
  the build and checked by #219. The per-helper run cost is Remaining work:
  #219.
- FND-005 is Remaining work: #214.
- FND-006 is resolved (#212 rulings, 2026-09-19, SR-504 FND-006): the lane deletions name the
  implementation tickets.
- FND-007 is Remaining work: #213.
- FND-008 is Remaining work: IR #141 and CG #86.
- FND-009 is resolved (#212 rulings, 2026-09-19, FND-012): the E9 recompile runs under the proved
  package's recorded stage limits.
- FND-010 is resolved (#212 rulings, 2026-09-19, SR-504 FND-006): SEAM-3 says the last lane PR
  deletes the remainder.

Verdict after Round 3: ACCEPT WITH FINDINGS.

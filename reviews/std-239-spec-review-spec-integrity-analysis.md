---
id: SR-14050
title: "integrity review of STD-239 psc edges built and omitted (PR #1192, QSpec PR #331)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@39f1130926a6da9993c49a56fef0365d3a7eecb4; spec/decisions/ADR-025-weak-memory-models-for-parallel.md, spec/functional/FR-223-order-seq-cst-events-by-the-rc11-partial-sc-order.md, spec/test-cases/TC-668-rc11-partial-sc-order.md; agent-ix/quire-specification@4010594880b59e0154cdc9f7e0a86d0aca18339b; spec/functional/memory/FR-437-explore-release-acquire-memory.md, spec/test-cases/TC-391-litmus-conformance-vectors.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-025
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-223
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-668
    type: reviews
---
# SR-14050: integrity review of STD-239 (PR #1192, QSpec PR #331)

## Summary

Ticket: STD-239. Diffs reviewed: QSL `git diff origin/main...39f1130`
(3 files, +44/-10) and QSpec `git diff origin/main...4010594` (2 files,
+37/-14). Both PRs are based on `main` and are not stacked. The open PRs
#1193 and #1194 (QSL) and #332, #333 and #336 (QSpec) are stacked on these
branches, so their FR-223-AC-5 and later, PSC-7, LT-12 and later, and
FR-437-AC-8 and later are this PR's ids carried forward. No id collides.

Checks:

- Litmus recomputation. The script is in the scratchpad at
  rv-ra-a/litmus/lit.py and lt.py. It enumerates every execution of each
  program under `sc` (interleavings), under `tso` (buffers, with TSO-5's
  `XCHG` for a `seq_cst` store and TSO-6's `MFENCE`), and under RC11 as an
  axiomatic model (Lahav et al. 2017: rf, mo, coherence, `po ∪ rf` acyclic,
  `psc` acyclic). It also runs `ra` operationally by RA-1 to RA-6 and
  PSC-1 to PSC-3 as this PR words them, without garbage collection. LT-12,
  LT-13 and LT-14 match the table in every column: `sc` forbidden, `tso`
  forbidden, `ra` allowed for LT-12 and LT-13 and forbidden for LT-14. RC11
  forbids all three. As a sanity check, SB with `seq_cst` accesses and SB
  with fences come out forbidden everywhere, IRIW with `seq_cst` accesses
  comes out forbidden, and IRIW with release/acquire comes out allowed under
  RC11 and `ra`. The code as it stands has no fence-to-access edge
  (ra.rs:604-615) and admits LT-14, which matches the PR's "Remaining work"
  note.
- Lead default 3, whether `S` carries the `eco` part of the fence clause.
  I checked this exhaustively over two program families:
  - 36,864 programs, each with two threads of the form `op; fence sc; op`,
    where each `op` is a load or store of X or Y, relaxed or `seq_cst`, plus
    an optional third thread with one `op`;
  - 20,736 two-thread programs with release and acquire accesses added.

  The `eco` part of `[Fsc];hb;eco;hb;[Fsc]` changes RC11's outcomes in 256
  of these programs. `ra` excludes every one of those outcomes, and no
  RC11 outcome is missing from `ra`. Tens of thousands of further random
  programs, each checked against RC11, also had no RC11 outcome missing from `ra`. So the
  enumeration supports the default and the PSC-7 claim "every RC11
  execution is an `ra` execution". It is not a proof: the programs have no
  RMW, no non-atomic access, no garbage collection, and at most 8 events.
- Lead default 4, the fork/join corner. This one is sound. RC11's `sb`
  includes fork and join, so the fence-to-access edge is also an RC11 edge
  there.
- Lead defaults 1 and 2. These match RC11: LT-12 and LT-13 are cycles that
  use exactly the PSC-7 (a) and (b) edges. The random search also found
  outcomes outside PSC-7's stated kinds; see FND-001 and FND-002.
- QSL and QSpec agree, except FND-005.
- Ids are unique. Wording describes behaviour only. The PR bodies do not
  paraphrase review findings.
- Gates. `make check-spec-validation` exits 0, with no diagnostic on a
  touched file. `tools/check-index-completeness.sh` passes. The PR touches
  no `docs/` file and no test-parsed vector table. QSpec `quire validate
  --scope . spec/**/*.md` reports 134 failing documents, the same as main,
  none of them a touched file, and the two touched files validate cleanly.
  `cargo test --test checked_package_v2 --test backend_provider_v1` passes
  14 and 117 tests. Tools: quire 0.36.2 (engine 0.50.2) and quoin 0.28.3,
  both newer than the rollout's 0.36.1/0.50.1 and 0.28.1.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | PSC-3's edge list has no edge from an SC access `e` of another thread to a `seq_cst` fence `x` that `e` happens before. That is RC11's `[Esc];scb;hb?;[Fsc]`, with `scb` = `po`, or `hb\|loc` when `e` is itself the release head. The frontier decides it (PSC-2), but PSC-7 does not list it. So `ra` admits RC11-forbidden outcomes that PSC-7 does not name, and PSC-7's sentence "`ra` additionally admits an execution only when every RC11 `psc` cycle through it uses an omitted edge" is false as written. Two reproductions by script. First, LT-12 with `fence sc` between `v`'s two loads (`u`: `X := 1` sc; `Y := 1` release ‖ `v`: `a := Y` acquire; fence sc; `b := Z` sc ‖ `w`: `Z := 1` sc; `c := X` sc), where a = 1, b = 0, c = 0 is allowed by `ra` and forbidden by RC11, because the cycle needs `X := 1 → fence`. Second, the fuzz case `[X:=1 sc; Y:=2 sc; r0:=Z sc] ‖ [r1:=Y rlx; fence sc; r2:=X sc] ‖ [Z:=3 rel; Y:=4 sc]`. With this edge added (`e → x` for each live SC access `e` that happens before `b`'s view when `x` is a fence), both are excluded. With this edge and FND-002's fix, every other `ra`-only outcome then passes the RC11 check with the PSC-7 (a) edges removed and every `scb` step required to join two SC events, and no RC11 outcome is lost. Add the edge to PSC-3, FR-437 and FR-223, or list it in PSC-7 with its reason. | spec/decisions/ADR-025-weak-memory-models-for-parallel.md:590,594; spec/functional/memory/FR-437-explore-release-acquire-memory.md:105 |
| FND-002 | medium | RA-5 sets a `seq_cst` fence's release views `rel_b(ℓ)` to `cur_b` before the fence becomes an SC event. PSC-2 does not say whether those views' frontier names the fence. If they do not (the code's order: ra.rs `fenced` sets `rel` before `sc_event`), a relaxed store after the fence carries no record of it. The fence-to-access edge that LT-14 relies on is then lost. Script: LT-14 with `Y := 1` relaxed instead of release has a = 1, b = 0, final P = 2 allowed by `ra` and forbidden by RC11 (the fence is a release fence, so it synchronises with `a := Y` acquire). The outcome is excluded once the release views name the fence. PSC-7 does not cover this outcome, unless (b)'s wording is stretched (FND-004). State that a `seq_cst` fence's release views record the fence in their frontier. | spec/decisions/ADR-025-weak-memory-models-for-parallel.md:219,589 |
| FND-003 | medium | PSC-3 (and FR-437) decide `hb\|loc` edges against "`b`'s current view" without saying whether, for a `seq_cst` load, that view is taken before or after RA-2 joins the read message's view. The old text ("each live `e` that RC11's `psc` places before `x`") was complete by definition. The new list is not, under the before-join reading: it loses the edge from an SC write to the SC load that reads it (RC11 `hb\|loc`, through `sw`). Script: the all-`seq_cst` program `X := 1` ‖ `a := X`; `b := Y` ‖ `Y := 1`; `c := X` reaches a = 1, b = 0, c = 0 under the before-join reading, which is not an SC outcome. It does not under the after-join reading, which the code uses (ra.rs `load` calls `read_from`, then `sc_event`). Say that a load's edges are decided from its view after RA-2's join, or list the rf edge explicitly. | spec/decisions/ADR-025-weak-memory-models-for-parallel.md:590; spec/functional/memory/FR-437-explore-release-acquire-memory.md:107 |
| FND-004 | medium | PSC-7 (b) attributes the omitted edges to "the `po?` around `[Fsc]` in `psc_base`", but RC11's `psc_base` is `([Esc] ∪ [Fsc];hb?);scb;([Esc] ∪ hb?;[Fsc])`, with `hb?`. The class "an edge between an SC fence and an SC access that RC11 derives through a non-SC event adjacent to the fence in program order" also, read literally, covers LT-14's edge, because `Y := 1` release is a non-SC event that is po-adjacent to the fence. Yet PSC-3 builds that edge and FR-223-AC-6 and FR-437-AC-9 require it. Two implementers would draw the line differently. State the class by relation: the edges `[Fsc];hb?;scb;[Esc]` and `[Esc];scb;hb?;[Fsc]` whose `scb` step has a non-SC access at its fence-side end, as in LT-13's `rb` from a relaxed load. The same wording is in FR-437 "Omitted clauses" and FR-223's behaviour bullet. | spec/decisions/ADR-025-weak-memory-models-for-parallel.md:594; spec/functional/memory/FR-437-explore-release-acquire-memory.md:119; spec/functional/FR-223-order-seq-cst-events-by-the-rc11-partial-sc-order.md:62 |
| FND-005 | medium | FR-437-AC-9's second clause, "a thread that acquired it and stored with `seq_cst` leaves the third thread's `seq_cst` load of the fenced location enabled for the new message only", leaves out the condition that `w`'s `P := 2` is mo-after `P := 1`. QSL FR-223-AC-6 states that condition. Without it the clause is false. With `P := 2` mo-before `P := 1`, `b := X` reading 0 is enabled, and the script finds a = 1, b = 0, final P = 1 reachable under `ra`, RC11 and `sc`. So the QSpec and QSL halves disagree. Add "and the third thread has stored `P := 2` after it in modification order". | spec/functional/memory/FR-437-explore-release-acquire-memory.md:179 |
| FND-006 | low | PSC-2 says the frontier decides "whether it happens before the view", but `S` is a view (RA-1) and RA-5 joins it into `cur_b`. After a `seq_cst` fence, the frontier therefore names events that reach the thread only through `S`, not through RC11 happens-before. PSC-3's "happens before" edges then also include `S`'s reach. The enumeration found no RC11 execution lost by this, but the text claims happens-before. Say that the frontier covers happens-before and the reach of `S`. | spec/decisions/ADR-025-weak-memory-models-for-parallel.md:589 |
| FND-007 | low | RU-3 still says "only the partial SC order is exact", and its refs read "PSC-1 to PSC-6". With PSC-7, `ra` admits more than RC11 for `seq_cst` accesses. Restate it as "admits RC11's executions and, beyond them, only PSC-7's" and extend the refs to PSC-7. | spec/decisions/ADR-025-weak-memory-models-for-parallel.md:561 |
| FND-008 | low | STD-239 lists three items, and its acceptance asks for a rule for each. This PR answers items 1 and 2. Item 3, whether a removed SC read's own edges to later writes need anything beyond its predecessors in `F(ℓ)`, is answered only in the stacked STD-246 PR (#1193, PSC-4 (a)), and neither PR #1192 nor the ticket says so. Note in the PR body or on the ticket that item 3 lands with STD-246. | spec/decisions/ADR-025-weak-memory-models-for-parallel.md:591 |

## Verdict

Not mergeable as is: one high and four medium findings. The LT-12, LT-13
and LT-14 cells are right in every column (recomputed by script). Lead
defaults 1, 2, 4 and 5 are consistent with RC11 and leave every AC
buildable. Exhaustive enumeration supports lead default 3 (the `eco` part
carried by `S`) on the families checked. It is not a proof, and the owner
should still confirm it. The QSL and QSpec halves agree, except FND-005.
The fixes are:

- add the access-to-fence edge, or omit it by name (FND-001);
- state the release-view frontier of a `seq_cst` fence (FND-002);
- state the view a load's edges are decided from (FND-003);
- restate PSC-7 (b) by relation (FND-004).

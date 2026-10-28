---
id: SR-1753001
title: "code review (with rust-review) of PR #1753 (QSL-1005): dense-time routing"
type: SpecReview
analysis: code-review
date: 2026-10-28
scope: "agent-ix/quire-spec-language; PR #1753 git diff origin/qsl-544-refinement-safety...d8404b5a4 (12 files): qsl-route lib.rs, request.rs, deadlock.rs; qsl-semantics family/requirements.rs, check/mod.rs, temporal_clause(.rs, /timed.rs); qsl-analyze zone_check/manifest.rs, refine.rs; tests"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
---
# SR-1753001: code review of PR #1753 (QSL-1005)

## Summary

Ticket: QSL-1005. Head reviewed: d8404b5a4, on qsl-544-refinement-safety (20b4a7c8b).

**No wire change.** I emitted three packages on the base (20b4a7c8b) and on the head and compared their bytes:
- the dense `Rpc` claim under the infinite-trace profile;
- the dense `Rpc` claim under the timed profile;
- `CasRefinesCounter`.

The SHA-256 of each pair is identical (d478cb02…, 0758b3e6…, 9efe675c…). `Requirements::dense_time` and `BackendDescriptor::dense_time` are QSL-internal: neither is written to the package or the FR-331 manifest. `descriptor_from_manifest` sets `false`, and only EN-6's linked `descriptor` calls `with_dense_time`.

**Routing** (`candidates_of`). A refinement record over a non-dense concrete subject goes through `restrict_to_untimed_refinement`, which drops EN-6. Every other item goes through `restrict_to_dense_time`:
- a dense item keeps only engines that `decides_dense_time` (EN-6), matching FR-243 l.23 and FR-430-AC-1;
- a non-dense temporal item keeps `[en-1, en-6]` unchanged, which is intended.

The restriction runs before `reduced_candidates`' `restrict_to_reductions`. So a dense item that selects partial-order reduction narrows to EN-6, and EN-6 lacks it, so the item settles V-8 naming the reduction, as FR-430 "Reductions" and #1690 require. `keep_candidates` prunes `fairness` with the candidates, and an unknown named backend passes unchanged (FR-075-AC-3).

**Dense marks.**
- A temporal clause is dense when its `over` parameter's model declares `time dense` (`timed::over_dense_time`).
- A refinement record is dense when its concrete model does. A timed abstract model is refused at S3 (FR-430), so the abstract side cannot be dense.
- Probabilistic claim records are not marked. They keep their own route, which matches FR-243's exception.

In a mixed package, each record's mark comes from its own subject's model, so a dense subject and an untimed one in one package route independently. The derived deadlock and time-lock items take `is_dense(models_of)`, and the deadlock items merge with `|=` per subject.

**Rust.** `with_dense_time` and `over_dense_time` are `#[must_use]` builders, and `keep_candidates` takes a generic `Fn`. Nothing is wrong.

Gates at d8404b5a4 (locked-build, jobs 4, `--all-features`, debug info off):
- qsl-route lib: 19/19.
- qsl-analyze it `dense_route`/`timed_refinement`/`refinement_request`/`deadlock_item`/`time_lock_item`/`zone_search`/`request`/`reduction`/`symmetry`: 125/125.
- qsl-semantics it `temporal`/`timed`/`refinement`: 175/175.
- clippy `-p qsl-route -p qsl-analyze -p qsl-semantics --all-targets -D warnings`: exit 0.
- fmt: exit 0.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean. Mergeable.

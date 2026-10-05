---
id: SR-1320
title: "QSL-482 gap analysis of PR #633 (NFR-001 parse-time metric, FR-276-AC-2 at 200k, TC-757 note)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@1b7e37d85f032bcdeabbdae6fecf535a2d9f66d9; PR #633 diff against origin/main; spec/non-functional/NFR-001-bound-syntax-work.md; spec/tests.md (TC-757); qsl-replay/src/spine/lifecycle/tests.rs; qsl-bench/src/bin/qsl-bench-probe.rs; Makefile (bench-probe)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/NFR-001
    type: reviews
---
## Summary

Ticket: QSL-482. Manual check of the owner-ruled scope against the diff; no plan
bundle.

- **NFR-001 parse-time metric: value test.** The threshold is a ratio (time per
  declaration at 200,000 within 2x of that at 2,000), not a frozen timing, so it
  holds across machines and only fails when growth is super-linear. That is a
  real bound on a real property. The PR body's measurement meets it: S1+S2 is
  108 ms at 2,000 (54 us/decl) and 10,816 ms at 200,000 (54 us/decl), ratio
  about 1.0.
- **FR-276-AC-2 / TC-757 step 2.** Now 200,000 declarations as the AC names;
  binding `a_check_cancelled_from_another_thread_stops_within_one_charge` to
  TC-757 / FR-276-AC-2 is correct.
- **TC-757 remaining-work note.** Removed; the row now says step 2 runs at
  200,000. The `DECLARATIONS` doc comment's stale S1 explanation is removed too.

## Verdict

Approve with one medium finding: the metric's measurement cannot be reproduced
from the repo's own benchmark entry point.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The NFR-001 parse-time metric (method `benchmark`, sizes 2,000 / 8,000 / 50,000 / 200,000) has no repo entry point that produces it. `make bench-probe` (the documented one-shot large-input timing target) does not call the new `parse-volume` subcommand, and nothing computes the per-declaration ratio the threshold names; the only measurement is the table in the PR description. Fix: add `for n in 2000 8000 50000 200000; do $(BENCH_PROBE) parse-volume $$n \|\| exit 1; done` to `bench-probe` (and optionally have the probe print `us_per_decl`). Timing stays out of `ci:` per the Makefile's rule. | Makefile:398-407 |

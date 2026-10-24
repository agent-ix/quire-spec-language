---
id: SR-12885
title: "Gap analysis (manual acceptance check) — QSL-865 make ci on macOS (PR 975)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@ea854a05679de3c70fefb8bcc2e9b2282ee9c465; QSL-865 description and comments (three named host failures, xtask is_unreaped, Makefile GNU Make 3.81 split, xtask canonical tempdirs) against the PR 975 diff and /Users/peter/dev/qsl-lane2/logs/qsl865-ci3.log"
review_set: subset
---

## Summary

Ticket: QSL-865. This is a manual acceptance check of each ticket item against the diff, focused test runs and the coder's full `make ci` log. A Quoin matrix gap analysis was not run, because the PR changes only test and CI plumbing: no spec, no production code and no trace tags change (the new `cli_preserves_non_utf8_os_paths` carries the same TC-017 tag as the code it was split from). Plan completion: not assessed.

## Verdict

**All items met.**

- AC1, portable size in `check-no-committed-binaries.sh`: met. `wc -c` replaces GNU `stat -c%s`, and the script exits 0 on macOS.
- AC2, `ten_thousand_edge_chain` without `/proc` on macOS: met. It reads peak RSS through `/usr/bin/time -l` (bytes), Linux keeps VmHWM, and it passes on draco. The ticket allowed this approach.
- AC3, non-UTF-8 path test on macOS: met. The file-name half is gated off macOS with an APFS EILSEQ reason, and the argv half still runs. Both halves run on Linux.
- AC4, comment 1, xtask `/proc/{pid}` liveness: met. `ps -p` is correct for zombies and for reaped pids on macOS (probed).
- AC5, coder finding, Make 3.81 `override export`: met. Probed on 3.81 and 4.4.1.
- AC6, coder finding, `/var` vs `/private/var` tempdirs: met. `cargo test -p xtask --lib` passes on macOS.
- AC7, `make ci` runs to the end on draco: met. The log was written at 19:40, after HEAD ea854a056 (17:53), and ends `exit=0`. It covers ten_thousand_edge_chain, all-features, seam-probe, deny, docs and arch-lint.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

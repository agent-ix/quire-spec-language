---
id: SR-15930
title: "spec review of STD-281 SMT certificate check limits and Cancel (QSL PR #1254)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@241e24fe703c1e3164f43b95f2ca4d7e7069eeb0; spec/functional/FR-314-check-an-smt-proof-certificate.md, spec/functional/FR-127-settle-a-model-check-verdict-as-a-terminal-record.md, spec/test-cases/TC-1165-a-bounded-smt-certificate-check-stops-at-its-limits.md"
review_set: subset
evaluated_revision: "241e24fe703c1e3164f43b95f2ca4d7e7069eeb0"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-314
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-127
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1165
    type: reviews
---
# SR-15930: spec review of STD-281 SMT certificate check limits and Cancel (QSL PR #1254)

## Summary

Ticket: STD-281 (unblocks part of QSL-662). PR quire-spec-language#1254, head 241e24fe7 (matches `git ls-remote`), reviewed as a merge with origin/main 39823fe73 (881eb6325). Three files, spec Markdown only. FR-314's two entries take FR-245 `CertificateCheckLimits` and FR-276 `Cancel`, `SmtProofCheck` gains `Stopped(IncompleteCause)`, a "Limits and cancellation" section charges bytes and edges, FR-127's SMT arm settles `Stopped` V-7, and FR-314-AC-5 with TC-1165 carry the vectors.

These claims hold:

- The settlement is right and matches the existing V-7 rows: FR-127's table settles both a reached limit and a cancellation `failed`, execution `resource-incomplete`, basis `unavailable`, category incomplete, so AC-5's `cancelled{requested}` agrees with the new FR-127 sentence. A stopped check never settles `proved` and stays distinct from `Unverifiable` (`proved`, `Uncertified`).
- The setting names `certificate_check.max_certificate_bytes` and `certificate_check.max_certificate_edges` are exactly the FR-255 engine-table rows of #1244 at 73d07a397, with FR-245's defaults 2^30 and 2^24; they pass the dotted `SettingName` pattern. The PR must merge after #1244.
- The charge model is implementable and deterministic: every unit is a byte read, a printed expected-query byte, a command or a premise/discharge entry, charged before the work in rule order, with Cancel polled at each charge as FR-338 does. The bytes-1 vector is denied at the second charged byte, and the default limits cover the TC-893 certificates.
- Ids: TC-1165 and FR-314-AC-5 appear on no other open PR head. #1246 also edits FR-127; a trial merge of both is clean. No status wording or compat layer. The gate on the merge exits 0 with no diagnostic on a touched file.

The `code-review` method was skipped, because the diff contains no code.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | major: `Stopped(IncompleteCause::LimitReached{limit, value, setting})` with `limit` `max_certificate_bytes` or `max_certificate_edges` cannot be built. FR-127 types the member as `LimitReached { limit: ModelCheckLimit, ... }`, and `ModelCheckLimit` is closed to FR-126's members plus FR-166's and FR-184's additions; none is a certificate limit. FR-245 sidesteps this with `Stopped(IncompleteCause, CertificateLimit)`, which lead default 2 drops. Fix: in FR-314, extend the enum the way FR-166 and FR-184 do (`pub enum ModelCheckLimit { // FR-126's members, plus: MaxCertificateBytes, MaxCertificateEdges }`) and write the AC with those variants, or state another typed home for the two limits. | spec/functional/FR-314-check-an-smt-proof-certificate.md:110,186-190; spec/functional/FR-127-settle-a-model-check-verdict-as-a-terminal-record.md:98; spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:134-138 |
| FND-002 | medium | minor: the precedence rule (a rejection an earlier rule finds wins over a later denied charge) is not exercised. AC-5's `QueryMismatch` vector sets `max_certificate_bytes` to the full byte total, so no charge is denied, and a checker that returns `Stopped` in place of an earlier rejection would also pass. Fix: add a budget that admits the bytes charged up to the first-byte mismatch but is below the total (for example the carried query's bytes plus the expected query's, with the proof bytes left out), expecting `QueryMismatch`, not `Stopped`. | spec/functional/FR-314-check-an-smt-proof-certificate.md:191-192,258; spec/test-cases/TC-1165-a-bounded-smt-certificate-check-stops-at-its-limits.md:29-30 |
| FND-003 | low | minor: `max_certificate_edges` is tested only one below the count and at the 2^24 default. A checker that over-counts (for example charges the `anchor` or an `assume`'s implicit premise) still stops at one below and verifies at the default. Fix: also check with `max_certificate_edges` equal to the command count plus the premise and discharge entries, expecting `Verified`. | spec/functional/FR-314-check-an-smt-proof-certificate.md:258 |
| FND-004 | low | minor: the new Dependencies entry names FR-276 without a link, and the frontmatter adds `depends_on` FR-245 but neither FR-276 (the `Cancel` handle) nor FR-255 (the setting names). Fix: link FR-276 and add both `depends_on` edges. | spec/functional/FR-314-check-an-smt-proof-certificate.md:17-22,270-274 |

## Verdict

Not mergeable until FND-001 is fixed (a small type addition in FR-314 and the AC wording), and only after #1244 merges, because the two setting names are #1244's FR-255 rows. FND-002 to FND-004 are minor and cheap in the same round. The charge model, the V-7 settlement, the cancellation path, the setting names, the trial merge with #1246, id uniqueness and the gate are verified. Owner or planner decisions: choose FND-001's typed home for the certificate limits (extend `ModelCheckLimit`, which is the FR-166/FR-184 pattern, or restore FR-245's separate `CertificateLimit`, which FR-245 itself never defines); confirm merge order #1244 then #1254.

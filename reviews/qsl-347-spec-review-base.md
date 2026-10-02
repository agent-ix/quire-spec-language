---
id: SR-1190
title: "Spec review of quire-spec-language PR #588 commit de5a57e0 (QSL-347): ADR-011 §4 names the I2 reader's consumer"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@6ffdca6fe18c174773ef1b36e335bcc4cd7a33f2; PR #588 commit de5a57e0 against main 2ece5712"
review_set: subset
---
# Spec review of quire-spec-language PR #588 commit de5a57e0 (QSL-347)

## Summary

Ticket: QSL-347. Commit de5a57e0 adds to ADR-011 §4 that the I2 reader has one consumer, the S4 source resolution (ADR-015 D-1 step 6). Checked against the code: `qsl_package::read_import_view` (qsl-package/src/checked_v2.rs:825) is called from step 6 "View" of the resolution in qsl-replay/src/spine.rs:1158, after step 5 "Identity", as ADR-015 D-1 orders them. Dependency binding source 2 (`replay`) exists in the same section. The three readers the ticket names (`unsupported_wire_record`, the consumer pinned request `new`, `declared_names`) still carry `dead_code` allowances citing QSL-347; the ADR now states that the reader keeps none of them, which is the target the code half deletes to.

Examined:
- ADR-011 §4 I2 binding (examined)
- ADR-015 D-1 (context_only)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean. The added §4 text matches the code's real consumer and ADR-015 D-1's step numbering. Mergeable.

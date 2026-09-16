---
type: log
title: "Plan-013 — Update Log"
description: "Chronological history of the complete-V1 native delivery plan."
---
# Plan-013 — Update Log

## History

* **2026-09-15** — Created from QSpec baseline `8d0fbad` after Plan-008 Task-010 and PR #64 passed. The plan assigns 83 Agent-A capability rows to nine serial tasks, preserves existing L1–L6 evidence state and installs TC-144 as the repository-local allocation audit.
* **2026-09-15** — Completed Task-046 after the full pre-implementation review, exact QSpec projection audit, local Rust gates and PR-readiness review. Task-047 remains the next serial implementation task.
* **2026-09-15** — Started Task-047 from merged QSL PR #124. Crosswalked the frozen complete-V1 source/package, grammar, CST and incremental-tooling contracts before adding the TC-180/184, TC-222/223 and inherited TC-230 implementation evidence; corrected the task table to leave TC-181 with Task-049, TC-220/221 with Task-052 and authority-dependent TC-182/183 with Task-054.
* **2026-09-15** — Implemented Task-047 with the declarative complete grammar, exact dependency-closed source/profile graph, lossless CST/source mapping and revision-bound formatter/editor services. The pre-merge reviews found and drove fixes for historical token compatibility, declarative-table drift, foreign CST references, self-attested extension/manifest evidence and unchecked formatter semantics. The review removed both self-attested drafts and reallocated FR-132/133 with TC-182/183 to Task-054, where the concrete checked semantic authorities exist; final gates remain pending.
* **2026-09-15** — Corrected a second premature authority claim: Task-047 reuses the existing diagnostic code type and limits formatting to exact-catalog reparse/token correspondence. FR-270–272/TC-047 and the checked-package formatter identity with concrete TC-223 move entirely to Task-054, because exact catalog-source bytes, typed causes and the type-checked package do not exist at the source-admission stage. The stale V1-EXPR-023 mapping to unrelated TC-230 was replaced with TC-047.
* **2026-09-15** — Completed Task-047 after sealing semantic construction behind the unforgeable `ReaderAuthority` boundary while keeping source/editor calls on the authority-free exact-reference `ProfileCatalog`. Independent Rust, semantic and gap reviews passed; the full Rust suite, strict Clippy, doctests, formatting, native Quoin validation and diff hygiene passed. QSpec PR #67 merged the corresponding central allocation correction, leaving the checked reader/package and TC-223 with Task-054. QSL #117 is ready to merge before Task-048 begins.
* **2026-09-15** — Merged QSL #117 as PR #125 (`d2f4345`) and started Task-048 / QSL #118 from that exact mainline. No prior exact-scalar implementation WIP exists to continue; the older signed-64 protocol and model scalar types remain separate compatibility domains rather than substitutes for the complete mathematical semantics.
* **2026-09-15** — Corrected Task-048's stale equality allocation against the authoritative QSpec delivery manifest: #118 supplies equality primitives for its scalar families, while V1-EXPR-016 and complete FR-149/TC-194 closure remain with Task-049 / QSL #119 for records, tuples and collections.

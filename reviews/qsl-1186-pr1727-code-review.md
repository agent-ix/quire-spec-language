---
id: SR-1727001
title: "Code review of quire-spec-language PR #1727: lower parallel controls to V2 (QSL-1186a)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@c73d934be; PR #1727 diff against qsl-1181c-protocol-subject (4 files), against QSpec FR-466 parallel layout and per-operator checks, FR-464-AC-1/2/4 and Control tree, FR-375-AC-23/26, FR-432 (QSpec 48373530)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-specification/FR-466
    type: reviews
---
# Code review of quire-spec-language PR #1727

## Summary

Ticket: QSL-1186 (slice a).

**Scope.** One commit, c73d934be, on `qsl-1181c-protocol-subject`: 4 files. Nothing outside the stack is in the diff.

**The layout, checked against FR-466's `parallel` row:**

| Argument | What the PR writes |
| --- | --- |
| `memory` | The written model as a one-text Group, or the empty Group |
| `branches` | A Tuple of `{label, body}` records, sorted by UTF-8 bytes. My probe of `b`, `B`, `a` gives `B`, `a`, `b`. |
| `join` | `policy`, plus `count` exactly for `quorum` |
| `joined` | Ascending and deduplicated |
| `outstanding` | The written rule, or the empty Group |

- **Operator.** The node is `quire.op.protocol.control` with `profile_operator` `parallel` under the clause's law (FR-464-AC-1/2).
- **Declarations** (FR-464 "Control tree", AC-4): each control is declared at its holder's path followed by its own name. A branch adds no segment, so a branch body is held by its `parallel`.
- **Catalog.** Each `parallel` node joins `ProtocolMembers.nodes`, so the catalog's `fork`/`join` classes lower to it. The memory drain classes need no node.

**The predicate join refuses cleanly.**
- `ProtocolInput` leaves a predicate-join `parallel` unheld, so the whole clause refuses `unsupported_construct`/`not-yet-implemented` naming `parallel` before lowering runs.
- `parallel_arguments`' own `Predicate` arm is an unreachable fault.
- Nothing is emitted for it (tested by `a_predicate_join_is_not_written_yet`).

**Probes** (not committed; in `probe_1727.diff`). Each admitted case below also reads back through IR and is verified:

| Probe | Result |
| --- | --- |
| A `parallel` inside a branch's `sequence` | Declared `[Flow, run, Main, Par, Ay, In]`, its branch body `[…, Ay, In, X]` |
| A `parallel` as a branch body | `[…, Par, In]` and `[…, Par, In, X]` |
| `memory tso` with `join all` | `memory` `["tso"]` |
| `quorum(2)` of 2 joined; `quorum(1)` of 1 joined | Admitted |
| `quorum(0)` | Refuses `QuorumBelowOne` |
| `quorum(3)` of 2; `quorum(2)` of 1 joined | Refuse `QuorumAboveLabels` |
| `join all [a, a, b]`; `any [b, b]` | Admitted. S3 counts the distinct labels, and lowering writes `joined` once each. |
| `quorum(3) [a, a, b]` | Refuses, because the count uses distinct labels, so a written duplicate cannot push `count` past `joined` |
| `all [a]` | Refuses `OmitsBranch` |
| An unknown label | Refuses `Missing` |
| `any []` | Refuses `NoLabel` |
| A duplicate branch label | Refuses `Ambiguous` |
| `outstanding continue` | `["continue"]` |
| `ra`/`tso` with `any` | Refuse `WeakJoin` (S3, outside this slice) |

**Gates** (locked-build, jobs 4, debug info off, `--all-features`, QSPEC_DIR 48373530):
- qsl-semantics: lib 801, it 1328, doc 30, all pass.
- qsl-package lib: 356 pass, 2 ignored, 2 fail. The failures are `conformance_c14…` and `conformance_i2…`, which also fail on main (QSL-1190).
- Workspace clippy `--all-targets -D warnings`: exit 0.

## Verdict

**Clean.** I found no defect: the layout, ordering, declaration paths, member keys and the predicate refusal all match FR-466 and FR-464, nested cases included.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| - | - | No findings. | - |

## Dispositions

Round 1, head 7cd3f46f6. The `parallel` lowering is re-ported onto main's `controls.rs` (#1710, QSL-1126), and I re-reviewed it as new code. `parallel_arguments` and `control_declarations` are carried over unchanged. A `branch` is skipped in `build_order`'s loop, so it has no node and no path segment. Every written control, `parallel` included, enters the member keys through `Run.nodes`. Binder scoping comes from S3's `slots.entries` and `root_out`.

I re-ran the probe matrix on the head; it is not committed. Every admitted case reads back through IR and is verified.
- **Nesting.** Admitted, with the declarations `[…, Par, Ay, In]` / `[…, Ay, In, X]` and `[…, Par, In]` / `[…, Par, In, X]`.
- **Branch events.** Events inside branches are declared `[…, Par, A, E1]` and `[…, Par, B, E2]`.
- **Branch binders after the join.** After `join all`, a later event reads `e1`. After `join any`, `e1` refuses `MissingName` (FR-466: `Out(parallel)` is `In(parallel)` for non-`all` policies).
- **Join refusals.** Each of these refuses: `quorum(0)`, `quorum(3)` of 2, `quorum(2)` of 1 joined, and `quorum(3) [a, a, b]`. So do `all` omitting a branch, an unknown label, an empty list, and a duplicate branch label.
- **Admitted joins and memory models.** `quorum(2)` of 2, `quorum(1)` of 1, duplicate joined labels (written once), UTF-8 branch order, `tso`/`all` and `outstanding continue` are all admitted.
- **Predicate join.** It still refuses `not-yet-implemented` before lowering (`a_predicate_join_is_not_written_yet`).

The four `Fill` protocol-refinement tests now expect `concrete_protocol()` to be `Some`. That is correct: `Fill` holds a `parallel`, which is now written, so the clause has a node.

Gates (locked-build, jobs 4, debug info off, `--all-features`, QSPEC_DIR 48373530):
- qsl-semantics: lib 804, it 1328, doc 30, all pass.
- qsl-package lib: 376 pass, 2 ignored, and only c14/i2 fail (QSL-1190).
- Workspace clippy `-D warnings`: exit 0.

This round adds no findings. The review was clean, and the re-port stays clean.

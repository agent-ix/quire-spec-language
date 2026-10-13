---
id: SR-9030
title: "QSL-726 code review of PR #816"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@3e004e2039af35d8fa340899aca3ed4852748fda; merge-base 2878b9bc14ecf1b2c42e7ddfcd6b4e36374f32a7; base impl/business-model-meanings (now includes #811, #813); qsl-semantics/src/check/dispatch_link.rs; qsl-semantics/src/check/refusal.rs; qsl-semantics/src/check/assemble.rs; qsl-semantics/src/check/check.rs; qsl-semantics/src/check/checked_dispatch.rs; qsl-semantics/src/check/lowering.rs; qsl-semantics/src/check/region.rs; qsl-semantics/src/check/mod.rs; qsl-semantics/src/model/intake.rs; qsl-replay/src/spine/lifecycle.rs"
review_set: subset
---
## Summary

Ticket: QSL-726, AC-3 (QSL-INPUT-AC-3). This is a code review with its Rust lane folded in. It was run read-only over the PR diff at the reviewed sha, in a detached worktree. The PR adds `PackageDeclarations::link_dispatch`, the first production caller of `checked_dispatch_operation`. The spine runs it in `finish_unit` before `check`. The PR also adds a `CheckCause::DispatchLink` carrier, reads an operation's `redefines` at intake, keeps every selected model view, and makes synthesized dispatch functions resolve to source regions so E4 emits.

The PR is stale against its base. #811 and #813 have merged into `impl/business-model-meanings` since the branch was cut. `git merge-tree` shows one textual conflict, the TC-1011 row of `spec/tests.md`. There is also a semantic conflict: #811 deleted `ModelRefusalCause::limit_exceeded()` and `Incomplete::limit_exceeded()` and replaced them with `denial()`, which returns `BudgetDenial`. `dispatch_link.rs` calls both deleted methods, so the merged ref does not compile.

Checked clean:

- The signature of `checked_dispatch_operation` is unchanged, and all test callers are untouched.
- The domain package's `requires`/`ensures` text is never read. `family_clauses` reads only `operation_body`, `unit_clauses_for` and the operation table.
- No `unwrap`, `expect` or `panic!` is added in production code.
- No ticket ids appear in code or spec text.
- There is no unrelated churn.
- The new `CheckCause` variant does not need `make seam-probe`. The probe records where the `__SeamProbe` variant makes a match non-exhaustive. `code()` gets a real arm and no new exhaustive match is added, so the set of probe locations is unchanged.

## Verdict

**FAIL.** FND-001 blocks the merge: the PR must be rebuilt on the current ref. FND-002 and FND-003 are wrong results on valid units, and they contradict the FR-302 sentence the PR adds.

Most of the design is sound for AC-4. Refusals from every family are collected into one `CompileRefusal::Check`, an ambiguous family carries its subtypes, candidates and dominance pairs, and the tables reach `CheckedGraph::dispatch_tables()`. TC-794 aggregation and TC-809 S6a selection can build on this.

The gate logs (`qsl726e-gate-*.log`) are green, but only on the stale merge-base. They prove nothing about the merged ref. Run the full gates again, including the all-features clippy, after the merge.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Stale on base: the merged ref does not compile, and the limit classification contradicts #811. `link_cause` calls `incomplete.limit_exceeded()`, and `model_cause` calls `model.cause.limit_exceeded()`. #811 deleted both and replaced them with `denial()` → `BudgetDenial`. The `Incomplete` arm also hard-codes `stage_limit_exceeded`, which the ruling says a `ModelNormalizationLimitsV1` ceiling never produces. It must be `resource_exhausted`/`insufficient-next-charge`. `DispatchLinkLimit` (setting/bound/reached) is a third hand-built copy of the record that `BudgetDenial` now builds once. Fix: carry a `BudgetDenial` in `DispatchLinkCause` and delete `DispatchLinkLimit`. Take the catalog pair from `ModelRefusal::catalog_code()` for walk ceilings and from `denial()` for `Incomplete`. Return its `catalog_fields()` through `CompileRefusal::fields()` for a `Check` refusal with a `DispatchLink` cause, so the charge record reaches the report. Update the `DispatchLinkCause` doc, which still cites `family-steps`. | qsl-semantics/src/check/dispatch_link.rs:472-516; qsl-semantics/src/check/refusal.rs:655-690 |
| FND-002 | medium | `called_families` picks a family root for every operation whose member name matches a dispatched call, in every selected view. It does not check whether any call can reach that root. An unrelated type with a same-named operation and no bodies is linked anyway, and its `no-applicable` failure refuses a valid unit that never calls it. FR-302 requires a checked package "only when every called family links". The coder lists this as a limitation, but it is a wrong result and cannot be deferred. The FR-302 sentence the PR adds writes it into the spec (see SR-9032 FND-001). Fix: keep a failure for a name-matched family pending. Refuse only when a checked call names that family's table, or select families from receiver static types. | qsl-semantics/src/check/dispatch_link.rs:132-158 |
| FND-003 | medium | The receivers exposed for a table are `call.owner` plus `table.entries()`. Linking skips abstract subtypes (model/dispatch.rs:328, D05), so an abstract type that conforms to the root's declaring type is never exposed. `check` matches `receiver_type` exactly (state_model.rs:327), so a call through a receiver whose static type is that abstract type finds no dispatch operation. This contradicts the PR's own FR-302 sentence, "any type conforming to it ... names that root's table". The coder's listed limitation, a receiver statically typed as a supertype of the root's declaring type, cannot occur: that supertype does not declare the member in this family. Fix: expose every type in the view that conforms to the root's declaring type, from the generalization closure, not only the table entries. | qsl-semantics/src/check/dispatch_link.rs:305-328 |
| FND-004 | medium | `called_members` scans only state clauses and kept operation clauses. A dispatched call inside an `operation-body` is a checked function body too. If no clause also calls that body's callee, the callee's family is never linked, and `check` cannot resolve the call. FR-302 Inputs lists only clause calls, so either the spec should say bodies are excluded, or `called_members` should also scan `operation_bodies`, iterating to a fixpoint because a linked body can call further operations. | qsl-semantics/src/check/dispatch_link.rs:110-128 |
| FND-005 | low | `family_clauses` rebuilds the redefinition family with its own loop, which repeats until no new member is added. This walk is unmetered and costs O(operations × depth). It duplicates the family walk the bridge then performs under `family_steps` in `model::dispatch`. Have the bridge report the family it walks, or supply clause data for every same-named operation in the view and let the bridge choose. Either way the family is defined once. | qsl-semantics/src/check/dispatch_link.rs:163-188 |
| FND-006 | low | There are two silent fallbacks. If `respelled` ever refuses, `receiver_as_parameter` returns the expression unchanged with `SelfRef` still in it, and the body then fails typing with an unrelated error. `merge_linked` clones every body function just to try `with_spans`, and on failure silently keeps the function without spans, which later surfaces as an unlocated occurrence. Map both failures to the existing `invariant()` cause, and move the function into `with_spans` instead of cloning it. | qsl-semantics/src/check/dispatch_link.rs:285-289, 428-438 |
| FND-007 | low | `precondition_span` returns the hull from the first `pre` clause to the last. When the clauses for one operation are not adjacent, the region also covers the declarations in between. FR-096 locates a declaration at its own form's span. Keep one span per contributing clause, or locate the synthesized function at the first clause. | qsl-semantics/src/check/dispatch_link.rs:352-362 |

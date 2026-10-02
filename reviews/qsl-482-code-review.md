---
id: SR-1225
title: "Code review of quire-spec-language PR #605: arena forms, iterative control anchors, CST with no nesting ceiling"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@ec8166e82f696654d39a06e854aeb59baf9338c7; PR #605 diff against origin/main: qsl-cst/src/{lexer,parser,grammar,diagnostic}.rs, qsl-forms/src/{syntax,value,protocol_clause,dispatch,spans,lib}.rs, qsl-semantics/src/check/**, qsl-eval/src/value/expression/**, qsl-replay/src/spine.rs, qsl-bench/src/{check,deep_input,recursion,text_cluster}.rs, qsl-foundation/src/diagnostic.rs, src/parser*.rs, src/syntax.rs, xtask/src/seam_probe.rs, tests/it/family_outcome_layering.rs, and the test modules the API change touched"
review_set: subset
---
# Code review of quire-spec-language PR #605

## Summary

Ticket: QSL-482 (slice B1 of plan v2). The PR stores `qsl_forms::Expression`
as one post-order `Vec<ExprNode>` addressed by `ExprId`, runs the S2 build and
the control-anchor walk on `quire_walk`, gives `TypeForm` iterative
`Clone`/`PartialEq`/`Debug`/`Drop`, and deletes `DEFAULT_FORMS_NESTING_DEPTH`,
`FormsLimits`, the lexer `nesting` limit and `SyntaxLimit::NestingDepth`.
Absence of all four was checked by grep at the head; only ADR-030 prose and
FR-311 still name them (FR-311 is in SR-1226).

Measured at this head (worktree-local target dir):
- A probe crate in the scratchpad called `qsl_replay::spine::compile` on
  `qsl_bench::deep_input::Shape::OptionType(n)` (`type T = Option<…Integer…>;`)
  at `SpineLimits::default()`. S1 and S2 alone (`qsl_cst::parse`,
  `build_unit`, clone, drop) succeed at depth 30,000 on an 8 MiB thread.
  The full compile aborts the process with `fatal runtime error: stack
  overflow`: in a release build on an 8 MiB thread at depth 10,000 (7,000 is
  refused `nesting-depth-exceeded`), in a debug build on an 8 MiB thread at
  depth 1,500, and on a 2 MiB thread at depth 300. On origin/main, the S1
  default nesting ceiling of 64 refuses all of these inputs. FND-001.
- Expression chains (brackets, `+`, `else if`, `let`) under the same probe
  settle as S1 ceiling refusals or the S3 `nesting-depth-exceeded` refusal at
  every depth tried, up to 100,000 with raised limits. They do not crash.
- `cargo test -p qsl-forms --test it` for the five 100,000-deep tests passed
  in 134.6 s (debug). `cargo test -p qsl-route` passed (for PR #606, separate).

Coder questions:
1. Stack overflow: confirmed. S3's `CheckingLimits` depth guards the
   expression walk in `Typer`. It does not guard type-form resolution:
   `assemble.rs` and `check/mod.rs` call `resolve_form`, which recurses once
   per `Option`/collection level (type_form.rs:258, :309). The resulting
   `ValueType` also uses derived, recursive traits (B3, FR-262). FND-001.
2. Own `Vec<ExprNode>` instead of `quire_walk::Arena`: this is justified.
   `Arena::clone` takes a fresh token (quire-walk/src/arena.rs:101-125), so
   `Id`s stored inside nodes would not resolve in a clone, and two equal
   trees built separately would compare unequal. FR-356 Behavior 1 requires
   arena order (each node after its children, bottom-up as one forward loop),
   not the `Arena` type. `Expression` meets that. The cost is that `ExprId`
   is not bound to its tree, so a foreign id resolves to the wrong node, or
   panics in `at` when it is past the end. All production ids come from the
   same tree. The builder's tree invariant is FND-003.
3. Edits outside the lane are the minimum the API change needs. qsl-replay
   swaps `FormsFailure` for `FormsRefusal`, drops `SpineLimits.forms`, and
   moves two tests to constructors. xtask/seam_probe.rs renames
   `Expression` to `ExprNode` in docs and in one scan target.
   family_outcome_layering.rs adds `quire-walk` to the allowed layer-2 and
   layer-3 dependencies. The qsl-eval test modules only move to constructors.
4. The 100,000-deep tests use exactly the depth FR-256-AC-1, FR-257-AC-1/2
   and FR-102-AC-5 name (`DEPTH: usize = 100_000`). Nothing goes deeper. The
   bisection in `deepest_options` is capped at 200,000 and runs at default
   limits. If the run time matters, the fix is an opt-level for the test
   profile, not a smaller depth.
5. TC-169 (`every_variant_and_field_of_the_moved_types_is_unchanged`) is
   ceremony. It freezes the variant and field list so that the build breaks
   when anything changes. It checks no behaviour, and this PR had to rewrite
   it. FND-004.
6. `Expression::respelled` silently discards a rewrite that changes a
   node's children. It should refuse instead. FND-002.

Rust lane (rust-review): no new `unsafe`. The only new panic is the
documented `Expression::at` index for a foreign id. `ControlWalk` restores
the scope in `exit`, as FR-257 Behavior 3 asks, and keeps an
`Option<ScopeId>` parent pointer instead of a scope length (same
behaviour). The `Renaming` walk's binder order is correct, because
`quire_walk` enters children in push order. No new fixed depth cap was added
(grep for `MAX_*DEPTH` and `depth >=` in added lines). No compatibility
layer, re-export or shim was added.

## Verdict

Changes requested. FND-001 is high: a source-reachable process abort at
default limits that this PR introduces. Not mergeable until it is fixed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | A deep `Option<…>` (or `Sequence<…>`) type now aborts the process with a stack overflow in S3, at default limits. With the S1 nesting ceiling gone, S1 admits about 33,000 levels at default token and node limits, and S2 builds them iteratively. Then `PackageDeclarations::assemble` and the signature check call `resolve_form`, which recurses once per type level, and build a `ValueType` whose derived traits also recurse. S3's `CheckingLimits` depth guards only the expression walk. Measured: `type T = Option<…Integer…>;` through `spine::compile` at `SpineLimits::default()` aborts at depth 10,000 (release, 8 MiB thread), 1,500 (debug, 8 MiB) and 300 (debug, 2 MiB). origin/main refuses all of these at S1. The fuzz harness doc at qsl-bench/src/deep_input.rs:181-183 says S3's depth cap gives the outcome, and for `OptionType` that is false. Fix: convert `resolve_form` to `quire_walk`, and land B3 (FR-262, iterative `ValueType` traits) first or fold it in. The other option is to charge type-form depth against the existing `CheckingLimits` depth before `resolve_form` runs, until B2/B3 delete that limit. Test idea: on a 2 MiB thread, compile `DeepInput::new(Shape::OptionType(d))` with `SpineLimits::default()`, where `d` is the deepest nest S1 admits at default limits (bisect as `deepest_options` does). Assert that it returns `Ok` or a `CompileRefusal`. Today the test process aborts. | qsl-semantics/src/check/type_form.rs:258; qsl-semantics/src/check/type_form.rs:302-323; qsl-semantics/src/check/assemble.rs:1463-1547; qsl-semantics/src/check/mod.rs:587-592; qsl-cst/src/lexer.rs:199-202; qsl-bench/src/deep_input.rs:181-183 |
| FND-002 | low | `Expression::respelled` silently drops a rewrite that changes a node's children. It also drops any payload change made to that same node, and returns `Self` as if nothing happened. That hides a caller bug. Fix: return `Result<Self, ShapeChanged>` naming the node id, so a caller cannot lose a rewrite unnoticed. Or hand the closure a payload view with no child ids. Update the unit test so the `rewired` case asserts the error. | qsl-forms/src/syntax.rs:1283-1299; qsl-forms/src/syntax.rs:2679-2685 |
| FND-003 | low | `ExpressionBuilder::push` checks only that a child id is below the next index. A public caller can name the same child from two parents (a DAG) or leave orphan nodes, and `build` accepts both. Consumers that key per node index assume each node has one parent: `substitute_names`' `names`/`binders` vectors, and arena-order spans. Derived `PartialEq` also compares orphans. Fix: track claimed children in `push`, refuse a child that is already claimed, and have `build` refuse unless the last node is the only unclaimed one. | qsl-forms/src/syntax.rs:1421-1440; qsl-semantics/src/check/checked_dispatch.rs:841-885 |
| FND-004 | medium | TC-169 `every_variant_and_field_of_the_moved_types_is_unchanged` is a shape freeze. Its exhaustive matches pin every variant and field of the parsed-form types, so any change breaks the build, as this PR shows. It tests no behaviour. Its requirement, FR-067-CON-3, is a constraint on a move that finished long ago ("relocates … without changing any variant, field or method"). That is a promise that is no longer true, and it is ceremony. Fix: delete the test and FR-067-CON-3 in this PR. Drop the TC-169 binding from FR-067-AC-9, or delete AC-9 too: it scans for a duplicate definition left by the same finished move. | qsl-forms/src/syntax.rs:2284-2301; spec/functional/FR-067-*.md:201; spec/functional/FR-067-*.md:216 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | `substitute_names` now ends in `.expect("a renaming keeps every node's children")`, a new panic site on a production path in the S3 checker. The closure cannot change a child id, so the panic cannot fire today. But this module's own convention is a typed refusal "rather than an `.expect()`" (checked_dispatch.rs:228, :275), and the expect exists only because `respelled`'s one signature now serves both payload-only and shape-changing callers. Fix: add a payload-only rewrite that cannot fail, for example `Expression::respell_payloads`, whose closure receives no child ids, and use it here. Or return the `ShapeChanged` through `substitute_names`' caller as an internal fault. | qsl-semantics/src/check/checked_dispatch.rs:898-904 |

## Dispositions

Round 1, reviewed at 97bc6bfd014fe1c46cb1779b2205f10533153a2a. Checked against the fix commit 97bc6bfd's own diff; the branch was rebased onto 0578c960 (#607, B3) first.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 97bc6bfd: `resolve_form` runs on `quire_walk` (`Resolver`, type_form.rs:330-445). `encode_value_type` (family.rs:281), `is_package_independent` (family.rs:789) and `holds_references` (document.rs:1272) loop down the option/collection chain, and the encoding order is unchanged (bounds are written innermost first after the leaf, as the old recursion did). There is no cap. B3's iterative `ValueType` `Clone`/`PartialEq`/`Hash`/`Drop`/`Debug` is on the branch (0578c960 is an ancestor). The reproducing test `the_deepest_admitted_option_type_compiles_or_refuses_on_a_2_mib_stack`, re-run here on a 2 MiB debug thread, passes (1 passed, 16.6 s); the same configuration aborted at depth 300 before the fix. The deep_input.rs doc no longer claims S3's depth cap bounds a type. |
| FND-002 | fixed | 97bc6bfd: `Expression::respelled` returns `Result<Self, ShapeChanged>` (syntax.rs:1323-1340), and the unit test asserts `Err(ShapeChanged(tree.root_id()))` for the rewired tree. |
| FND-003 | fixed | 97bc6bfd: `ExpressionBuilder` tracks claimed children. `push` refuses `UnknownChild`, `ClaimedTwice` (another parent, or the same child twice in one node), and `build` refuses `Unclaimed` orphans and `Empty` (`TreeRefusal`, syntax.rs:1469-1555). `graft` marks grafted interior nodes claimed. `a_builder_builds_only_one_tree` covers both refusals. |
| FND-004 | fixed | 97bc6bfd: the TC-169 test, the TC-169 file and its spec/tests.md row, FR-067-CON-3 and FR-067-AC-9 are deleted, and so are the qsl-eval `value_expression_syntax_is_absent_from_the_module_tree` test and its doctest. FR-068's three citations are reworded, and a grep at the head finds no remaining reference. |

Round 2, reviewed at 734c512bc6d71a2ef893d8b460c0f5b604c857ae (fix commit 734c512b, on main 3cb91b40).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-005 | fixed | 734c512b: `Expression::respell_names(&self, FnMut(ExprId, Spellings<'_>)) -> Self` (syntax.rs:1447) hands the closure only the node id and `Spellings { reference: Option<&mut String>, binders: Vec<&mut String> }` (syntax.rs:1562), built by `ExprNode::spellings_mut` (an exhaustive match, :832). No child id is in reach, so the function cannot fail. `substitute_names` (checked_dispatch.rs:839-863) calls it and has no `.expect`. `binder_names` is deleted, and the `Bind` step reads `ExprNode::binders` (:788, exhaustive). The binder order is written twice, in `binders` for reading and in `spellings_mut` for writing, so `binders` is not literally the single source. The two agree, and an Accumulate swap would fail the existing `rename_parameters_reads_each_operand_in_its_binders_scope` (fold accumulator `y`, binder `x`). New test: `respell_names_rewrites_references_and_binders`. It passes in ~/dev/worktrees/logs/b1-make-ci-r4.log (make ci exit=0). |

Round 3, reviewed at 846c7fa810c5a4113435650221ae0fd60410783b (fix commit 846c7fa8, rebased onto main c54e3595; same change as e9369474 on origin/task/482-arena-forms).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-006 | fixed | 846c7fa8: the two lines "The rename walk, with nothing to rename, rewrites every form into exactly itself: each operand, binder and attribute in its own place." are back above `an_empty_rename_keeps_every_form`'s doc (checked_dispatch.rs:1752-1755), and the four lines match its doc on the pre-review base 3ef889a4. `the_rename_stack_grows_within_the_node_charge` keeps only its own three-line doc (:1731-1733). |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | low | Round 1 inserted `the_rename_stack_grows_within_the_node_charge` inside `an_empty_rename_keeps_every_form`'s doc comment. The stack test now opens with that test's first two lines ("The rename walk, with nothing to rename, rewrites every form into exactly itself: each operand, binder and attribute in its own place."), and `an_empty_rename_keeps_every_form` lost them, so each test's doc misdescribes it. Fix: move those two lines back above `an_empty_rename_keeps_every_form`'s remaining doc. | qsl-semantics/src/check/checked_dispatch.rs:1731-1738; qsl-semantics/src/check/checked_dispatch.rs:1755-1758 |

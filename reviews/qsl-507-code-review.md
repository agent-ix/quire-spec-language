---
id: SR-2449
title: "Code review of quire-spec-language PR #669: T1 temporal forms with an optional interval (QSL-507)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@282388e134159b7c4ee59feab69ccb4f10839048; PR #669 diff against origin/main (merge base b8981dc2): qsl-cst/src/cst.rs, qsl-cst/src/grammar.rs, qsl-cst/src/parser.rs, qsl-cst/src/token.rs, qsl-cst/tests/it/complete_grammar.rs, qsl-forms/src/dispatch.rs, qsl-forms/src/lib.rs, qsl-forms/src/syntax.rs, qsl-forms/src/temporal_clause.rs, qsl-forms/src/value.rs, qsl-forms/tests/it/main.rs, qsl-forms/tests/it/protocol_clause_forms.rs, qsl-forms/tests/it/temporal_clause_forms.rs, qsl-forms/tests/it/value_forms.rs, qsl-semantics/src/check/assemble.rs, qsl-semantics/src/check/check.rs, tests/fixtures/parser-differential/baseline.txt; context: tests/it/compile_command.rs, tests/it/parser_differential.rs, qsl-semantics/src/check/refusal.rs, qsl-semantics/src/check/protocol_clause.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-325
    type: reviews
---
# Code review of quire-spec-language PR #669

## Summary

Ticket: QSL-507. PR: quire-spec-language#669, head 282388e1, one review pass.
The Rust lane (rust-review) is folded into this file. Reviewer-only: no
builds were run (the build lock is busy). Each finding below was traced by
reading the code, not by running it.

What the PR does:

- **qsl-cst.** An operator's interval is now optional on all eight temporal
  operators. A new `Terminal::Bound` accepts only an integer token that
  parses as `u64`, so a bound that is too large or not a number fails at S1,
  at the bound. A new `Fairness` production
  (`fair [weak|strong] [whole|each] QualifiedName ;`) sits before a clause's
  captures. `fair`, `weak`, `strong` and `whole` are reserved in the complete
  edition.
- **qsl-forms.** Adds `LeadingTokenKind::Temporal`,
  `DeclarationForm::Temporal(Box<TemporalClauseForm>)` and the
  `temporal_clause` production. The formula is a flat arena built with an
  explicit work stack. An interval is built as written, so an inverted one
  gets no diagnostic.
- **qsl-semantics.** The exhaustive `DeclarationForm` match in the assembler
  gets a `Temporal` arm. It stores the clauses in a new pub field,
  `PackageDeclarations::temporal_clauses`. The PR body calls this
  `AssembledPackage::temporal_clauses`, but no type of that name exists.

**Rust lane: clean.** There is no `unsafe`, `unwrap`, `expect` or panic in
the non-test code. The formula build never recurses. Each work-stack entry is
one CST child, so the stack grows by at most one entry per CST node that S1
already charged. The derived `Clone`, `PartialEq` and `Debug` on
`TemporalFormulaForm` walk a flat `Vec` and never recurse. `ConditionForm`
reuses `Expression`, which already has non-recursive impls. The `u64` bound is
checked at S1, and the re-parse in `interval_form` is an invariant check that
cannot fail. `has_token` reads only a node's own tokens, so a qualified name
with a segment called `each` cannot be read as a granularity.

**Formula mapping, checked by hand.** Operand order holds for `implies`
(right-associative), for the left-associative `and`/`or` chain (links added
root first, operands queued so the leftmost one is mapped first), and for
unary and binary operators with and without an interval. Each operator's
`span` and `operator_span` are its own.

**Reserved words and the bound.** QSpec FR-362 (qspec-main-ro fbd6ea0f) gives
the same `fairness` EBNF and says that `fair`, `weak`, `strong` and `whole`
are reserved words of the temporal facet. Reserving them across the whole
complete-edition vocabulary matches how this codebase already reserves the
words of other facets (protocol, hybrid, synthesis). `Terminal::MemberName`
still accepts a reserved word after `::`. No complete-edition source in this
repo uses one of the four words as an identifier. Neither do the five
consumer repos (see the public API section below).

**The `unexpected` arm at value.rs:1006.** The ticket was written against
9647e5a8. At that commit, line 1006 of qsl-forms/src/value.rs is
`| Production::TemporalClause`, inside the catch-all arm of the `Value`
expression mapping (`Mapping::map`). That arm returns `Err(unexpected(node))`
for every production that is not an expression. The mapping cannot meet a
temporal production at run time, because those productions now have their
own builder. The arm exists only because the codebase enumerates every
production instead of using a wildcard. Deleting it would mean adding a `_`
wildcard, which loses exhaustiveness, or adding a production classifier,
which is beyond this ticket. What temporal clauses actually hit before this PR
was the S2 `NoDispatchEntry` refusal. `from_spelling` now claims `temporal`,
so that path is gone. So keeping the arm, with `Production::Fairness` added
for exhaustiveness, is right. The ticket's "Deletes in the same PR" line is
mis-targeted and should be corrected on the ticket. It is not a code defect.

**Parser-differential baseline.** Only `complete` buckets changed: 18 of 100.
No `historical` or `composed` bucket changed. That is expected, because
those two families render through the native parser, which this PR does not
touch. The complete-V1 rendering includes every diagnostic message. So
besides "generated units that omit an interval now parse", the change of
`Terminal` description from `unsigned integer` to `unsigned 64-bit integer`
also changes the rendering of mutated inputs with a bad bound. So may the new
`fair` alternative at a clause's `{`. See FND-004.

**Public API diff vs origin/main.**

- qsl-cst: new variant `Production::Fairness` (exhaustive enum), and new
  vocabulary token kinds `fair`, `weak`, `strong` and `whole`.
  `Terminal::Bound` is `pub(super)`.
- qsl-forms: new variants `LeadingTokenKind::Temporal` and
  `DeclarationForm::Temporal`. New pub types: `TemporalOperator`,
  `IntervalUpper`, `IntervalForm`, `TemporalNodeId`, `ConditionForm`,
  `TemporalOperatorForm`, `TemporalNodeForm`, `TemporalFormulaForm`,
  `ParameterForm`, `ActivationForm`, `FairnessKind`, `FairnessGranularity`,
  `FairnessConstraintForm` and `TemporalClauseForm`. All are re-exported
  from the crate root.
- qsl-semantics: new pub field `PackageDeclarations::temporal_clauses`.

**Consumer grep.** Fresh `git fetch`, then
`git grep origin/main -- '*.rs'` for `qsl_cst::`, `qsl_forms::`,
`use qsl_cst`, `use qsl_forms`, `PackageDeclarations {`, every new or changed
type name, `Production::` and `TokenKind::`. The SHAs grepped were:

- quire-integration 80431da1
- quire-driver 5cddd5f6
- quire-contract-codegen e5cbe075
- quire-contract-ir 87737121
- quire-protocol 8b1dc4ef

The only QSL use is in quire-integration `tests/qsl_model_owner_admission.rs`,
which calls `qsl_cst::parse`, `qsl_forms::build_unit` and
`PackageDeclarations::assemble`. None of those matches on a changed enum or
builds the struct literally. The `FairnessKind`/`FairnessGranularity` in
quire-driver and the `DeclarationForm` in quire-contract-ir are those repos'
own types. No consumer breaks.

## Verdict

Changes requested. Two high findings:

- **FND-001.** A unit with a temporal clause used to be refused. S3 now
  accepts it without checking it, and the clause is silently left out of the
  emitted package.
- **FND-002.** For the same reason, the TC-435 stage-refusal test in
  `tests/it/compile_command.rs` can no longer pass, so main would go red.

The S1/S2 work itself is correct and well built. The FR-325-AC-1 to AC-3
tests have strong oracles: exact intervals, exact spans recovered from the
source text, and the diagnostic at the bound's span with no form built.

The AC-4 test pads the model version so the two headers have the same byte
length. It then compares the clause forms with both `PartialEq` and `Debug`.
S2 has no profile input, so today the test can fail only if spans drift. It
still guards against a future S2 that reads the profile. It would also not
pass vacuously, because it asserts the fairness, activation and interval
content of the forms it compares.

Fairness forms are inside FR-325's own "Outputs" ("its fairness constraint
forms (FR-123)"), so building them is not "beyond FR-325". FR-325 just has no
AC for them (see SR-2450). `IntervalForm.span` is also required by FR-325
"Outputs" ("the source span of the operator and of its interval") and by AC-1
("each with its own span"), so it is not an addition.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | A unit with a `temporal` clause used to be refused (S2 `NoDispatchEntry`, `unsupported_construct`). It is now accepted by S3 without any check. The `Temporal` arm stores the clause in `PackageDeclarations::temporal_clauses`, which nothing reads, so the unit compiles and the emitted package silently lacks the property. This repeats the defect class that `ProtocolAnchorCause::Unimplemented` exists to prevent ("Refused rather than compiled with unchecked content and silently missing from the emitted package", SR-753 FND-002, SR-761 FND-001, SR-770 FND-001). Until QSL-508 (T2) lands the FR-326/FR-123 check, S3 should refuse each temporal clause with `UnsupportedConstruct` / `not-yet-implemented` at the clause span, and a test should pin that. QSL-508 then deletes the refusal. | qsl-semantics/src/check/assemble.rs:564, qsl-semantics/src/check/assemble.rs:1739, qsl-semantics/src/check/check.rs:250-252 |
| FND-002 | high | TC-435 (FR-027-AC-8) `each_spine_stage_refusal_reports_its_stage_and_code` asserts that `temporal Due using v over (view: M::Node) clock "c" on origin { always[0,1] holds(true) }` refuses at stage `forms` with `unsupported_construct`, exit 21. S2 now builds this clause, so the refusal cannot happen at `forms`, and the PR leaves the test unchanged. The author ran filtered root `it` tests. Fix: move the `forms`-stage case to a spelling that no family claims (for example a `synthesis` declaration, as `value_forms.rs` did for FR-091-AC-6). With FND-001 fixed, a temporal case can also be added as a `check`-stage refusal. | tests/it/compile_command.rs:572-580 |
| FND-003 | medium | `temporal_clause` refuses any clause that has a `capture` with `UnrepresentedConstruct { production: Capture }`. That construct is valid under QSpec FR-362's grammar and is in qsl-cst's own accepted corpus. No test pins the refusal, its cause or its span, so a later change could start dropping captures silently and stay green. Fix: add a test in the FR-091-AC-7 style asserting the cause and the capture's span. | qsl-forms/src/temporal_clause.rs:34-35 |
| FND-004 | low | The parser-differential baseline was re-recorded as 18 opaque bucket hashes. Nobody can confirm from the diff that only intended outcomes moved. The PR body names one cause (an omitted interval now parses), but the changed bound description (`unsigned integer` to `unsigned 64-bit integer`) and the new `fair` alternative also change complete-V1 renderings. Value test: the baseline's stated purpose, proving the explicit-stack rewrite equivalent, is already met. Every grammar change re-records it, and re-recording passes any change. So it is now a resync pin that guards nothing, and per the no-ceremony rule it should be deleted rather than re-recorded. That is the lead's call on scope. | tests/fixtures/parser-differential/baseline.txt, tests/it/parser_differential.rs:13-21 |

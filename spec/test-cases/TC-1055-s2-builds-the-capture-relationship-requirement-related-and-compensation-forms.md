---
id: TC-1055
title: "S2 builds the capture, relationship, requirement, related-by and compensation forms with their spans"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-308
    type: verifies
---
# TC-1055: S2 builds the capture, relationship, requirement, related-by and compensation forms with their spans

## Description

Verify that S2 builds one typed form for each `capture`, `relationship`,
`requires temporal`, `related by` and compensation member a protocol writes,
in source order, with every part and span, resolves nothing, and refuses a
recovering CST with no partial form. Scope: FR-308-AC-7 to FR-308-AC-12.

Catches a capture built into the wrong stage field, a repeated relationship or
requirement merged into one form, a `related by` end dropped or reordered, a
`commit never` built as an empty node reference, a zero count or inverted
interval refused at S2 instead of S3, and a partial form built beside a
recovery node.

## Test Procedure

Build the S2 forms of one protocol per row with profile alias `P`, reading the forms and the span of each part. Assert the form fields, never a
rendered message.

| # | Input | Expected |
| --- | --- | --- |
| 1 | a protocol writing `capture order: M::OrderRef = opened.order;` before `role Service on M::Node;` | `captures` holds one `CaptureForm`: name `order`, type `M::OrderRef`, value `opened.order`, span covering the member with its `;` |
| 2 | a compensation writing `capture target: M::Total = forward.total;` then `capture floor: M::Total = 0;` before `activate first (trigger: M::Plain) when { trigger.ready } { capture activated: Boolean = trigger.ready; }` | `registration_captures` holds `target` then `floor`; `activation_captures` holds `activated`; the protocol's `captures` is empty |
| 3 | `relationship Pipe = M::pipe;` then `relationship Drain = M::drain;` | `relationships` holds `Pipe` with target `M::pipe`, then `Drain` with target `M::drain`, each span slicing to its text |
| 4 | `requires temporal Due; requires temporal Late; requires temporal Due;` | `requirements` holds three forms, `Due`, `Late`, `Due`, in that order; none is resolved or merged |
| 5 | `effect Applied of Main::Tried as (applied: M::Node) related by Pays (applied.order, opened.order) { true };` | the node's `related` holds one form: relationship `Pays`, two ends `applied.order` and `opened.order` in written order, span covering the clause |
| 6 | `attempt Tried by Service on M::Node::step contracts [] as (attempted: M::Plain) related by Pays (a, b) related by Pays (c, d) related by Owns (e, f) { true };` | `related` holds three forms in source order: `Pays` with ends `a`, `b`; `Pays` with `c`, `d`; `Owns` with `e`, `f` |
| 7 | a `related by Pays (f(applied), opened.order)` whose first end is a call expression | the first end is the call's expression form, whole; S2 reads no function |
| 8 | a compensation with `within [0,30]; attempts 3 of M::Attempt; retry (earlier: M::Attempt, later: M::Attempt) { true }; commit Main::Committed; recover (recovered: M::Receipt) { true };` | every member present with its span: `within` `[0,30]`, `attempts` count 3 and record `M::Attempt`, `retry` binders `earlier`, `later` and relation, `commit` `Node(Main::Committed)`, `recover` binder `recovered` and predicate |
| 9 | row 8 with `commit never;` | `commit` is `Never`, with the span of `never`; no node reference is built |
| 10 | row 8 with `attempts 0` and with `within [31,30]` | forms built with count 0 and interval `[31,30]` as written; no S2 diagnostic |
| 11 | row 8 with `attempts 18446744073709551616` | S1 parse diagnostic at the count; no form |
| 12 | five protocols, each with a recovery node inside a `capture`, a `relationship`, a `requires temporal`, a `related by` and a compensation member | each refuses `FormsCause::RecoveringCst`; no partial form |
| 13 | a protocol writing one of every member above, built twice | equal forms |

## Expected Results

Every member a protocol writes reaches the checker as its own typed form, in the
field of its position and in source order, with its spans. A repeated name is
kept as written. A zero count, an inverted interval and a name that resolves to
nothing build at S2 and are refused by S3. A recovering CST yields no form.

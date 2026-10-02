---
id: TC-901
title: "The quire fences of spec artifacts are checked against the objects they declare"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-355
    type: verifies
---
# TC-901: The quire fences of spec artifacts are checked against the objects they declare

## Description

Verify that `check_fences` checks every `quire` fence of an artifact
inventory as a state invariant of the object its artifact declares, reports
each refusal at the artifact's path, line and column, keeps checking after a
refusal, ignores fences of other languages, and is deterministic and
cancellable.

Scope: FR-355-AC-1 to FR-355-AC-6.

## Test Procedure

Fixture: an `entity` artifact `account.md`, id `account`, typed by the
entity module's manifest, whose Properties table declares
`account_id: Integer 1..1` (identity), `verified: Boolean 1..1`,
`suspended: Boolean 1..1`, and `first_order: Integer 1..1` and
`suspended_at: Integer 1..1`, each with presence `optional`, with the two
clauses of FR-355-AC-1 under its Invariants section. Every field type is
one FR-056 admits, so the inventory passes intake. Every check uses the
alias `Accounts`.

1. Check the fixture inventory.
2. Add the clauses `present(self.verified)`, `self.account_id`,
   `self.nickname` and `self.suspended and`, each under its own clause
   heading, and check.
3. Add to step 1's inventory an `enumeration` artifact `tier.md`, id
   `tier`, typed by the enumeration module's manifest, with the values
   `Standard` and `Gold`, and to `account` the clause
   `self.verified implies Accounts::tier::Gold = Accounts::tier::Gold`;
   check.
4. Write the `present(self.verified)` fence of step 2 with an indented
   first line holding only `true and` and the expression on its second
   line; check it with CRLF line endings, then with LF line endings.
5. Check an artifact whose only fence is labelled `text`; then an artifact
   with a `quire` fence whose frontmatter names the object type `gadget`,
   which no supplied manifest declares.
6. Check step 2's inventory twice; then check it with a cancelled `Cancel`.

Tag the tests `#[trace("TC-901", "FR-355-AC-n")]`.

## Expected Results

- Step 1: two `FenceResult`s, each with no diagnostic.
- Step 2: six `FenceResult`s in document order. The first two are empty.
  `present(self.verified)` has `ill_typed`/`type-mismatch`;
  `self.account_id` has `ill_typed`/`non-boolean-root`; `self.nickname` has
  `missing_declaration`/`missing-name`; `self.suspended and` has
  `invalid_syntax`/`unexpected-end`. Each refusal names `account.md` and
  the line inside its fence.
- Step 3: intake admits `tier`; the new clause's `FenceResult` has
  `unsupported_construct`/`declaration-form` at `Accounts::tier`, and the
  first two `FenceResult`s are empty.
- Step 4: in both encodings, the refusal names the artifact line of the
  fence's second body line and the column of `present`.
- Step 5: no `FenceResult` for the `text` fence; one `FenceResult` with
  `missing_declaration`/`missing-name` naming the second artifact's id.
- Step 6: equal reports; then `StageFailure::Cancelled` and no report.

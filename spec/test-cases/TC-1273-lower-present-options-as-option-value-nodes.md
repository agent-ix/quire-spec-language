---
id: TC-1273
title: "Lower a checked present option as an identity-bound option value"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-140
    type: verifies
---

# TC-1273: Lower a checked present option as an identity-bound option value

## Description

Verify QSL's S4 lowering and QSL I2 read-back for the present option expression
in FR-140-AC-11.

## Test Procedure

Compile the optional-field refinement row `head = if self.size >= 1 then
Some(self.head) else none` with `head: Int[0, 1]?`. In the emitted checked
package, find the node produced by the checked `Some(self.head)` expression.
Confirm it is one `value`/`option_value`, its checked type is `Option<Int[0,
1]>`, and its payload reference names the emitted node for `self.head`. Confirm
the option value's current key recomputes from the QSpec FR-322 application-node
preimage and its dependencies contain the referenced payload. Confirm there is
no `quire.op.option.some`, no application for `Some`, and no function-call node
for `Some`. Read the emitted package through QSL I2 and confirm the refinement
is admitted. The `none` branch remains the context-typed option-none literal.

Tag the test `#[trace("TC-1273", "FR-093-AC-48", "FR-140-AC-11")]`.

## Expected Results

Exactly one present option value node represents the checked `Some` expression;
its type, payload reference, key and dependency follow QSpec FR-322, and QSL I2
admits the package. No application or function-call node represents `Some`.

---
id: TC-1043
title: "Receiver-scoped field entries and scoped deletes appear in the frame node wire and decide the frame"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-154
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-115
    type: verifies
---
# TC-1043: Receiver-scoped field entries and scoped deletes appear in the frame node wire and decide the frame

## Description

Verify the frame node body S4 emits for scoped `modifies` and `deletes` entries
(QSpec FR-340's `scope` and `deletes_scoped`), the intake refusals, the I2
reader's frame step, and the decision `check_frame` and the `Frame` run take
from the scopes.

Scope: FR-154-AC-6 to FR-154-AC-10 and FR-115-AC-11.

## Test Procedure

Fixture: FR-120's `test/counters` package, selected as `Counters`, extended
with: field `step: Int[0, 2]` on `Counter`; object type `Sub` specializing
`Counter`; object type `Gauge` with field `d`; relationship `Supersedes`
between `Counter` objects; and the operations below. Each operation is also
named by its own postcondition clause (`DoneBump`, `DoneMerge`, `DoneNoop`,
`DoneOwn`, `DoneAll`, `DoneDrop`, `DoneDispose`, each `{ true }`) so that the
package holds its frame node. Node identity strings are written with the prefix
`ix://test/counters/` as `Counter/value`, `Counter/step`, `Gauge/d`, `Counter`,
`Gauge` and `Supersedes`. In the wire vectors `<Counter>` stands for the NodeRef
object `{"digest":"<digest of Counter>","domain":"quire.checked-semantic-node/v1"}`
of the `Counter` object type node, and `<Gauge>` for `Gauge`'s. Every frame
body is written with its members in canonical JCS order. Tag the tests
`#[trace("TC-1043", "FR-154-AC-n")]` and `#[trace("TC-1043", "FR-115-AC-11")]`.

Operations and their model-document `frame` (FR-154 "Inputs"):

| Operation | Parameters | `frame` |
| --- | --- | --- |
| `bump` | none | `{"modifies":["Counter/value"],"modifies_receiver":["Counter/step"],"deletes_receiver":true}` |
| `merge` | `a: Reference<Counter>`, `b: Reference<Counter>` | `{"deletes":["Counter"],"deletes_parameters":["b","a"],"deletes_receiver":true}` |
| `noop` | none | `{}` |
| `own` | none | `{"modifies_receiver":["Counter/value"]}` |
| `all` | none | `{"modifies":["Counter/value"]}` |
| `drop` | none | `{"deletes_receiver":true}` |
| `dispose` | none | `{"deletes":["Gauge"],"deletes_receiver":true}` |

Meaning in surface text: `bump` is `modifies value`, `modifies self.step`,
`deletes self`; `merge` is `deletes b`, `deletes a`, `deletes self`,
`deletes Counter`; `own` is `modifies self.value`; `drop` is `deletes self`.

1. Emit the unit and read each operation's `state`/`frame` node body.
2. Compare the node ids of `own` and `all`, and of `drop` and `noop`. Emit
   package B1, the fixture without `all`, and package B2, B1 with `own`'s
   frame replaced by `{"modifies":["Counter/value"]}`; compare their
   `package_id`s. (B2 holds no second operation with an equal frame.)
3. Intake the package once per row of this table, each row replacing the
   `frame` of an operation `t(n: Integer, a: Reference<Counter>)` of
   `Counter` (of `Sub` in I7):

   | Row | `frame` |
   | --- | --- |
   | I1 | `{"modifies_receiver":["Counter/nope"]}` |
   | I2 | `{"modifies_receiver":["Gauge/d"]}` |
   | I3 | `{"modifies_receiver":["Supersedes"]}` |
   | I4 | `{"deletes_parameters":["missing"]}` |
   | I5 | `{"deletes_parameters":["n"]}` |
   | I6 | `{"deletes_parameters":["a"],"deletes_receiver":true}` |
   | I7 | `{"modifies_receiver":["Counter/value"]}` |
   | I8 | `{"modifies":["Counter/value"],"modifies_receiver":["Counter/nope","Counter/value"]}` |
   | I9 | `{"modifies":["Counter/value"],"modifies_receiver":["Counter/value"]}` |
   | I10 | `{"deletes_receiver":"yes"}` |
   | I11 | `{"modifies_self":[]}` |

4. For each of X1 to X8, copy the emitted package, edit the named frame node
   body, and re-key it: compute the edited frame node's key, replace the key
   in the `state`/`operation_anchor` node body that references it, re-key that
   anchor and every node that depends on the anchor, and recompute the
   `package_id`. Without the re-keying, FR-322's node-key check refuses the
   copy `invalid_package`/`stale-node-key` (FR-320-AC-4) before the frame step
   runs. Then read the copy with the I2 reader. X1 is W1 with the `step`
   entry's `scope` removed; X2 is W1 with `scope` `"all"`; X3 is W1 without
   `deletes_scoped`; X4 is W2 with `deletes_scoped` `[parameter a, receiver,
   parameter b]`; X5 is W2 with `deletes_scoped` `[receiver, parameter b,
   parameter a]`; X6 is W2 with a `parameter` entry `c` added last, `merge`
   having no parameter `c`; X7 is W1.
5. `check_frame` over state `w` holding `c1`, `c2` and `c3` of `Counter`, all
   `value` 0 and `step` 0, with these effects, receivers and posts:

   | Row | Effect | Receiver, arguments | Post |
   | --- | --- | --- | --- |
   | D1 | `modifies self.value`, `modifies step` | `c1` | `c1.value` 1, `c2.step` 1 |
   | D2 | as D1 | `c1` | `c2.value` 1 |
   | D3 | `deletes self`, `deletes a` | `c1`, `a` = `c2` | without `c1` and `c2` |
   | D4 | as D3 | `c1`, `a` = `c2` | without `c3` |
   | D5 | `deletes a` | `c1`, `a` = `c2` | without `c1` |

6. `Frame` runs over invocations with `self` `c1` and pre snapshot `w`:
   F1 runs `own` with a post that sets `c2.value` to 1; F2 runs `all` with
   the same post; F3 runs `dispose` with a post without `c2` and `deleted:
   [c2]`. Each invocation declares `created: []` and the deletion its post
   makes.

## Expected Results

Wire vectors (W1 to W6), each a full frame node `body`:

- W1 (`bump`): `{"creates":[],"deletes":[],"deletes_scoped":[{"kind":"receiver"}],"modifies":[{"declaration":<Counter>,"kind":"field","name":"step","scope":"receiver"},{"declaration":<Counter>,"kind":"field","name":"value","scope":"every-object"}],"term":"frame"}`. One declaration, so the entries order by name: `step` before `value`.
- W2 (`merge`): `{"creates":[],"deletes":[<Counter>],"deletes_scoped":[{"kind":"receiver"},{"kind":"parameter","name":"a"},{"kind":"parameter","name":"b"}],"modifies":[],"term":"frame"}`. The authored order `b`, `a`, `self` is not the emitted order.
- W3 (`noop`): `{"creates":[],"deletes":[],"deletes_scoped":[],"modifies":[],"term":"frame"}`.
- W4 (`own`): `{"creates":[],"deletes":[],"deletes_scoped":[],"modifies":[{"declaration":<Counter>,"kind":"field","name":"value","scope":"receiver"}],"term":"frame"}`.
- W5 (`all`): W4 with `"scope":"every-object"`.
- W6 (`drop`): `{"creates":[],"deletes":[],"deletes_scoped":[{"kind":"receiver"}],"modifies":[],"term":"frame"}`.

Results:

- Step 1: the bodies of `bump`, `merge`, `noop`, `own`, `all` and `drop` are
  W1 to W6 byte for byte. The `dependencies` of `bump`'s and `merge`'s frame
  nodes contain `<Counter>`'s key, the node their entries name, and the scoped
  entries add no other.
- Step 2: `own` and `all` have different node ids (W4 and W5 differ in one
  `scope`); `drop` and `noop` have different node ids (W6 and W3 differ only in
  `deletes_scoped`); B1 and B2 have different `package_id`s.
- Step 3: I1 refuses `missing_declaration`/`missing-name` at the entry. I2
  and I3 refuse `invalid_model_binding`/`malformed-declaration` at the entry
  (`d` is a field of `Gauge`, to which `Counter` does not conform;
  `Supersedes` is a relationship). I4 and I5 refuse `missing_declaration`/
  `missing-name` at the entry (`missing` is no parameter; `n` is an
  `Integer`). I6 and I7 admit. I8 refuses only `missing_declaration`/
  `missing-name` at `Counter/nope`; it does not report the conflict on
  `Counter/value`, because the conflict is checked once every entry resolves.
  I9 refuses `invalid_model_binding`/`conflicting-binding` naming both
  entries and `t`. I10 refuses `invalid_model_binding`/`malformed-declaration`
  at `deletes_receiver` and I11 at `modifies_self`, before any entry is read.
- Step 4: X1, X2 and X3 are refused as schema failures of the frame body. X4
  and X5 refuse `invalid_semantic_graph`. X6 refuses `missing_declaration`/
  `missing-name`. X7 is admitted.
- Step 5: D1 returns a delta with no creation or deletion (`c1.value` is
  granted by the receiver entry, `c2.step` by the every-object entry). D2 is
  `frame_violation`/`unauthorized-change` naming `c2` and `value`. D3 returns
  a delta whose deleted keys are `c1` and `c2` (`c1` by `deletes self`, `c2`
  by `deletes a`). D4 is `frame_violation`/`unauthorized-change` naming `c3`.
  D5 is `frame_violation`/`unauthorized-change` naming `c1`.
- Step 6: F1 reports `violation` with the `FieldWrite` change naming `c2`,
  `value` and `modifies` holding the key of `value`, cause
  `frame_violation`/`unauthorized-change`; F2 reports `success`. F3 reports
  `violation` with the `Deleted` change naming `c2`, type `Counter` and
  `deletes` holding the key of `Gauge` only, because `deletes self` grants
  `c1` alone and names no declaration.

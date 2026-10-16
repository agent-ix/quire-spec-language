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
with: field `step` on `Counter`; object type `Sub` specializing `Counter`;
object type `Gauge` with field `d`; relationship `Supersedes` between
`Counter` objects; and the operations below, each also named by a
postcondition clause `Done { true }` so that the package holds its frame node.
`<Counter>` and `<Gauge>` stand for those object type nodes' keys. Every frame
body is written with its members in canonical JCS order. Tag the tests
`#[trace("TC-1043", "FR-154-AC-n")]` and `#[trace("TC-1043", "FR-115-AC-11")]`.

Operations and frames:

| Operation | Parameters | Frame |
| --- | --- | --- |
| `bump` | none | `modifies self.step`, `modifies value`, `deletes self` |
| `merge` | `a: Reference<Counter>`, `b: Reference<Counter>` | `deletes b`, `deletes a`, `deletes self`, `deletes Counter` |
| `noop` | none | no entry |
| `own` | none | `modifies self.value` |
| `all` | none | `modifies value` |
| `drop` | none | `deletes self` |
| `dispose` | none | `deletes self`, `deletes Gauge` |

1. Emit the unit and read each operation's `state`/`frame` node body.
2. Compare the node ids of `own` and `all`, and of `drop` and `noop`, and
   the `package_id` of the package with that of the same package whose `own`
   frame is `modifies value`.
3. Intake the package once per row of this table, each row replacing the
   frame of an operation `t(n: Integer, a: Reference<Counter>)` (an operation
   of `Counter` unless the row says `Sub`):

   | Row | Frame |
   | --- | --- |
   | I1 | `modifies self.nope` |
   | I2 | `modifies self.d` |
   | I3 | `modifies self.Supersedes` |
   | I4 | `deletes missing` |
   | I5 | `deletes n` |
   | I6 | `deletes a`, `deletes self` |
   | I7 | operation of `Sub`: `modifies self.value` |
   | I8 | `modifies self.nope`, `modifies self.value`, `modifies value` |

4. Read each of these frame node bodies with the I2 reader:
   X1 is W1 with the `step` entry's `scope` removed; X2 is W1 with `scope`
   `"all"`; X3 is W1 without `deletes_scoped`; X4 is W2 with `deletes_scoped`
   `[parameter a, receiver, parameter b]`; X5 is W2 with `deletes_scoped`
   `[receiver, parameter b, parameter a]`; X6 is W2 with a `parameter` entry
   `c` where `merge` has no parameter `c`; X7 is W4 followed by a second `value`
   entry with scope `every-object`; X8 is W1.
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

- W1 (`bump`): `{"creates":[],"deletes":[],"deletes_scoped":[{"kind":"receiver"}],"modifies":[{"declaration":"<Counter>","kind":"field","name":"step","scope":"receiver"},{"declaration":"<Counter>","kind":"field","name":"value","scope":"every-object"}],"term":"frame"}`. One declaration, so the entries order by name: `step` before `value`.
- W2 (`merge`): `{"creates":[],"deletes":["<Counter>"],"deletes_scoped":[{"kind":"receiver"},{"kind":"parameter","name":"a"},{"kind":"parameter","name":"b"}],"modifies":[],"term":"frame"}`. The authored order `b`, `a`, `self` is not the emitted order.
- W3 (`noop`): `{"creates":[],"deletes":[],"deletes_scoped":[],"modifies":[],"term":"frame"}`.
- W4 (`own`): `{"creates":[],"deletes":[],"deletes_scoped":[],"modifies":[{"declaration":"<Counter>","kind":"field","name":"value","scope":"receiver"}],"term":"frame"}`.
- W5 (`all`): W4 with `"scope":"every-object"`.
- W6 (`drop`): `{"creates":[],"deletes":[],"deletes_scoped":[{"kind":"receiver"}],"modifies":[],"term":"frame"}`.

Results:

- Step 1: the bodies of `bump`, `merge`, `noop`, `own`, `all` and `drop` are
  W1 to W6 byte for byte. The `dependencies` of `bump`'s and `merge`'s frame
  nodes contain `<Counter>`, the node their entries name, and the scoped
  entries add no other.
- Step 2: `own` and `all` have different node ids (W4 and W5 differ in one
  `scope`); `drop` and `noop` have different node ids (W6 and W3 differ only in
  `deletes_scoped`); the package with `own`'s frame changed to W5 has a
  different `package_id`.
- Step 3: I1 refuses `missing_declaration`/`missing-name` at the entry. I2
  and I3 refuse `invalid_model_binding`/`malformed-declaration` at the entry
  (`d` is a field of `Gauge`, to which `Counter` does not conform;
  `Supersedes` is a relationship). I4 and I5 refuse `missing_declaration`/
  `missing-name` at the entry (`missing` is no parameter; `n` is an
  `Integer`). I6 and I7 admit. I8 refuses only `missing_declaration`/
  `missing-name` at `modifies self.nope`; it does not report the conflict
  between `self.value` and `value`, because the conflict is checked once every
  entry resolves.
- Step 4: X1 and X2 are refused as schema failures of the frame body, and so
  is X3. X4 and X5 refuse `invalid_semantic_graph`. X6 refuses
  `missing_declaration`/`missing-name`. X7 refuses `invalid_model_binding`/
  `conflicting-binding`. X8 is admitted.
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

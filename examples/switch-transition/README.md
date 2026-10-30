# Switch transition example

The authored Boolean Switch pair: a domain model with a `Switch` object whose
Boolean field `on` is modified by the operations `set(flag)` and `adjust(flag)`,
and complete-V1 sources over them. `switch.native` has one `post` clause over
`set`; the producer derives one `state`/`transition` node for the Boolean model
field (FR-105-AC-13). `adjust.native` has a precondition `P1` and a
postcondition `Q2` over `adjust` (FR-058-AC-1). All files are QSL-owned and
AGPL-3.0-only; a reader copies nothing, it passes these paths.

| File | Role |
| --- | --- |
| `switch.native` | the QSL source (edition `1-draft`); its `model` line names the package by identity alone |
| `adjust.native` | the QSL source with `pre P1` and `post Q2` over `adjust` |
| `switch.semantic-ir.json` | the domain model package document the sources select by identity |

Emit the `quire.checked-package/v2` bytes from the repository root:

```sh
cargo run --locked --example emit_checked_v2 -- \
  --package-name switch \
  --model examples/switch-transition/switch.semantic-ir.json \
  examples/switch-transition/switch.native
```

The emitted bytes embed the path of the source exactly as given, so a caller
that needs stable bytes passes the canonical repository-relative path above
from the repository root every time.

Tests (`tests/it/emit_checked_v2.rs`) emit both sources against the model
file and read the packages back.

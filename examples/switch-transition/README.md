# Switch transition example

The authored Boolean Switch pair: a domain model with a `Switch` object whose
Boolean field `on` is modified by the operation `set(flag)`, and a complete-V1
source with one `post` clause over `set`. The producer derives one
`state`/`transition` node for the Boolean model field (FR-105-AC-13). Both
files are QSL-owned and AGPL-3.0-only; a reader copies nothing, it passes
these paths.

| File | Role |
| --- | --- |
| `switch.native` | the QSL source (edition `1-draft`) |
| `switch.semantic-ir.json` | the domain model package document it selects by digest |

Emit the `quire.checked-package/v2` bytes from the repository root:

```sh
cargo run --locked --example emit_checked_v2 -- \
  --model examples/switch-transition/switch.semantic-ir.json \
  examples/switch-transition/switch.native
```

The emitted bytes embed the source path exactly as given, so a caller that
needs stable bytes passes the canonical repository-relative paths above from
the repository root every time. From another checkout directory, run with that
directory as the working directory.

The source names the model by its `sha256-jcs` digest (FR-056 selects models
by digest), and the
`the_switch_example_pair_emits_a_transition_and_is_admitted` test in
`tests/it/emit_checked_v2.rs` checks that digest against the model file; the
digest is over canonical JSON, so reformatting the model is harmless, and a
real model edit fails the test with the computed digest to paste into
`switch.native`. That embedded digest is a known frozen-digest antipattern
(follow-up in the STD-565 family). The test derives every other expectation
from the producer and from Contract IR's strict `CheckedPackageV2::read`.

# Quire Specification Language

[![Discord](https://img.shields.io/badge/Discord-Join%20us-5865F2?logo=discord&logoColor=white)](https://discord.gg/k8DVhuYBR2)

QSL (Quire Specification Language) is a formal language for rules about a
system's data and behavior. This repository contains its compiler, `quire-spec`,
and its reference evaluator.

The compiler checks a rule against a domain model and writes a checked package.
The evaluator runs a checked rule against data and returns one of five
outcomes: holds, violated, refused, unsupported or incomplete. Proof and
code-generation backends read the same checked package.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/images/hero-dark.svg">
  <img alt="A rule written in QSL, either in a .native file or in a fenced block inside a Markdown spec, is compiled against a domain model into a checked package, then evaluated against data. The result is one of: holds, violated, refused, unsupported or incomplete." src="docs/images/hero-light.svg">
</picture>

> **Status: public prerelease.** The language, its editions and every API here
> are still changing. No crate is published yet. Expect breaking changes.

## A first rule

This unit states three rules about the `ConfigVersion` objects of the domain
model `example/config-version`:

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/images/first-rule-dark.svg">
  <img alt="An annotated QSL unit: (1) the header names the language and edition, (2) the profile says which part of the language the rules use, (3) the model line imports a domain package by exact identity and digest, (4) invariants must hold for every object, (5) a post rule compares a value after an operation with its value before." src="docs/images/first-rule-light.svg">
</picture>

```
language "ix:native" edition "1-draft";
profile v = "quire.value.complete/v1";
model Config = "example/config-version" version "1.0.0" digest "sha256-jcs:…";

invariant ParentOrder using v on Config::ConfigVersion at current {
  present(self.parent) implies deref(value(self.parent)).versionNumber < self.versionNumber
}

invariant NoCycle using v on Config::ConfigVersion at current {
  not reaches(self, self, parent)
}

post VersionUnchanged using v on Config::ConfigVersion::attemptUpdate {
  self.versionNumber = pre(self.versionNumber)
}
```

- **`language … edition`** sets the language version of the file.
- **`profile`** selects the part of the language the rules may use.
  `quire.value.complete/v1` covers values, records and expressions. The line
  binds it to the name `v`, and each rule says `using v`.
- **`model`** imports the domain model by identity, version and content digest.
  A rule is always checked against that exact model.
- **`invariant … at current`** must hold for every `ConfigVersion` in the data.
  `ParentOrder`: a parent, when present, has a lower version number.
  `self.parent` is an optional reference; `present` tests it, `value` unwraps
  it and `deref` follows it. `NoCycle`: following `parent` links never returns
  to the start.
- **`post … on …::attemptUpdate`** is checked around an operation. `pre(x)` is
  the value of `x` before the operation. `VersionUnchanged`: an update attempt
  keeps the version number.

### Rules in Markdown specs

A rule can also sit in an `ix:native` fenced block in a Quire Markdown spec,
under the prose it formalizes:

````markdown
### ParentOrder

A version's parent, when it has one, always has a lower version number.

```ix:native
invariant ParentOrder using v on Config::ConfigVersion at current {
  present(self.parent) implies deref(value(self.parent)).versionNumber < self.versionNumber
}
```
````

The optional `quire-extraction` feature reads the block's bytes from the
Markdown file, using [quire-rs](https://github.com/agent-ix/quire-rs), and keeps
a source map to the document. Errors are reported at their line and column in
the Markdown file. This is a library API; the `run` command does not yet accept
Markdown for `1-draft` rules.

## Outcomes

Every check ends in one outcome, and the process exit code matches it:

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/images/outcomes-dark.svg">
  <img alt="Every QSL check ends in exactly one outcome with its own exit code: success 0; violation or undefined 10; refused 20; unsupported 21; incomplete or inconclusive 22; internal failure 30. Only success and violation are answers about the rule; the others say why no answer was given." src="docs/images/outcomes-light.svg">
</picture>

| Outcome | Exit | Meaning |
| --- | --- | --- |
| success | 0 | The check completed and the rule holds. |
| violation, undefined | 10 | The check completed and the rule is broken, or the rule asked for a value that is not defined. |
| refused | 20 | The input was rejected before any rule ran: bad syntax, a wrong model digest, a malformed request. |
| unsupported | 21 | The construct is valid QSL, but the selected profile or backend cannot check it. |
| incomplete, inconclusive | 22 | Data was missing or the work budget ran out. |
| internal failure | 30 | A bug in QSL. |

Only success and violation carry a truth value. Refused and incomplete results
have no truth field in their output. Every result names the source, model and
input it was computed from.

## Language

Each unit declares its profiles. Each profile enables one family of rules. All
of the families below are in the `1-draft` edition. In the excerpts, `G`, `T`
and `P` are profiles and `M` is a model, declared at the top of the unit as in
the first example.

**Values and functions.** Bounded integers, exact rationals, records, unions
and pure functions. There is no floating point.

```
type Digit = Int[0, 9];
record Point { x: Digit; y: Digit; }
function px using v(p: Point): Digit pure { if p.x > p.y then p.x else p.y }
```

**State.** Invariants over the current objects, `pre`/`post` rules around
operations, and sums and sizes over collections.

```
invariant Healthy using G on M::Workflow at current {
    size<M::Tally>(self.receipts) <= 5
    and sum<M::Total>(receipt in self.receipts: receipt) <= self.total
}
```

**Time.** Temporal rules over a sequence of events, with explicit clocks and
bounded windows. This one says the ratio stays allowed for the 30 events after
the origin:

```
temporal Due using T over (view: M::Workflow) clock "events" on origin {
    always[0,30] holds(Allowed(view.ratio))
}
```

**Protocols.** Choreographies between services: roles, channels with ordering
and delivery bounds, sequences, choices, parallel branches, retries, timeouts
and compensation. An excerpt:

```
protocol Orders using P over (view: M::WorkflowView) on each (started: M::StartView) when (started.ready) {
  role OrdersService on M::OrderService;
  role PaymentsService on M::PaymentService;
  channel Requests from OrdersService to PaymentsService carries M::ChargeRequest
    ordering fifo by (message: M::ChargeRequest) { message.key } delivery [0,3];
  run sequence Main {
    send Sent via Requests as (sent: M::SendView) { sent.amount = requested };
    receive Received via Requests of Sent as (received: M::ReceiptView) { received.ok };
    commit Committed by OrdersService as (committed: M::CommitView) { committed.ok };
  }
}
```

The full example, with compensation, retries and timeouts, is
[`tests/fixtures/composed-choreography.native`](tests/fixtures/composed-choreography.native).

| Profile | Family |
| --- | --- |
| `quire.value.complete/v1` | values, records, functions, predicates |
| `quire.state.graph/v1` | invariants, `pre`/`post`, reachability over object graphs |
| `quire.temporal.event-position.false-extension/v1` | temporal rules over event positions |
| `quire.temporal.timestamped-event.finite-window/v1` | temporal rules over timestamped events in a finite window |
| `quire.temporal.fixed-sample.false-extension/v1` | temporal rules over fixed-rate samples |
| `quire.temporal.infinite-trace/v1` | temporal rules over unbounded traces (model checking) |
| `quire.protocol.finite-global/v1` | protocols between roles |

The `1-draft` language is a closed list of capabilities in nine families. QSL
checks every package against that list.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/images/capabilities-dark.svg">
  <img alt="The complete 1-draft language is a closed list of capabilities in nine families: source, types, expressions, temporal, protocol, runtime, backend, evidence and tooling." src="docs/images/capabilities-light.svg">
</picture>

**Editions.** This README covers the `1-draft` edition, the complete version-1
language. The older `0-draft` edition remains for historical fixtures and is
being removed. Do not write new rules in it.

## Compiler stages

QSL compiles in stages. Each stage consumes the previous stage's output type
and produces a new one. Unchecked and checked rules are different Rust types,
so unchecked input cannot reach an evaluator or backend.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/images/pipeline-dark.svg">
  <img alt="QSL compiles source through a lossless syntax tree and parsed forms into a checked graph, resolving names against domain models and dependency packages, then links a checked package. The package goes three ways: to the reference evaluator, to native model checking, or as checked-package/v2 bytes to Contract IR and proof backends. A counterexample from any engine is replayed through the same compiler and evaluator to confirm it." src="docs/images/pipeline-light.svg">
</picture>

| Stage | Guarantees |
| --- | --- |
| **Source** | Exact bytes with an identity: authority, id and revision. Markdown fences enter here. |
| **Syntax tree** | A byte-exact tree of the source, including comments and spacing. The formatter works from it. A file that parses is not yet checked. |
| **Parsed forms** | One unchecked form per declaration, each with its source span. |
| **Checked graph** | Every name is resolved against the imported domain model and dependency packages. Every expression is typed. Every partial operation, such as a division or a dereference, is proved defined or becomes an explicit runtime obligation. |
| **Checked package** | The checked rules and their dependencies' checked rules, with a package identity and a source map. It can be written as `quire.checked-package/v2` bytes. |
| **Reference evaluation** | Deterministic, side-effect-free execution against supplied data. Ends in one of the outcomes above. |
| **Model checking** | In-process engines for temporal and protocol rules: model checking, timed-zone search, schedulability and probabilistic analysis. |
| **Proof backends** | [Contract IR](https://github.com/agent-ix/quire-contract-ir) reads the v2 bytes. [Contract codegen](https://github.com/agent-ix/quire-contract-codegen) generates code, property tests and bounded proofs from it. |
| **Replay** | A counterexample from any engine is recompiled from its source and re-run on the reference evaluator. A backend's "violated" result is accepted only if the reference evaluator returns the same outcome. |

## Usage

Install Rust. [`rust-toolchain.toml`](rust-toolchain.toml) pins the toolchain,
and `rustup` installs it automatically. No other tools or services are needed.

```sh
git clone https://github.com/agent-ix/quire-spec-language
cd quire-spec-language
cargo build --locked --bin quire-spec
```

**Format.** `format` prints the formatted source and leaves the file unchanged.
The four arguments before the path are the source identity (authority, id,
revision namespace, revision). Every diagnostic carries them.

```sh
target/debug/quire-spec format agent-ix test:spine fixture fixture:1 tests/fixtures/spine-compile.native
```

**Run a rule against data.** The ConfigVersion example writes one directory per
case, holding the unit from [A first rule](#a-first-rule), its domain model and
a snapshot of `ConfigVersion` objects. `requests.sh` writes a `run.json` request
that checks one rule for the object keyed `child`:

```sh
cargo run --locked --example config_version_fixtures -- /tmp/config-version
for case in healthy-parent violating-parent dangling-parent incomplete-population; do
  examples/config-version/requests.sh /tmp/config-version/$case ParentOrder
  target/debug/quire-spec run /tmp/config-version/$case/run.json; echo "exit $?"
done
```

| Case | Data | Outcome | Exit |
| --- | --- | --- | --- |
| `healthy-parent` | parent version 1, child version 2 | the rule holds | 0 |
| `violating-parent` | parent version 3, child version 2 | the rule is broken | 10 |
| `dangling-parent` | the parent is not in the snapshot | refused | 20 |
| `incomplete-population` | the snapshot's population is not complete | incomplete | 22 |

`requests.sh /tmp/config-version/cycle NoCycle` checks the other invariant; the
`cycle` case breaks it (exit 10). Each result is one JSON document on stdout
that names its source, model and snapshot.

**Compile a package.** `requests.sh` also writes `compile.json`. `compile`
prints the checked package as `quire.checked-package/v2` JSON:

```sh
target/debug/quire-spec compile /tmp/config-version/healthy-parent/compile.json > package.json
```

The [ConfigVersion example](examples/config-version/README.md) lists all
seventeen cases: parents, cycles, object identity, operation frames, and refused
and incomplete runs. `run` can also evaluate one pure function;
[`tests/it/spine_run.rs`](tests/it/spine_run.rs) shows that request.

**Optional features.**

| Feature | Adds |
| --- | --- |
| `quire-extraction` | Reads rules from Quire Markdown spec artifacts, with locations mapped to the document. |
| `handoff-writer` | Writes the compiled-protocol handoff files that protocol tools consume. |
| `population-fixtures` | Writes the population fixtures used by the admission tests. |

`quire-spec` is the compiler's own command line. The planned main entry point
is `quire` from quire-driver (private), which runs QSL, Contract IR and the
backends as one tool.

## The Quire ecosystem

The standard defines the language. QSL compiles rules into checked packages.
Contract IR turns them into contracts for code generation, the runtime and the
analyzers. The driver runs the chain. All of them use a set of shared `no_std` libraries.


<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/images/ecosystem-dark.svg">
  <img alt="The Quire repositories. The main chain runs from the standard, quire-specification, to the compiler, quire-spec-language, then quire-contract-ir, quire-contract-codegen and quire-contract-runtime. quire-spec-language also feeds quire-protocol, and quire-contract-ir feeds quire-analyze. quire-observation feeds quire-protocol and quire-mltl. Above the chain, quire-driver runs it, quire-integration tests it and quire-verification plans verification. Below it, shared libraries used throughout: quire-exact, quire-semantic-value, quire-walk, quire-canonical and quire-verification-contracts." src="docs/images/ecosystem-light.svg">
</picture>

| Repository | Role | Relation to QSL |
| --- | --- | --- |
| quire-specification (private) | The language standard: rule semantics, profiles, interchange formats and the conformance tests. | QSL implements it; `make ci` checks QSL against its test vectors. |
| [quire-exact](https://github.com/agent-ix/quire-exact) | Exact numbers and closed outcomes; `no_std`. | QSL depends on it. |
| [quire-semantic-value](https://github.com/agent-ix/quire-semantic-value) | Semantic values with their origin, built on quire-exact; `no_std`. | QSL depends on it. |
| [quire-walk](https://github.com/agent-ix/quire-walk) | Walks deep trees on a heap stack; `no_std`. | QSL depends on it. |
| [quire-canonical](https://github.com/agent-ix/quire-canonical) | Canonical JSON (RFC 8785), hashed while it is written. | QSL depends on it. |
| [quire-contract-ir](https://github.com/agent-ix/quire-contract-ir) | The target-neutral contract model that backends use. | QSL depends on its model crate; IR reads QSL's checked packages. |
| [quire-contract-codegen](https://github.com/agent-ix/quire-contract-codegen) | Generates Rust, property tests, bounded proofs and evidence from contracts. | Depends on QSL to replay counterexamples. |
| [quire-contract-runtime](https://github.com/agent-ix/quire-contract-runtime) | The `no_std` runtime for generated checks. | Used by codegen's output. |
| [quire-analyze](https://github.com/agent-ix/quire-analyze) | SMT-based consistency and implication checks across contracts. | Works on Contract IR. |
| [quire-verification-contracts](https://github.com/agent-ix/quire-verification-contracts) | The verification record types the tools exchange. | Used by IR, codegen and the standard. |
| [quire-observation](https://github.com/agent-ix/quire-observation) | Turns raw signals into qualified observations for rule evaluation. | Supplies data for temporal checks. |
| [quire-mltl](https://github.com/agent-ix/quire-mltl) | Connects the MLTL temporal-logic engine to quire-observation. | No direct dependency. |
| quire-protocol (private) | Checks running systems against compiled protocols, including compensation. | Consumes QSL's compiled protocols. |
| quire-verification (private) | Selects verification techniques and plans for each requirement. | No direct dependency. |
| quire-driver (private) | The `quire` command line that runs the whole chain. | Depends on QSL. |
| quire-integration (private) | Pins a working set of versions and tests the chain together. | Depends on QSL. |

## Crates

The compiler is a Cargo workspace in strict layers. A crate depends only on
layers below it; `make arch-lint` enforces this.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/images/layers-dark.svg">
  <img alt="The QSL workspace in layers, bottom to top: the shared no_std libraries quire-exact, quire-semantic-value, quire-walk and quire-canonical; qsl-foundation; qsl-cst; qsl-forms; qsl-semantics; qsl-package; then qsl-eval and qsl-route; and at the top qsl-replay, qsl-analyze and the quire-spec command line." src="docs/images/layers-light.svg">
</picture>

| Crate | Contents |
| --- | --- |
| `qsl-foundation` | Source identity, source maps, diagnostics and exit codes, wire-format names. |
| `qsl-cst` | Lexer, grammar, the lossless syntax tree and the formatter. |
| `qsl-forms` | Parsed, unchecked forms for each declaration family. |
| `qsl-semantics` | Model intake, name resolution, type checking and definedness proofs. The largest crate. |
| `qsl-package` | Checked packages and the `quire.checked-package/v2` writer and reader. |
| `qsl-eval` | The reference evaluator, including temporal automata and finite simulation. |
| `qsl-route` | Matches a package's requirements to the backends that can check it. |
| `qsl-analyze` | In-process model checking, timed zones and schedulability. |
| `qsl-replay` | Recompiles and re-runs counterexamples; codegen calls it. |
| `qsl-source` | Reads rules from Quire Markdown (feature `quire-extraction`). |
| `qsl-string-edge` | The FR-064 string-edge scanner (library and `qsl-string-edge` command) that QSL's `xtask string-edge` and consumer repositories run; see [`qsl-string-edge/README.md`](qsl-string-edge/README.md). |
| `qsl-attrs`, `qsl-walk-grow`, `qsl-bench` | A lint marker attribute, deep-recursion stack growth, benchmarks. |
| root crate | The `quire-spec` binary and the library facade. |
| `xtask`, `tools/arch-lint` | Repository checks: dependency direction, API surface, conformance. |

Other directories:

- [`spec/`](spec/): requirements. Each requirement, acceptance criterion and
  test case is a Markdown file. Tests are tagged with the criteria they check.
- [`spec/decisions/`](spec/decisions/): architecture decisions, including the
  stage design.
- [`docs/`](docs/): reference pages, including the
  [library notes](docs/library-notes.md),
  [diagnostic codes](docs/native-error-codes.md),
  [run and compile requests](docs/native-standalone.md) and the
  [compiled protocol wire format](docs/compiled-protocol-v2.md).

## Contributing

`make ci` is the local gate: formatting, clippy, tests across feature
combinations, docs and the architecture lints. It reads the standard's test
vectors from a sibling `quire-specification` checkout (`QSPEC_DIR`). Hosted CI
runs only on manual dispatch, so run the gate before opening a pull request.
See [CONTRIBUTING.md](CONTRIBUTING.md) and [AGENTS.md](AGENTS.md).

Contributions require the [CLA](CLA.md).

## License

QSL is licensed under [AGPL-3.0-or-later](LICENSE). Fixtures keep their
`AGPL-3.0-only` headers. See [LICENSE-DECISION.md](LICENSE-DECISION.md).

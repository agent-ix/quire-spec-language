<!-- reviewer repo=agent-ix/quire-spec-language visibility=private quoin=0.29.4 module=spec-artifacts-process@v0.29.2 id=SR-2438001 method=code-review lang=rust pr=quire-spec-language#2438 model=claude-sonnet-5-5 run=rev2438-r1 -->

Verdict: mergeable after one small fix round (head f0e8aee49). Cargo.toml untouched, lock moves only quire-contract-model on main; helpers take `&mut NativeJoinBudget`; V2ReadLimits.storage required, no Default; with_supply fallible, joins once, witness defects deferred; row-based identity flow intact. Green: fmt, clippy x2, arch-lint x2, qsl-semantics, qsl-eval it (1208), qsl-replay lib, qsl-analyze, root checked-handoff it, qsl-package integration. qsl-package lib 4 failures: TM-26 (known, PR 2442) plus the 3 coder-listed, all reproduced identically on a control run of origin/main 8ffcf0959 (main fails 20 there), so not regressions. Mutations (denial mapped to Unknown; constructor ignoring denial) each fail 3 tests.

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | medium | soundness | Denial never names i2.work_units / i2.native_join_storage in production; native_budget_setting is test-only |
| FND-002 | medium | coverage | AC-24 one-short/exact controls missing: writer uses 0, view storage-only, from_read storage-only, from_emission work-only |
| FND-003 | low | other | Blank line splits TC-1331 table before row 20 |

+++ [reviewer data]

```yaml
clean: false
scope:
  - {id: FR-531-AC-21, path: "spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md", role: examined, excerpt: "with_supply returns the package on NativeJoinBudget::new(W, S), ExhaustedWork on (W-1, S), ExhaustedStorage on (W, S-1), the work denial naming i2.work_units and the storage denial i2.native_join_storage"}
  - {id: FR-531-AC-22, path: "spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md", role: examined, excerpt: "foreign, unknown and missing witnesses stay deferred to the slot call; a budget one unit short of the unknown rows' join returns the budget denial, not UnknownIdentityWitness"}
  - {id: FR-531-AC-23, path: "spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md", role: examined, excerpt: "two joins on one budget are charged cumulatively; later selection, world and projection consume no further native work or storage"}
  - {id: FR-531-AC-24, path: "spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md", role: examined, excerpt: "V2ReadLimits has required storage and no Default; the view at exact storage succeeds and one less returns ViewFailure::Exhausted; from_read and from_emission one short return PackageMismatch::Exhausted; the writer with native_join one unit short returns Error::NativeJoinExhausted and writes no directory"}
  - {id: FR-531-AC-25, path: "spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md", role: examined, excerpt: "each entry takes a NativeJoinBudget by mutable reference; no NativeJoinBudget::new and no u64::MAX allowance in a library path below an entry; BarePackage is neither Copy nor Clone"}
  - {id: TC-1331, path: "spec/test-cases/TC-1331-evaluate-from-a-checked-package-read-from-its-bytes-alone.md", role: examined, excerpt: "Scope: FR-531-AC-1 to FR-531-AC-25"}
  - {id: FR-517, path: "spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md", role: examined, excerpt: "native_join: the finite work and logical-storage ceilings of the identity file's native joins; no default; a denied charge returns Error::NativeJoinExhausted and writes no directory"}
  - {id: FR-255, path: "spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md", role: context_only, excerpt: "i2.native_join_storage | I2 v2 reader native identity join | logical storage | V2ReadLimits.storage | 16777216 logical units"}
findings:
  - fnd: FND-001
    severity: medium
    confidence: high
    method: code-review
    check: soundness
    artifact_id: FR-531-AC-21
    related_ids: [FR-255]
    path: qsl-semantics/src/model/intake.rs
    lines: "168-178"
    excerpt: |-
      with_supply returns ... ExhaustedWork on new(W - 1, S) and ExhaustedStorage on new(W, S - 1), each with IR's locus and attempted charge, ... the work denial naming `i2.work_units` and the storage denial naming `i2.native_join_storage`
    finding: "native_budget_setting is the only code that names the i2.* setting for a denial and its sole callers are tests (tests/it/bare_package.rs). Production denials (with_supply Err, ViewFailure::Exhausted, PackageMismatch::Exhausted, Error::NativeJoinExhausted) carry only IR's ExactTypeProjectionError, so no production refusal names the setting that raises the limit (FR-255), and LimitKind::LogicalStorage / Setting::I2NativeJoinStorage have no production denial path. Either map the denial to the stage_limit_exceeded-class refusal at the entries, or restate FR-531/AC-21 to the library form that ships."
  - fnd: FND-002
    severity: medium
    confidence: high
    method: code-review
    check: coverage
    artifact_id: FR-531-AC-24
    related_ids: [TC-1331, FR-517]
    path: tests/it/bare_package.rs
    lines: "1590-1700"
    excerpt: |-
      the handoff writer over `native_join` one unit short returns `Error::NativeJoinExhausted` and writes no directory
    finding: "The writer test uses NativeJoinLimits::new(0, ..) and (.., 0), not the measured bound minus one, and never runs the exact measured bound; the view test checks only storage one short; from_read only storage; from_emission only work. The requested exact-bound and one-short controls for work AND storage are missing on these entries."
  - fnd: FND-003
    severity: low
    confidence: high
    method: code-review
    check: other
    artifact_id: TC-1331
    related_ids: [FR-531-AC-21, FR-531-AC-25]
    path: spec/test-cases/TC-1331-evaluate-from-a-checked-package-read-from-its-bytes-alone.md
    lines: "40"
    excerpt: |-
      | 19 | 20 | old format; missing required arrays; ... |
      
      | 20 | 21 | a fixture's `with_supply` on a budget measured at (W, S), ...
    finding: "A blank line between rows 19 and 20 ends the Test Procedure table, so rows 20 to 24 are not table rows."
bindings: []
```

+++


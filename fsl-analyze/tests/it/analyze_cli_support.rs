// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared harness for the `native-analyze/1` stubs (FR-620 to FR-636): writes
//! a request document and its files into a fresh directory, hands it to the
//! in-process command adapter (`fsl_command::command::analyze`) with an FSpec
//! FR-290 negotiation double standing for the driver's `negotiate_*`, and
//! returns the exit code and the FR-286 outcome document. The driver binary is not used:
//! the language repository cannot depend on the driver, and the adapter is
//! the library half every frontend calls.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::counter::{clause_text, document, unit_text, CONTRACT};

/// One command-adapter outcome: its exit code and its document bytes.
pub(crate) struct Run {
    pub(crate) exit: i32,
    pub(crate) stdout: Vec<u8>,
}

impl Run {
    /// The machine-mode outcome document.
    pub(crate) fn document(&self) -> Value {
        serde_json::from_slice(&self.stdout).unwrap_or_else(|error| {
            panic!(
                "the outcome is one JSON document ({error}); exit {}",
                self.exit
            )
        })
    }

    /// The outcome item at `index`.
    pub(crate) fn item(&self, index: usize) -> Value {
        self.document()["items"][index].clone()
    }
}

/// A fresh directory holding one request's files.
pub(crate) struct Workspace {
    dir: PathBuf,
}

impl Workspace {
    pub(crate) fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("fsl-analyze-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        Self { dir }
    }

    pub(crate) fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("the file's directory");
        }
        std::fs::write(&path, bytes).expect("the file is written");
        path
    }

    /// The path of `name` in the workspace.
    pub(crate) fn path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }
}

/// The handoff directory FR-850's `package` selection names.
pub(crate) const HANDOFF: &str = "handoff";

/// Compile the `models` and `program` of `request` (a `native-analyze/1`
/// request object over files already in `ws`, with the `libraries` its
/// program imports) into the FR-517 handoff directory [`HANDOFF`]; returns
/// `request` with `package` selecting that directory in place of `program`
/// and `libraries`, and the handoff's package bytes.
pub(crate) fn package_selection(ws: &Workspace, request: &Value) -> (Value, Vec<u8>) {
    let mut compiled = json!({
        "models": request["models"], "program": request["program"],
        "handoff": {"directory": HANDOFF, "package_name": "analyze-unit",
            "native_join": {"work_units": 1_u64 << 40, "storage": 1_u64 << 24}}});
    if let Some(libraries) = request.get("libraries") {
        compiled["libraries"] = libraries.clone();
    }
    let compile = json!({"format": "native-compile/1", "request": compiled});
    let compile_path = ws.write("compile.json", compile.to_string().as_bytes());
    let package = fsl_command::command::compile(&compile_path).expect("the program compiles");
    let written = std::fs::read(ws.path(&format!("{HANDOFF}/checked-package.json")))
        .expect("the handoff holds its package");
    assert_eq!(written, package, "the handoff holds the compiled package");
    let mut selected = request.clone();
    selected
        .as_object_mut()
        .expect("a request object")
        .remove("program");
    selected
        .as_object_mut()
        .expect("a request object")
        .remove("libraries");
    selected["package"] = json!({"directory": HANDOFF});
    (selected, package)
}

/// The FSpec FR-290 settlement the driver's `negotiate_*` gives an item
/// from its candidates, as a test double: one candidate is `supported`, none
/// `unsupported`, several `invalid-request` `ambiguous-backend`, and an
/// unregistered named backend `invalid-request` `unknown-backend`.
pub(crate) struct Fr290;

impl fsl_analyze::analyze::document::Negotiate for Fr290 {
    fn negotiate(
        &self,
        item: &fsl_analyze::analyze::ClaimItem,
        candidates: &fsl_route::CandidateOutcome,
    ) -> (
        fsl_route::routing::Disposition,
        Option<fsl_analyze::analyze::Diagnostic>,
    ) {
        use fsl_analyze::analyze::{Diagnostic, Payload};
        use fsl_route::routing::Disposition;
        use fsl_semantics::check::NegotiationOutcome;
        match candidates {
            fsl_route::CandidateOutcome::UnknownBackend(backend) => (
                Disposition::InvalidRequest,
                Some(Diagnostic {
                    code: NegotiationOutcome::UnknownBackend.code(),
                    payload: Payload::Backend(backend.clone()),
                }),
            ),
            fsl_route::CandidateOutcome::Candidates(set) => match set.candidates() {
                [candidate] => (Disposition::Supported(candidate.clone()), None),
                [] => (
                    Disposition::Unsupported,
                    Some(Diagnostic {
                        code: NegotiationOutcome::UnsupportedRequestedCapability.code(),
                        payload: Payload::Kind(item.kind),
                    }),
                ),
                _ => (
                    Disposition::InvalidRequest,
                    Some(Diagnostic::new(NegotiationOutcome::AmbiguousBackend.code())),
                ),
            },
        }
    }
}

/// Hand `request` to the command adapter and return its exit code and its
/// outcome document: the result document of an accepted request, or the
/// FR-286 `analyze` document of a refused one.
pub(crate) fn analyze(request: &Path) -> Run {
    analyze_with_providers(request, Vec::new())
}

/// [`analyze`] with `providers` registered as the caller's process providers
/// (FR-635), through `fsl_command::command::analyze_with_providers`.
pub(crate) fn analyze_with_providers(
    request: &Path,
    providers: Vec<fsl_analyze::analyze::Engine>,
) -> Run {
    analyze_with_providers_under(
        request,
        providers,
        &fsl_analyze::provider::ProviderOptions::default(),
    )
}

/// [`analyze_with_providers`] under the run's provider `options` (FR-964).
pub(crate) fn analyze_with_providers_under(
    request: &Path,
    providers: Vec<fsl_analyze::analyze::Engine>,
    options: &fsl_analyze::provider::ProviderOptions,
) -> Run {
    match fsl_command::command::analyze_with_providers(
        request,
        &Fr290,
        providers,
        options,
        &filament_exact::Cancel::new(),
    ) {
        Ok(result) => Run {
            exit: i32::from(result.exit_code),
            stdout: result.value.to_string().into_bytes(),
        },
        Err(error) => Run {
            exit: i32::from(error.category().exit_code()),
            stdout: error
                .outcome_document(fsl_replay::Operation::Analyze)
                .to_bytes()
                .expect("the outcome document serializes"),
        },
    }
}

/// An initialization snapshot `start` of the Counter fixture's counters.
pub(crate) fn counter_snapshot(objects: &[(&str, i64)]) -> Vec<u8> {
    let objects: Vec<Value> = objects
        .iter()
        .map(|(key, value)| {
            json!({"key": key, "type": "ix://test/counter/Counter",
                "fields": {"value": {"integer": value.to_string()}}})
        })
        .collect();
    json!({
        "format": "filament.state.snapshot/v1",
        "identity": {"authority": "test", "identity": "start"},
        "observation": "current", "model": {"identity": "test/counter"},
        "anchor": {"kind": "initialization", "name": "start"},
        "populations": [{"population": "ix://test/counter/counters", "complete": true,
            "objects": objects}],
    })
    .to_string()
    .into_bytes()
}

/// The `request` object of a `native-analyze/1` document over `program`,
/// the Counter domain package and the snapshot `start`, with `items`.
pub(crate) fn request_members(items: Option<Value>) -> Value {
    let mut request = json!({
        "program": {"source": {"file": "program.fsl", "authority": "test",
            "identity": "analyze-unit", "document": "AnalyzeUnit", "formal_revision": 1}},
        "models": [{"format": "semantic-ir/2.0.0", "source": {"file": "model.json",
            "authority": "test", "identity": "counter-model", "document": "Counter",
            "formal_revision": 1}}],
        "documents": [{"file": "start.json"}],
    });
    if let Some(items) = items {
        request["items"] = items;
    }
    request
}

/// Write the Counter unit with `body` (after the contract), its domain
/// package, the snapshot of counter `a` at 0, and the request whose
/// `request` object is `request`; returns the request path.
pub(crate) fn counter_request_with(ws: &Workspace, body: &str, request: Value) -> PathBuf {
    ws.write(
        "program.fsl",
        unit_text(&format!("{CONTRACT}{body}")).as_bytes(),
    );
    ws.write("model.json", &document(false));
    ws.write("start.json", &counter_snapshot(&[("a", 0)]));
    let document = json!({"format": "native-analyze/1", "request": request});
    ws.write("analyze.json", document.to_string().as_bytes())
}

/// [`counter_request_with`] over the default members and `items`.
pub(crate) fn counter_request(ws: &Workspace, body: &str, items: Value) -> PathBuf {
    counter_request_with(ws, body, request_members(Some(items)))
}

/// FR-627's staying Counter: `setTo(n: Int[0, 3])` is enabled only at
/// `value = 0` with `n = 0`, a self-loop. A behaviour may take it forever
/// while `inc` stays enabled, so only fairness on `inc` forces `value = 3`.
pub(crate) const STAY: &str =
    "pre Stay using v on M::Counter::setTo { self.value = 0 and n = 0 }\n\
     post Keeps using v on M::Counter::setTo { self.value = n }\n";

/// [`request_over`] for the staying Counter (the contract, [`STAY`] and
/// `body`) from counter `a` at 0, with `items`.
pub(crate) fn staying_counter_request(ws: &Workspace, body: &str, items: Value) -> PathBuf {
    request_over(
        ws,
        &unit_text(&format!("{CONTRACT}{STAY}{body}")),
        &crate::counter::document_with_ranged_parameter(),
        &[counter_snapshot(&[("a", 0)])],
        json!({"items": items}),
    )
}

/// The clause `name` over the Counter subject with `fairness` and `formula`.
pub(crate) fn clause(name: &str, fairness: &str, formula: &str) -> String {
    clause_text(name, fairness, formula)
}

/// FR-620's `Bounded` clause.
pub(crate) fn bounded() -> String {
    clause("Bounded", "", "always holds(c.value <= 3)")
}

/// A clause the Counter subject violates at `value = 3`.
pub(crate) fn below_three() -> String {
    clause("BelowThree", "", "always holds(c.value <= 2)")
}

/// The FSpec FR-331 `result` of an outcome item's terminal record.
pub(crate) fn label(item: &Value) -> &str {
    item["record"]["result"]
        .as_str()
        .unwrap_or_else(|| panic!("an item with a terminal record: {item}"))
}

/// Write `unit`, the domain package `model`, each snapshot of `snapshots`
/// and a `native-analyze/1` request whose `request` object is the default
/// members plus `extra`; returns the request path.
pub(crate) fn request_over(
    ws: &Workspace,
    unit: &str,
    model: &[u8],
    snapshots: &[Vec<u8>],
    extra: Value,
) -> PathBuf {
    ws.write("program.fsl", unit.as_bytes());
    ws.write("model.json", model);
    let mut request = request_members(None);
    let documents: Vec<Value> = snapshots
        .iter()
        .enumerate()
        .map(|(n, bytes)| {
            let name = format!("snapshot-{n}.json");
            ws.write(&name, bytes);
            json!({"file": name})
        })
        .collect();
    request["documents"] = Value::Array(documents);
    for (name, value) in extra.as_object().expect("extra request members") {
        request[name] = value.clone();
    }
    let document = json!({"format": "native-analyze/1", "request": request});
    ws.write("analyze.json", document.to_string().as_bytes())
}

/// [`request_over`] for the litmus domain (cells `x`, `y` and `core`) and
/// the unit with `body`.
pub(crate) fn litmus_request(ws: &Workspace, body: &str, extra: Value) -> PathBuf {
    let snapshots: Vec<Vec<u8>> = crate::litmus::Documents::new()
        .snapshots
        .into_values()
        .collect();
    request_over(
        ws,
        &crate::litmus::unit_text(body),
        &crate::litmus::resulting_document(),
        &snapshots,
        extra,
    )
}

/// The two-role order protocol over the litmus domain: `Buyer` sends
/// `Placed` on `Order`, `Seller` receives it as `Got` (unless `receive` is
/// false) and chooses `Ship` or `Cancel`.
pub(crate) fn order_protocol(receive: bool) -> String {
    let got = if receive {
        "receive Got via Order of Placed as (got: Ack) { true };\n"
    } else {
        ""
    };
    let choice = if receive {
        "choice Decide by Seller visible (got.ok) {\n\
           case no when { not got.ok } sequence Cancel { }\n\
           case yes when { got.ok } sequence Ship { }\n\
         }"
    } else {
        ""
    };
    format!(
        "record Ack {{ ok: Boolean; }}\n\
         protocol Flow using pp over (k: M::Core) on origin {{\n\
         role Buyer on M::Cell;\nrole Seller on M::Cell;\n\
         channel Order from Buyer to Seller carries Ack ordering unordered delivery exactly-once-premise capacity 4 overflow reject;\n\
         run sequence Main {{\n\
         send Placed via Order as (placed: Ack) {{ true }};\n{got}{choice} }}\n\
         finish Done as (zz: Boolean) {{ true }};\n}}\n"
    )
}

/// [`analyze`] under the run's `replay.input_bytes` (FR-255), the
/// `RunLimits` value FR-940-AC-22 names.
pub(crate) fn analyze_under_replay_bytes(request: &Path, replay_input_bytes: u64) -> Run {
    let limits = fsl_command::command::RunLimits {
        replay_input_bytes,
        ..fsl_command::command::RunLimits::default()
    };
    match fsl_command::command::analyze_with_limits(
        request,
        limits,
        &Fr290,
        &filament_exact::Cancel::new(),
    ) {
        Ok(result) => Run {
            exit: i32::from(result.exit_code),
            stdout: result.value.to_string().into_bytes(),
        },
        Err(error) => Run {
            exit: i32::from(error.category().exit_code()),
            stdout: error
                .outcome_document(fsl_replay::Operation::Analyze)
                .to_bytes()
                .expect("the outcome document serializes"),
        },
    }
}

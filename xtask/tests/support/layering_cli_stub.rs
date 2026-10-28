// SPDX-License-Identifier: AGPL-3.0-or-later
//! Test-only child CLI responses for the `make refinement-layering` wrapper.

use std::path::Path;
use std::process;

use qsl_semantics::value::HeaderLayer;
use serde_json::{json, Value};

fn response(missing_layer: bool) -> Value {
    let results: Vec<_> = HeaderLayer::ALL
        .into_iter()
        .filter(|layer| !(missing_layer && *layer == HeaderLayer::TemporalComplete))
        .map(|layer| {
            json!({
                "kind": "layer",
                "layer": layer.identity(),
                "witnesses": [{
                    "class": "admitted",
                    "codes": [],
                    "candidates": [{
                        "backend": "test-stub",
                        "disposition": "not-negotiated"
                    }],
                    "reason": "not-negotiated",
                    "result": "unresolved (unsupported)"
                }],
                "result": "unresolved (unsupported)"
            })
        })
        .collect();
    json!({"results": results, "verdict": "unsupported", "exit": 21})
}

fn main() {
    let executable = std::env::args_os()
        .next()
        .expect("the stub has an executable path");
    let name = Path::new(&executable)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .expect("stub name is UTF-8");
    let (status, missing_layer) = match name {
        "layering-cli-stub-0" => (0, false),
        "layering-cli-stub-1" => (1, false),
        "layering-cli-stub-10" => (10, false),
        "layering-cli-stub-21-missing-layer" => (21, true),
        _ => (21, false),
    };
    println!("{}", response(missing_layer));
    process::exit(status);
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-398 step 1 (FR-091-AC-11): `qsl-forms` is ADR-011 §6.1 layer 2, whose
//! "Depends on" cell is "1, F, K". Every path in this crate that leaves the
//! crate resolves into one of its `[dependencies]`, so the manifest is the
//! whole allow-list: `qsl-cst` (1), `qsl-foundation` (F) and `quire-exact`
//! (K). The root crate depends on `qsl-forms`, so Cargo refuses the reverse
//! edge, and no path in this crate can reach `check`, `value` or `model`.

use ix_trace_rs::trace;

#[trace("FR-091-AC-11", "TC-398")]
#[test]
fn dependencies_name_only_layer_1_f_and_k() {
    let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let manifest = std::fs::read_to_string(&manifest_path).expect("qsl-forms/Cargo.toml reads");
    let manifest: toml::Table = manifest.parse().expect("qsl-forms/Cargo.toml parses");
    let dependencies: Vec<&str> = manifest
        .get("dependencies")
        .and_then(toml::Value::as_table)
        .expect("qsl-forms/Cargo.toml has a [dependencies] table")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        dependencies,
        ["qsl-cst", "qsl-foundation", "quire-exact"],
        "qsl-forms may depend only on layer 1, F and K (ADR-011 §6.1)"
    );
    for section in ["build-dependencies", "target"] {
        assert!(
            manifest.get(section).is_none(),
            "qsl-forms/Cargo.toml has a [{section}] table, outside the layer-2 allow-list"
        );
    }
}

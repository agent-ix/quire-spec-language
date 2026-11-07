// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-521 row 11 (QSL-838): the retained-path successor of each dimension of
//! the deleted compiled-protocol reader's work limits. The retired bound was a
//! limit charged by the compiled-protocol wire; the retained bound is
//! the `i2.*` ceiling of [`V2ReadLimits`] that the I2 reader charges over the
//! same package. Each ceiling is exercised at zero allowance, at the exact
//! measured usage and one unit short, and the refusal names the ceiling's
//! setting, the configured bound and the count the refused charge reaches.
//!
//! | legacy dimension | retained ceiling |
//! | --- | --- |
//! | `payload_bytes`, `output_bytes` | `i2.input_bytes` ([`V2ReadLimits::artifact_bytes`]) over the emitted canonical bytes |
//! | `entries`, `declarations`, `models`, `definitions`, `sources` | `i2.nodes` and `i2.occurrences` (every table record is a graph node or a source-map entry) |
//! | `references`, `dependencies` | `i2.edges` |
//! | `byte_work`, `content_bytes` | `i2.work_units` here and `s3.work_units` / `s3.input_bytes` at S3 |
//! | `depth` | no ceiling exists (FR-264); the 100,000-deep reads are the successor |

use super::*;

/// One ceiling: its setting, the limits with that ceiling replaced by a
/// bound, and the usage the package reaches.
struct Ceiling {
    setting: Setting,
    with: fn(V2ReadLimits, u64) -> V2ReadLimits,
    usage: u64,
    /// What the refused first charge reports: IR counts a whole collection
    /// (nodes, edges, ...) at once, and a single unit for work.
    zero_actual: u128,
    /// Whether the fixture is a valid package, so that the exact bound
    /// verifies it; otherwise the exact bound only has to pass the ceiling.
    verifies: bool,
}

fn read_under(bytes: &[u8], limits: V2ReadLimits) -> Read {
    read_v2(
        bytes,
        identity("pkg"),
        limits,
        &CheckedPackageEvidence::new(),
    )
}

/// The read is stopped by `setting` at `bound`, having reached `actual`.
fn assert_stopped(outcome: &Read, setting: Setting, bound: u64, actual: u128, label: &str) {
    let Read::Limit(limit) = outcome else {
        panic!("{label}: {setting} at {bound} must stop the read, got {outcome:?}");
    };
    assert_eq!(limit.setting(), setting, "{label}");
    assert_eq!(limit.configured_bound(), bound, "{label}");
    assert_eq!(limit.actual(), actual, "{label}");
}

fn assert_admitted(outcome: &Read, label: &str) {
    assert!(
        matches!(outcome, Read::Verified { .. }),
        "{label}: expected Verified, got {outcome:?}"
    );
}

/// Zero allowance, exact usage and one unit short of one ceiling.
fn zero_exact_one_short(bytes: &[u8], ceiling: &Ceiling) {
    let base = V2ReadLimits::fixture();
    let (setting, usage) = (ceiling.setting, ceiling.usage);
    assert!(usage >= 1, "{setting}: the fixture reaches the ceiling");
    let label = |kind: &str| format!("{setting} {kind}");
    // Zero allowance: the first charge is refused.
    assert_stopped(
        &read_under(bytes, (ceiling.with)(base, 0)),
        setting,
        0,
        ceiling.zero_actual,
        &label("zero"),
    );
    // Exact: the package is admitted with nothing to spare.
    let exact = read_under(bytes, (ceiling.with)(base, usage));
    if ceiling.verifies {
        assert_admitted(&exact, &label("exact"));
    } else {
        assert!(
            !matches!(exact, Read::Limit(_)),
            "{setting} exact: the ceiling must not stop the read, got {exact:?}"
        );
    }
    // One short: refused at the charge that reaches the usage.
    assert_stopped(
        &read_under(bytes, (ceiling.with)(base, usage - 1)),
        setting,
        usage - 1,
        u128::from(usage),
        &label("one short"),
    );
}

/// FR-096-AC-10, FR-255-AC-1, FR-255-AC-4, FR-521-AC-5 (TC-1316): the node,
/// edge, occurrence and work ceilings of the I2 reader each admit exactly
/// their measured usage and refuse zero and one short, with the setting that
/// raises them. These are the successors of the legacy `entries`,
/// `declarations`, `models`, `definitions`, `sources`, `references`,
/// `dependencies` and `byte_work` dimensions.
#[trace("TC-1316", "FR-521-AC-5", "FR-096-AC-10", "FR-255-AC-1")]
#[trace("FR-255-AC-4")]
#[test]
fn each_counted_i2_ceiling_admits_its_exact_usage_and_refuses_zero_and_one_short() {
    let lock = base_lock(vec![]);
    let (_, three_nodes) = envelope_declaring(&[("pkg::A", "A"), ("pkg::B", "B"), ("pkg::C", "C")]);
    let mut edged = valid_envelope(&lock);
    let node_id = edged["semantic_graph"]["nodes"][0]["node_id"].clone();
    edged["semantic_graph"]["nodes"][0]["dependencies"] = json!([node_id.clone(), node_id]);
    let mut diagnosed = valid_envelope(&lock);
    diagnosed["diagnostics"]["entries"] = json!([
        {"stage": "type_checking", "code": "invalid_syntax", "cause_tag": "malformed-json", "details": [], "loci": []},
        {"stage": "type_checking", "code": "invalid_syntax", "cause_tag": "malformed-json", "details": [], "loci": []},
    ]);

    let nodes = Ceiling {
        setting: Setting::I2Nodes,
        with: |limits, bound| limits.with_nodes(bound),
        usage: 3,
        zero_actual: 3,
        verifies: true,
    };
    let edges = Ceiling {
        setting: Setting::I2Edges,
        with: |limits, bound| limits.with_edges(bound),
        usage: 2,
        zero_actual: 2,
        verifies: false,
    };
    let occurrences = Ceiling {
        setting: Setting::I2Occurrences,
        with: |limits, bound| limits.with_occurrences(bound),
        usage: 6,
        zero_actual: 1,
        verifies: true,
    };
    let diagnostics = Ceiling {
        setting: Setting::I2Diagnostics,
        with: |limits, bound| limits.with_diagnostics(bound),
        usage: 2,
        zero_actual: 2,
        verifies: false,
    };
    for (envelope, ceiling) in [
        (&three_nodes, &nodes),
        (&edged, &edges),
        (&three_nodes, &occurrences),
        (&diagnosed, &diagnostics),
    ] {
        zero_exact_one_short(&jcs(envelope), ceiling);
    }
}

/// The measured `i2.work_units` of the one-node package, found by raising the
/// bound from zero until the read is admitted. The cost is a property of the
/// package, so the exact bound is asserted as a value.
#[trace("TC-1316", "FR-521-AC-5", "FR-096-AC-10", "FR-255-AC-1")]
#[test]
fn the_i2_work_ceiling_admits_its_measured_usage_and_refuses_zero_and_one_short() {
    let bytes = jcs(&valid_envelope(&base_lock(vec![])));
    let base = V2ReadLimits::fixture();
    let measured = (0..64_u64)
        .find(|bound| {
            matches!(
                read_under(&bytes, base.with_work(*bound)),
                Read::Verified { .. }
            )
        })
        .expect("a small bound admits the one-node package");
    assert_eq!(measured, 2, "the one-node package costs two work units");
    zero_exact_one_short(
        &bytes,
        &Ceiling {
            setting: Setting::I2WorkUnits,
            with: |limits, bound| limits.with_work(bound),
            usage: measured,
            zero_actual: 1,
            verifies: true,
        },
    );
}

/// FR-096-AC-10, FR-255-AC-1, FR-513-AC-10, FR-521-AC-5 (TC-1316): the
/// artifact byte ceiling `i2.input_bytes` is the successor of the legacy
/// `payload_bytes` and `output_bytes` dimensions. The canonical bytes admit at
/// exactly their length, one byte short refuses with the offered length, and
/// a zero allowance refuses any non-empty offer; none carries a locus.
#[trace("TC-1316", "FR-521-AC-5", "FR-096-AC-10", "FR-255-AC-1")]
#[trace("FR-513-AC-10")]
#[test]
fn the_input_byte_ceiling_admits_the_exact_length_and_refuses_zero_and_one_short() {
    let bytes = jcs(&valid_envelope(&base_lock(vec![])));
    let length = bytes.len();
    let base = V2ReadLimits::fixture();
    let at = |bound: usize| read_under(&bytes, base.with_artifact_bytes(bound));
    assert_admitted(&at(length), "i2.input_bytes exact");
    for bound in [0, length - 1] {
        let outcome = at(bound);
        assert_stopped(
            &outcome,
            Setting::I2InputBytes,
            u64::try_from(bound).unwrap(),
            u128::try_from(length).unwrap(),
            "i2.input_bytes below the length",
        );
        let Read::Limit(limit) = outcome else {
            unreachable!("asserted stopped")
        };
        assert_eq!(limit.locus(), None);
        assert_eq!(limit.kind(), LimitKind::InputBytes);
    }
}

/// A wire whose graph holds `count` nodes of the given tag and form, each
/// owned by its own lock source when `own_sources` is set, and each depending
/// on the previous node when `chain` is set. The inventory the fixture
/// varies is the only thing that grows with `count`.
fn inventory_envelope(
    count: usize,
    tag: &str,
    form: &str,
    own_sources: bool,
    chain: bool,
) -> Value {
    let (_, mut envelope) = envelope_declaring(
        &(0..count)
            .map(|index| (format!("pkg::N{index:03}"), format!("N{index:03}")))
            .collect::<Vec<_>>()
            .iter()
            .map(|(label, export)| (label.as_str(), export.as_str()))
            .collect::<Vec<_>>(),
    );
    let ids: Vec<Value> = envelope["semantic_graph"]["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .map(|node| node["node_id"].clone())
        .collect();
    for (index, node) in envelope["semantic_graph"]["nodes"]
        .as_array_mut()
        .expect("nodes")
        .iter_mut()
        .enumerate()
    {
        node["node_tag"] = json!(tag);
        node["semantic_form"] = json!(form);
        if own_sources {
            node["owner"]["identity"] = json!(format!("src{index}"));
        }
        if chain && index > 0 {
            node["dependencies"] = json!([ids[index - 1].clone()]);
        }
    }
    if own_sources {
        envelope["lock"]["sources"] = (0..count)
            .map(|index| source_ref(&format!("src{index}")))
            .collect();
        for (index, entry) in envelope["source_map"]
            .as_array_mut()
            .expect("source map")
            .iter_mut()
            .enumerate()
        {
            entry["regions"][0]["source"] = source_ref(&format!("src{index}"));
        }
    }
    envelope
}

/// FR-521 row 11, QSL-838 ruling (TC-1316): the legacy `sources`,
/// `dependencies`, `definitions`, `models` and `declarations` inventory
/// dimensions are bounded on the retained path by the graph ceilings, because
/// each inventory entry is a graph node (with its source-map occurrence) or a
/// graph edge. Each case grows only its own inventory to `N` entries and shows
/// the ceiling that bounds it refusing at zero and one under `N`, and not
/// stopping the read at `N`. These fixtures are not otherwise valid packages
/// (the node forms are placeholders), so the exact case proves only that the
/// ceiling does not stop the read.
///
/// | legacy dimension | bounded by |
/// | --- | --- |
/// | `sources` | `i2.nodes` and `i2.occurrences`: each source owns a node and a source-map entry |
/// | `dependencies` | `i2.edges`: each dependency is a graph dependency edge |
/// | `definitions`, `models` | `i2.nodes`, tested over real emissions in `emit/tests/inventory_ceilings.rs` |
/// | `declarations` | `i2.nodes`: each declaration is a declared node |
#[trace("TC-1316", "FR-521-AC-5", "FR-096-AC-10", "FR-255-AC-1")]
#[test]
fn each_inventory_dimension_is_refused_by_its_graph_ceiling_at_zero_and_one_under() {
    const N: usize = 5;
    let nodes = |count: u64| Ceiling {
        setting: Setting::I2Nodes,
        with: |limits, bound| limits.with_nodes(bound),
        usage: count,
        zero_actual: count.into(),
        verifies: false,
    };
    let cases = [
        (
            "sources",
            inventory_envelope(N, "scalar_type", "text", true, false),
            nodes(5),
        ),
        (
            "sources (occurrences)",
            inventory_envelope(N, "scalar_type", "text", true, false),
            Ceiling {
                setting: Setting::I2Occurrences,
                with: |limits, bound| limits.with_occurrences(bound),
                usage: 10,
                zero_actual: 1,
                verifies: false,
            },
        ),
        (
            "dependencies",
            inventory_envelope(N, "scalar_type", "text", false, true),
            Ceiling {
                setting: Setting::I2Edges,
                with: |limits, bound| limits.with_edges(bound),
                usage: 4,
                zero_actual: 1,
                verifies: false,
            },
        ),
        (
            "declarations",
            inventory_envelope(N, "scalar_type", "text", false, false),
            nodes(5),
        ),
    ];
    for (dimension, envelope, ceiling) in cases {
        let count = |pointer: &str| {
            envelope
                .pointer(pointer)
                .map(|v| v.as_array().map(Vec::len))
        };
        match ceiling.setting {
            Setting::I2Nodes => {
                assert_eq!(count("/semantic_graph/nodes"), Some(Some(N)), "{dimension}")
            }
            Setting::I2Occurrences => {
                assert_eq!(count("/lock/sources"), Some(Some(N)), "{dimension}");
                assert_eq!(count("/source_map"), Some(Some(N)), "{dimension}");
            }
            _ => {}
        }
        zero_exact_one_short(&jcs(&envelope), &ceiling);
    }
}

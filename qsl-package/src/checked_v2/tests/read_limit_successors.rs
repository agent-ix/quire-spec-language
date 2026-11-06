// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-521 row 11 (QSL-838): the retained-path successor of each dimension of
//! the legacy `protocol_artifact::work::Limits`. The legacy bound is a
//! `Limits` field charged by the compiled-protocol wire; the retained bound is
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

// SPDX-License-Identifier: AGPL-3.0-or-later
//! `quire.value.text.unicode-17.0.0/v1`: the vendored Unicode artifacts are the
//! digests the definition pins, the normalization tables agree with the UCD
//! files for every scalar, and text admission passes the complete
//! `NormalizationTest.txt` corpus.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use ix_trace_rs::trace;
use quire_spec_language::value::{
    admit_text, Meter, ScalarLimits, TextPayload, TextProfile, TextType, UNICODE_TEXT_DEFINITION,
    UNICODE_VERSION,
};
use sha2::{Digest, Sha256};
use unicode_normalization::char::{canonical_combining_class, compose};
use unicode_normalization::UnicodeNormalization;

const DEFINITION: &str = "resources/complete-value/quire-specification/proposals/quire-v1/definitions/value-text-unicode-17.md";

fn unicode_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/complete-value/unicode-17.0.0")
}

fn read(name: &str) -> String {
    std::fs::read_to_string(unicode_dir().join(name)).unwrap()
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[trace("Task-048")]
#[test]
fn vendored_unicode_artifacts_match_the_definition_digests() {
    assert_eq!(UNICODE_VERSION, (17, 0, 0));
    assert_eq!(unicode_normalization::UNICODE_VERSION, (17, 0, 0));
    let definition =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(DEFINITION)).unwrap();
    assert!(definition.contains(&format!("Definition identity: `{UNICODE_TEXT_DEFINITION}`")));
    // Read the pinned digests from the definition's artifact table itself.
    let rows: BTreeMap<String, String> = definition
        .lines()
        .filter(|line| line.starts_with("| ") && line.contains("`https://www.unicode.org/"))
        .map(|line| {
            let cells: Vec<_> = line.split('|').map(str::trim).collect();
            let url = cells[2].trim_matches('`');
            let digest = cells[3].trim_matches('`');
            (
                url.rsplit('/').next().unwrap().to_owned(),
                digest.to_owned(),
            )
        })
        .collect();
    assert_eq!(
        rows.keys().map(String::as_str).collect::<Vec<_>>(),
        [
            "CompositionExclusions.txt",
            "DerivedNormalizationProps.txt",
            "NormalizationTest.txt",
            "UnicodeData.txt",
            "license.txt",
            "tr15-57.html",
        ]
    );
    let mut vendored: Vec<_> = std::fs::read_dir(unicode_dir())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    vendored.sort();
    assert_eq!(vendored, rows.keys().cloned().collect::<Vec<_>>());
    for (name, digest) in rows {
        let bytes = std::fs::read(unicode_dir().join(&name)).unwrap();
        assert_eq!(sha256_hex(&bytes), digest, "{name}");
    }
}

fn scalar(hex: &str) -> char {
    char::from_u32(u32::from_str_radix(hex, 16).unwrap()).unwrap()
}

fn scalars(field: &str) -> String {
    field.split_whitespace().map(scalar).collect()
}

fn ranges(line: &str) -> impl Iterator<Item = char> {
    let (first, last) = line.split_once("..").unwrap_or((line, line));
    (u32::from_str_radix(first, 16).unwrap()..=u32::from_str_radix(last, 16).unwrap())
        .filter_map(char::from_u32)
}

struct UnicodeData {
    combining: BTreeMap<char, u8>,
    canonical: BTreeMap<char, Vec<char>>,
    compatibility: BTreeMap<char, Vec<char>>,
}

fn unicode_data() -> UnicodeData {
    let mut data = UnicodeData {
        combining: BTreeMap::new(),
        canonical: BTreeMap::new(),
        compatibility: BTreeMap::new(),
    };
    for line in read("UnicodeData.txt").lines() {
        let fields: Vec<_> = line.split(';').collect();
        // Surrogate code points are not Unicode scalars and cannot be text.
        let Some(code) = char::from_u32(u32::from_str_radix(fields[0], 16).unwrap()) else {
            continue;
        };
        let class: u8 = fields[3].parse().unwrap();
        if class != 0 {
            data.combining.insert(code, class);
        }
        let mapping = fields[5];
        if let Some(tagged) = mapping.strip_prefix('<') {
            let (_, rest) = tagged.split_once("> ").unwrap();
            data.compatibility
                .insert(code, rest.split(' ').map(scalar).collect());
        } else if !mapping.is_empty() {
            data.canonical
                .insert(code, mapping.split(' ').map(scalar).collect());
        }
    }
    data
}

const HANGUL_BASE: u32 = 0xAC00;
const HANGUL_COUNT: u32 = 11172;

impl UnicodeData {
    fn class(&self, value: char) -> u8 {
        self.combining.get(&value).copied().unwrap_or(0)
    }

    /// UAX #15 full decomposition followed by canonical ordering, derived only
    /// from `UnicodeData.txt` and the Hangul algorithm.
    fn decompose(&self, value: char, compatibility: bool) -> Vec<char> {
        let mut out = Vec::new();
        self.push_decomposed(value, compatibility, &mut out);
        // Stable canonical ordering of each non-starter run.
        let mut start = 0;
        while start < out.len() {
            if self.class(out[start]) == 0 {
                start += 1;
                continue;
            }
            let end = (start..out.len())
                .find(|index| self.class(out[*index]) == 0)
                .unwrap_or(out.len());
            out[start..end].sort_by_key(|value| self.class(*value));
            start = end;
        }
        out
    }

    fn push_decomposed(&self, value: char, compatibility: bool, out: &mut Vec<char>) {
        let code = u32::from(value);
        if (HANGUL_BASE..HANGUL_BASE + HANGUL_COUNT).contains(&code) {
            let index = code - HANGUL_BASE;
            out.push(char::from_u32(0x1100 + index / 588).unwrap());
            out.push(char::from_u32(0x1161 + (index % 588) / 28).unwrap());
            if !index.is_multiple_of(28) {
                out.push(char::from_u32(0x11A7 + index % 28).unwrap());
            }
            return;
        }
        let mapping = self.canonical.get(&value).or(if compatibility {
            self.compatibility.get(&value)
        } else {
            None
        });
        match mapping {
            Some(parts) => parts
                .iter()
                .for_each(|part| self.push_decomposed(*part, compatibility, out)),
            None => out.push(value),
        }
    }
}

fn full_composition_exclusions() -> BTreeSet<char> {
    read("DerivedNormalizationProps.txt")
        .lines()
        .filter_map(|line| {
            let (range, rest) = line.split_once(';')?;
            (rest.split('#').next()?.trim() == "Full_Composition_Exclusion")
                .then(|| range.trim().to_owned())
        })
        .flat_map(|range| ranges(&range).collect::<Vec<_>>())
        .collect()
}

#[trace("Task-048")]
#[test]
fn normalization_tables_equal_the_unicode_17_character_database() {
    let data = unicode_data();
    let excluded = full_composition_exclusions();
    let listed: BTreeSet<char> = read("CompositionExclusions.txt")
        .lines()
        .filter_map(|line| line.split('#').next())
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .flat_map(|entry| ranges(entry).collect::<Vec<_>>())
        .collect();
    assert!(listed.is_subset(&excluded));

    for value in (0..=0x10FFFF_u32).filter_map(char::from_u32) {
        assert_eq!(
            canonical_combining_class(value),
            data.class(value),
            "{value:?}"
        );
        let nfd: Vec<char> = value.to_string().nfd().collect();
        assert_eq!(nfd, data.decompose(value, false), "NFD {value:?}");
        let nfkd: Vec<char> = value.to_string().nfkd().collect();
        assert_eq!(nfkd, data.decompose(value, true), "NFKD {value:?}");
    }

    // Every primary composite composes, every excluded pair does not, and no
    // other pair of decomposition parts composes.
    let mut pairs = BTreeMap::new();
    for (composite, parts) in &data.canonical {
        if let [first, second] = parts.as_slice() {
            pairs.insert((*first, *second), *composite);
            let expected = (!excluded.contains(composite)).then_some(*composite);
            assert_eq!(compose(*first, *second), expected, "{composite:?}");
        }
    }
    let firsts: BTreeSet<char> = pairs.keys().map(|(first, _)| *first).collect();
    let seconds: BTreeSet<char> = pairs.keys().map(|(_, second)| *second).collect();
    for first in &firsts {
        for second in &seconds {
            if !pairs.contains_key(&(*first, *second)) {
                assert_eq!(compose(*first, *second), None, "{first:?} {second:?}");
            }
        }
    }
}

const UNLIMITED: ScalarLimits = ScalarLimits {
    integer_bits: u64::MAX,
    decimal_digits: u64::MAX,
    scale_expansion: u64::MAX,
    text_input_bytes: u64::MAX,
    text_scalars: u64::MAX,
    normalized_scalars: u64::MAX,
    unit_edges: u64::MAX,
    value_occurrences: u64::MAX,
    work_units: u64::MAX,
    result_units: u64::MAX,
};

fn normalize(profile: TextProfile, text: &str) -> String {
    let payload = TextPayload::from_utf8(text.as_bytes()).unwrap();
    let text_type = TextType::new(0, u64::MAX, profile).unwrap();
    admit_text(&payload, &text_type, &mut Meter::new(UNLIMITED))
        .completed()
        .unwrap()
        .retained()
        .to_owned()
}

#[trace("TC-186", "FR-141-AC-1")]
#[test]
fn text_admission_passes_the_complete_normalization_test_corpus() {
    use TextProfile::{Nfc, Nfd, Nfkc, Nfkd};
    let corpus = read("NormalizationTest.txt");
    let mut part = "";
    let mut part_one = BTreeSet::new();
    let mut lines = 0;
    for line in corpus.lines() {
        if let Some(header) = line.strip_prefix('@') {
            part = header.split_whitespace().next().unwrap();
            continue;
        }
        let Some(body) = line
            .split('#')
            .next()
            .filter(|body| !body.trim().is_empty())
        else {
            continue;
        };
        let c: Vec<String> = body.split(';').take(5).map(scalars).collect();
        assert_eq!(c.len(), 5, "{line}");
        if part == "Part1" {
            part_one.insert(c[0].chars().next().unwrap());
        }
        for source in &c[..3] {
            assert_eq!(normalize(Nfc, source), c[1], "NFC {line}");
            assert_eq!(normalize(Nfd, source), c[2], "NFD {line}");
        }
        for source in &c[3..] {
            assert_eq!(normalize(Nfc, source), c[3], "NFC {line}");
            assert_eq!(normalize(Nfd, source), c[4], "NFD {line}");
        }
        for source in &c {
            assert_eq!(normalize(Nfkc, source), c[3], "NFKC {line}");
            assert_eq!(normalize(Nfkd, source), c[4], "NFKD {line}");
        }
        lines += 1;
    }
    assert!(lines > 19_000, "{lines} corpus lines");

    // Part 1: every scalar not listed is invariant under all four forms. A
    // space starter separates them so no neighbours compose or reorder.
    let invariant: String = (0..=0x10FFFF_u32)
        .filter_map(char::from_u32)
        .filter(|value| !part_one.contains(value) && *value != ' ')
        .flat_map(|value| [value, ' '])
        .collect();
    for profile in [Nfc, Nfd, Nfkc, Nfkd] {
        assert!(normalize(profile, &invariant) == invariant, "{profile:?}");
    }
}

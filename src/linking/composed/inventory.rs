// SPDX-License-Identifier: AGPL-3.0-only
//! Exact closed intake and package-wide native declaration name collection.

use std::collections::BTreeMap;

use super::{
    Conflict, ConflictKind, DeclarationEntry, DeclarationId, DeclarationRefusal, Dimension,
    Exhaustion, InventoryIssue, NamespaceReport, ParsedSource, SyntaxNamespace, UnitId, Work,
};
use crate::syntax::composed::NativeUnit;

fn bytes(work: &mut Work, values: &[&str]) -> Result<(), Exhaustion> {
    for value in values {
        work.charge(Dimension::SourceBytes, value.len())?;
    }
    Ok(())
}

pub(super) fn prepare(
    report: &mut NamespaceReport<'_>,
    work: &mut Work,
) -> Result<Option<Vec<usize>>, Exhaustion> {
    let inventory = report.inventory;
    bytes(work, &[&inventory.language, &inventory.edition])?;
    if inventory.language != crate::syntax::LANGUAGE
        || inventory.edition != crate::syntax::composed::EDITION
    {
        report.issues.push(InventoryIssue::UnsupportedSelection);
    }
    if inventory.units.is_empty() {
        report.issues.push(InventoryIssue::EmptyInventory);
    }

    let mut expected = BTreeMap::<(&str, &str), Vec<usize>>::new();
    for (index, entry) in inventory.units.iter().enumerate() {
        work.charge(Dimension::Units, 1)?;
        bytes(
            work,
            &[
                &entry.authority,
                &entry.identity.identity,
                &entry.identity.revision,
            ],
        )?;
        if entry.authority.trim().is_empty()
            || entry.identity.identity.trim().is_empty()
            || entry.identity.revision.trim().is_empty()
        {
            report
                .issues
                .push(InventoryIssue::InvalidExpectedSource { expected: index });
        }
        expected
            .entry((&entry.identity.identity, &entry.identity.revision))
            .or_default()
            .push(index);
    }

    let mut supplied = BTreeMap::<(&str, &str), Vec<usize>>::new();
    for (index, source) in report.supplied.iter().enumerate() {
        work.charge(Dimension::Units, 1)?;
        bytes(
            work,
            &[
                &source.identity().identity,
                &source.identity().revision,
                source.path(),
                source.text(),
            ],
        )?;
        supplied
            .entry((&source.identity().identity, &source.identity().revision))
            .or_default()
            .push(index);
    }

    let mut correspondence_complete = true;
    for (key, indices) in &expected {
        if indices.len() != 1 {
            correspondence_complete = false;
            report.issues.push(InventoryIssue::DuplicateExpectedSource {
                expected: indices.clone(),
            });
        }
        if !supplied.contains_key(key) {
            correspondence_complete = false;
            for index in indices {
                report
                    .issues
                    .push(InventoryIssue::MissingSource { expected: *index });
            }
        }
    }
    for (key, indices) in &supplied {
        if indices.len() != 1 {
            correspondence_complete = false;
            report.issues.push(InventoryIssue::DuplicateSuppliedSource {
                supplied: indices.clone(),
            });
        }
        if !expected.contains_key(key) {
            correspondence_complete = false;
            for index in indices {
                report
                    .issues
                    .push(InventoryIssue::UnexpectedSource { supplied: *index });
            }
        }
    }
    if !correspondence_complete {
        return Ok(None);
    }

    let mut selections = Vec::new();
    for (index, source) in report.supplied.iter().enumerate() {
        let selected = expected[&(
            source.identity().identity.as_str(),
            source.identity().revision.as_str(),
        )][0];
        selections.push(selected);
        if source.digest() != inventory.units[selected].digest {
            report.issues.push(InventoryIssue::DigestMismatch {
                expected: selected,
                supplied: index,
            });
        }
        match crate::parse_native_source(source.clone(), report.parser_limits) {
            Ok(unit) => {
                let (language, edition) = match &unit {
                    NativeUnit::Historical(unit) => (unit.language(), unit.edition()),
                    NativeUnit::Composed(unit) => (unit.language(), unit.edition()),
                };
                if language.value != inventory.language || edition.value != inventory.edition {
                    report.issues.push(InventoryIssue::HeaderConflict {
                        supplied: index,
                        language: language.clone(),
                        edition: edition.clone(),
                    });
                }
                report.parsed.push(ParsedSource {
                    supplied: index,
                    unit,
                });
            }
            Err(diagnostic) => report.issues.push(InventoryIssue::ParseFailure {
                supplied: index,
                diagnostic,
            }),
        }
    }
    Ok(report.issues.is_empty().then_some(selections))
}

pub(super) fn collect(
    report: &mut NamespaceReport<'_>,
    expected: Vec<usize>,
    work: &mut Work,
) -> Result<SyntaxNamespace, Exhaustion> {
    let mut declarations = Vec::new();
    let mut names = BTreeMap::<String, Vec<DeclarationId>>::new();
    let mut authorities = BTreeMap::<&str, (Vec<usize>, Vec<DeclarationId>)>::new();
    for (unit_index, parsed) in report.parsed.iter().enumerate() {
        let NativeUnit::Composed(unit) = &parsed.unit else {
            unreachable!("closed composed inventory contains only composed syntax");
        };
        let selected = expected[unit_index];
        let authority = authorities
            .entry(&report.inventory.units[selected].authority)
            .or_default();
        authority.0.push(selected);
        for (declaration, syntax) in unit.declarations().iter().enumerate() {
            work.charge(Dimension::Declarations, 1)?;
            let id = DeclarationId(declarations.len());
            names.entry(syntax.name.value.clone()).or_default().push(id);
            authority.1.push(id);
            declarations.push(DeclarationEntry {
                unit: UnitId(unit_index),
                declaration,
                refusals: Vec::new(),
                references: Vec::new(),
            });
        }
    }

    let mut conflicts = Vec::new();
    for ids in names.values().filter(|ids| ids.len() > 1) {
        let group = conflicts.len();
        for id in ids {
            declarations[id.0]
                .refusals
                .push(DeclarationRefusal::DuplicateName { group });
        }
        conflicts.push(Conflict {
            kind: ConflictKind::NativeName,
            declarations: ids.clone(),
            expected: Vec::new(),
        });
    }
    for (expected, ids) in authorities
        .into_values()
        .filter(|(units, _)| units.len() > 1)
    {
        let group = conflicts.len();
        for id in &ids {
            declarations[id.0]
                .refusals
                .push(DeclarationRefusal::DuplicateAuthority { group });
        }
        conflicts.push(Conflict {
            kind: ConflictKind::SourceAuthority,
            declarations: ids,
            expected,
        });
    }

    let mut units = Vec::new();
    let mut supplied = Vec::new();
    for parsed in std::mem::take(&mut report.parsed) {
        let NativeUnit::Composed(unit) = parsed.unit else {
            unreachable!("closed composed inventory contains only composed syntax");
        };
        supplied.push(parsed.supplied);
        units.push(unit);
    }
    Ok(SyntaxNamespace {
        units,
        declarations,
        names,
        supplied,
        expected,
        conflicts,
        dependencies_complete: false,
    })
}

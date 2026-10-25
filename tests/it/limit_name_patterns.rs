// SPDX-License-Identifier: AGPL-3.0-or-later

const FR_255: &str =
    include_str!("../../spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md");

fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some('a'..='z'))
        && chars.all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        })
}

fn is_setting_name(value: &str) -> bool {
    let mut segments = value.split('.');
    segments.next().is_some_and(is_identifier) && segments.all(is_identifier)
}

#[ix_trace_rs::trace("TC-1161", "FR-255-AC-12")]
#[test]
fn every_fr255_limit_and_setting_name_matches_the_backend_provider_schema() {
    let mut table = None;
    let mut stage_settings = 0;
    let mut accounting_settings = 0;
    let mut engine_settings = 0;
    let mut engine_limits = 0;

    for line in FR_255.lines() {
        if line.starts_with("## ") {
            table = None;
            continue;
        }
        if !line.trim_start().starts_with('|') {
            table = None;
            continue;
        }

        let columns: Vec<_> = line
            .trim()
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .collect();

        if columns.first() == Some(&"Setting") {
            table = match columns.get(1).copied() {
                Some("Stage") => Some("stage"),
                Some("Counter") => Some("accounting"),
                Some("Limit") => Some("engine"),
                _ => None,
            };
            continue;
        }

        let Some(table) = table else { continue };
        if columns
            .iter()
            .all(|column| column.chars().all(|c| c == '-'))
        {
            continue;
        }

        let setting = columns[0].trim_matches('`');
        assert!(
            is_setting_name(setting),
            "FR-255 {table} setting {setting:?} must match the backend-provider SettingName pattern"
        );
        match table {
            "stage" => stage_settings += 1,
            "accounting" => {
                accounting_settings += 1;
                let counter = columns[1].trim_matches('`');
                assert!(
                    is_identifier(counter),
                    "FR-255 accounting counter {counter:?} must match the LimitName pattern"
                );
            }
            "engine" => {
                engine_settings += 1;
                let limit = columns[1].trim_matches('`');
                assert!(
                    is_identifier(limit),
                    "FR-255 engine limit {limit:?} must match the backend-provider LimitName pattern"
                );
                engine_limits += 1;
            }
            _ => unreachable!(),
        }
    }

    assert!(
        stage_settings > 0,
        "FR-255 stage settings table was not read"
    );
    assert!(
        accounting_settings > 0,
        "FR-255 accounting settings table was not read"
    );
    assert!(
        engine_settings > 0,
        "FR-255 engine settings table was not read"
    );
    assert_eq!(engine_settings, engine_limits);
}

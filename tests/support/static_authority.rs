// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-049 StaticAuthority digest vectors shared by state evaluation fixtures.

use quire_spec_language::state::CanonicalDigest;

pub fn canonical(value: char) -> CanonicalDigest {
    let digest = match value {
        '1' => "17bb26d31aa70e4655fc93485f8ffd9facabf1f09becf1db75c9ebcfe470dfe5",
        '2' => "0f987e2b9547cc4ed5e157315e4a8be8f85fc76681fe458b1caacc60b3518c87",
        '3' => "aba201d44fbf1cfd79c04cf9c813ffd60db7bb5433493fa5665e180b137f338e",
        '4' => "07039deac03f89e3778cf7cd496733e99664000cc534995317b725f04e31de0f",
        _ => panic!("unknown StaticAuthority vector"),
    };
    CanonicalDigest(format!("sha256-jcs:{digest}"))
}

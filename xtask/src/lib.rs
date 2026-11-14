//! Generates `src/icudtl.dat` from the base ICU data and the tzdata resources.

pub mod patch;

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{ensure, Context};
use serde::Deserialize;

/// The pristine ICU data the crate is based on.
pub const BASE: &str = "base/icudt77l.dat";
/// The directory with Unicode's time zone resources for one tzdata release.
pub const TZDATA_DIR: &str = "tzdata";
/// The generated data the crate ships.
pub const OUTPUT: &str = "src/icudtl.dat";
/// The crate source, which declares `TZDATA_VERSION`.
pub const LIB_RS: &str = "src/lib.rs";

/// The time zone resources ICU reads, as published per tzdata release under
/// `tzdata/icunew/<version>/44/le/` in https://github.com/unicode-org/icu-data.
pub const TZ_RESOURCES: &[&str] = &[
    "metaZones.res",
    "timezoneTypes.res",
    "windowsZones.res",
    "zoneinfo64.res",
];

const TZDATA_VERSION_PREFIX: &str = "pub const TZDATA_VERSION: &str = \"";

/// The repository root.
pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn read(path: &Path) -> anyhow::Result<Vec<u8>> {
    fs::read(path).with_context(|| format!("failed to read {}", path.display()))
}

/// Reads the time zone resources in `tzdata/`, keyed by item name.
pub fn read_tz_resources(root: &Path) -> anyhow::Result<BTreeMap<&'static str, Vec<u8>>> {
    TZ_RESOURCES
        .iter()
        .map(|&name| Ok((name, read(&root.join(TZDATA_DIR).join(name))?)))
        .collect()
}

/// Converts little-endian bytes to the `u32`s resource bundle parsers take.
pub fn to_u32s(bytes: &[u8]) -> anyhow::Result<Vec<u32>> {
    let (words, rest) = bytes.as_chunks::<4>();
    ensure!(rest.is_empty(), "data is not a multiple of 4 bytes");
    Ok(words.iter().map(|&word| u32::from_le_bytes(word)).collect())
}

/// Returns the `TZVersion` string of a `zoneinfo64.res` bundle, e.g. `2026f`.
pub fn tz_version(zoneinfo64: &[u8]) -> anyhow::Result<String> {
    #[derive(Deserialize)]
    struct ZoneInfo64 {
        #[serde(rename = "TZVersion")]
        tz_version: String,
    }
    let words = to_u32s(zoneinfo64)?;
    let ZoneInfo64 { tz_version } =
        resb::binary::from_words(&words).context("failed to read TZVersion from zoneinfo64.res")?;
    ensure!(
        tz_version.len() >= 5
            && tz_version[..4].bytes().all(|b| b.is_ascii_digit())
            && tz_version[4..].bytes().all(|b| b.is_ascii_lowercase()),
        "unexpected TZVersion {tz_version:?} in zoneinfo64.res"
    );
    Ok(tz_version)
}

/// Returns the `TZDATA_VERSION` declared in `lib_rs`.
pub fn declared_tz_version(lib_rs: &str) -> anyhow::Result<&str> {
    lib_rs
        .lines()
        .find_map(|line| {
            line.strip_prefix(TZDATA_VERSION_PREFIX)?
                .strip_suffix("\";")
        })
        .context("TZDATA_VERSION is not declared in src/lib.rs")
}

/// Returns `lib_rs` with `TZDATA_VERSION` set to `version`.
pub fn set_declared_tz_version(lib_rs: &str, version: &str) -> anyhow::Result<String> {
    let old = format!("{TZDATA_VERSION_PREFIX}{}\";", declared_tz_version(lib_rs)?);
    Ok(lib_rs.replacen(&old, &format!("{TZDATA_VERSION_PREFIX}{version}\";"), 1))
}

/// The generator's outputs.
pub struct Generated {
    /// The contents of `src/icudtl.dat`.
    pub data: Vec<u8>,
    /// The contents of `src/lib.rs`.
    pub lib_rs: String,
}

/// Generates the crate's data from the base ICU data and `tzdata/`.
pub fn generate(root: &Path) -> anyhow::Result<Generated> {
    let base = read(&root.join(BASE))?;
    let replacements = read_tz_resources(root)?;
    let version = tz_version(&replacements["zoneinfo64.res"])?;
    let data = patch::replace_items(&base, &replacements)
        .context("failed to replace the time zone resources in the ICU data")?;
    let lib_rs = fs::read_to_string(root.join(LIB_RS))?;
    let lib_rs = set_declared_tz_version(&lib_rs, &version)?;
    Ok(Generated { data, lib_rs })
}

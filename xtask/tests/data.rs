//! Checks the data the crate ships against its inputs.

use std::fs;

use icudtl::{ICU_DATA, ICU_VERSION, TZDATA_VERSION};
use xtask::{
    generate, patch::Package, read_tz_resources, root, to_u32s, tz_version, BASE, LIB_RS, OUTPUT,
    TZ_RESOURCES,
};
use zoneinfo64::{UtcOffset, ZoneInfo64};

fn base() -> Vec<u8> {
    fs::read(root().join(BASE)).unwrap()
}

fn shipped_item<'a>(package: &'a Package, name: &str) -> &'a [u8] {
    package.items[package.position(name).unwrap()]
}

#[test]
fn shipped_data_is_reproducible() {
    let root = root();
    let generated = generate(&root).unwrap();
    assert!(
        fs::read(root.join(OUTPUT)).unwrap() == generated.data,
        "{OUTPUT} is out of date; run `cargo xtask generate`"
    );
    assert_eq!(
        fs::read_to_string(root.join(LIB_RS)).unwrap(),
        generated.lib_rs,
        "TZDATA_VERSION is out of date; run `cargo xtask generate`"
    );
    assert!(ICU_DATA == generated.data, "ICU_DATA is not {OUTPUT}");
}

#[test]
fn shipped_data_is_aligned() {
    assert_eq!(ICU_DATA.as_ptr() as usize % 16, 0);
}

/// ICU reads items in place, so each one must start on a 16-byte boundary of
/// the package (which `shipped_data_is_aligned` places on one in memory).
#[test]
fn shipped_items_are_aligned() {
    let shipped = Package::parse(ICU_DATA).unwrap();
    for (name, item) in shipped.names.iter().zip(&shipped.items) {
        let offset = item.as_ptr() as usize - ICU_DATA.as_ptr() as usize;
        assert_eq!(offset % 16, 0, "{name} starts at offset {offset}");
    }
}

#[test]
fn only_tz_resources_differ_from_base() {
    let base_bytes = base();
    let base = Package::parse(&base_bytes).unwrap();
    let shipped = Package::parse(ICU_DATA).unwrap();
    let resources = read_tz_resources(&root()).unwrap();

    assert_eq!(shipped.names, base.names);
    for (i, name) in base.names.iter().enumerate() {
        let (_, basename) = name.split_once('/').unwrap();
        match resources.get(basename) {
            Some(resource) => assert!(shipped.items[i] == resource, "{name} differs from tzdata/"),
            None => assert!(
                shipped.items[i] == base.items[i],
                "{name} differs from the base"
            ),
        }
    }
    for name in TZ_RESOURCES {
        assert!(base.position(name).is_some(), "{name} is not in the base");
    }
}

#[test]
fn item_prefix_matches_icu_version() {
    let major = ICU_VERSION.split('.').next().unwrap();
    let shipped = Package::parse(ICU_DATA).unwrap();
    let prefix = format!("icudt{major}l/");
    assert!(shipped.names.iter().all(|name| name.starts_with(&prefix)));
}

#[test]
fn zoneinfo64_version_matches_tzdata_version() {
    let shipped = Package::parse(ICU_DATA).unwrap();
    let zoneinfo64 = shipped_item(&shipped, "zoneinfo64.res");
    assert_eq!(tz_version(zoneinfo64).unwrap(), TZDATA_VERSION);
}

/// 2026-12-01T12:00:00Z
const TIMESTAMP: i64 = 1_796_126_400;

/// `(time zone, expected offset in hours at TIMESTAMP)`. Each zone listed
/// after New York changed in 2026 and is wrong in the ICU 77.1 release data.
const OFFSETS: &[(&str, i32)] = &[
    ("America/New_York", -5),
    ("America/Vancouver", -7), // tzdata 2026b
    ("America/Edmonton", -6),  // 2026c
    ("America/Inuvik", -6),    // 2026d
    ("America/Winnipeg", -5),  // 2026e
    ("America/Havana", -4),    // 2026f
];

fn offset(zoneinfo64: &[u8], time_zone: &str) -> UtcOffset {
    let words = to_u32s(zoneinfo64).unwrap();
    let zoneinfo64 = ZoneInfo64::try_from_u32s(&words).unwrap();
    let zone = zoneinfo64.get(time_zone).unwrap();
    zone.for_timestamp(TIMESTAMP).offset
}

#[test]
fn shipped_zoneinfo64_has_current_offsets() {
    let shipped = Package::parse(ICU_DATA).unwrap();
    let zoneinfo64 = shipped_item(&shipped, "zoneinfo64.res");
    for &(time_zone, hours) in OFFSETS {
        assert_eq!(
            offset(zoneinfo64, time_zone),
            UtcOffset::from_seconds(hours * 3600),
            "offset of {time_zone}"
        );
    }
}

/// Makes sure the offsets test above can tell the base and shipped data apart.
#[test]
fn base_zoneinfo64_has_outdated_offsets() {
    let base_bytes = base();
    let base = Package::parse(&base_bytes).unwrap();
    let zoneinfo64 = shipped_item(&base, "zoneinfo64.res");
    assert_eq!(
        offset(zoneinfo64, "America/Vancouver"),
        UtcOffset::from_seconds(-8 * 3600)
    );
}

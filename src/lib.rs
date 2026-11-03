//! ICU 77 common data for V8, with current IANA time zone data.
//!
//! A drop-in replacement for `deno_core_icudata`:
//!
//! ```ignore
//! v8::icu::set_common_data_77(icudtl::ICU_DATA).unwrap();
//! ```
//!
//! The data is Chromium's `icudtl.dat` for ICU 77.1 with the four time zone
//! resources (`zoneinfo64.res`, `metaZones.res`, `timezoneTypes.res`, and
//! `windowsZones.res`) replaced by the ones Unicode publishes for
//! [`TZDATA_VERSION`]. See the README for details.

// From `deno_core_icudata` (MIT): V8 requires the data to be 16-byte aligned.
#[repr(C, align(16))]
struct IcuData<T: ?Sized>(T);

static ICU_DATA_RAW: &IcuData<[u8]> = &IcuData(*include_bytes!("icudtl.dat"));

/// Raw ICU data.
pub static ICU_DATA: &[u8] = &ICU_DATA_RAW.0;

/// The ICU version of the data. V8 must be built against the same ICU major
/// version.
pub const ICU_VERSION: &str = "77.1";

/// The IANA tzdata release of the time zone data. Written by `cargo xtask
/// generate`.
pub const TZDATA_VERSION: &str = "2026f";

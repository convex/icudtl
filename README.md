# icudtl

ICU 77 common data (`icudtl.dat`) for V8, with current IANA time zone data.

This crate is a drop-in replacement for
[`deno_core_icudata`](https://crates.io/crates/deno_core_icudata). It ships the
same ICU 77.1 data, except that the four time zone resources are replaced with
the ones Unicode publishes for the latest IANA tzdata release.

```toml
[dependencies]
icudtl = "0.77"
# or, without crates.io:
icudtl = { git = "https://github.com/convex/icudtl", rev = "<commit>" }
```

```rust
v8::icu::set_common_data_77(icudtl::ICU_DATA).unwrap();

assert_eq!(icudtl::ICU_VERSION, "77.1");
println!("tzdata {}", icudtl::TZDATA_VERSION); // e.g. "2026f"
```

The crate has no dependencies and no build script, and it never touches the
network at build time. `ICU_DATA` is a committed file included with
`include_bytes!`, aligned to 16 bytes as V8 requires.

## Why

V8 has no time zone data of its own. `Intl` and `Temporal` read it from ICU's
common data, which the embedder hands to V8 at startup. `deno_core_icudata`
ships Chromium's copy of that data and is only republished when ICU's major
version changes. That means its tzdata can be many IANA releases behind. The
ICU 77.1 data has tzdata 2025b, so it gets the 2026 rule changes wrong. For
example, it says `America/Vancouver` is at `GMT-08:00` on 2026-11-15 instead
of `GMT-07:00` (British Columbia's change in tzdata 2026b). Alberta (2026c),
the Northwest Territories (2026d), Manitoba (2026e), and Cuba (2026f) are
wrong too.

V8 is built against a specific ICU major version, so the data must stay ICU
77 data. Only the time zone resources can change. ICU keeps them in four
resource bundles:

- `zoneinfo64.res`: zone rules and offsets
- `metaZones.res`: metazone mappings, used for time zone names
- `timezoneTypes.res`: BCP 47 time zone IDs and aliases
- `windowsZones.res`: Windows time zone mappings

Unicode publishes those four files for every IANA release, in a format every
ICU since 4.4 can read.

## Contents

| Path | What it is |
| --- | --- |
| `src/icudtl.dat` | The data the crate ships. Generated, don't edit by hand. |
| `src/lib.rs` | `ICU_DATA`, `ICU_VERSION`, and `TZDATA_VERSION`. The generator writes `TZDATA_VERSION`. |
| `base/icudt77l.dat` | The pristine ICU 77.1 data from `deno_core_icudata` 0.77.0 / Chromium. See [`base/README.md`](base/README.md). |
| `tzdata/*.res` | Unicode's time zone resources for `TZDATA_VERSION`, from `https://github.com/unicode-org/icu-data/tree/main/tzdata/icunew/<version>/44/le`. `44` is the resource bundle format and `le` is little-endian. |
| `xtask/` | The generator and the data tests. Not published. |
| `e2e/` | An end-to-end test that loads the data into V8. Not published. |
| `scripts/update-tzdata.sh` | Downloads a tzdata release and regenerates the data. |
| `scripts/check-icupkg.sh` | Checks the generated data with ICU's `icupkg`. |

Only `src/`, `Cargo.toml`, the README, and the licenses are in the published
package. It is about 4.6 MB compressed, well under crates.io's 10 MB limit.

### How the data is generated

`cargo xtask generate` reads `base/icudt77l.dat` and the four files in
`tzdata/`. It checks that each replacement is an ICU resource bundle (`ResB`)
whose size is a multiple of 16 bytes. It then rewrites the package's table of
contents with those four items swapped in. Every other item is kept
byte-for-byte, and every item stays aligned to 16 bytes. The output goes to
`src/icudtl.dat`, and the `TZVersion` string read from `zoneinfo64.res` goes to
`TZDATA_VERSION` in `src/lib.rs`. The patcher in `xtask/src/patch.rs` is
adapted from the one in convex/convex.

The generator is deterministic. CI checks that the committed files match its
output (`cargo xtask generate --check`).

## Updating tzdata by hand

IANA announces releases at https://data.iana.org/time-zones/tzdb/NEWS. Unicode
usually publishes the matching resources a few days later at
https://github.com/unicode-org/icu-data/tree/main/tzdata/icunew.

```sh
scripts/update-tzdata.sh 2026g
cargo test
scripts/check-icupkg.sh   # needs icupkg: `apt install icu-devtools` or `dnf install icu`
```

The script downloads the four files into `tzdata/` and runs the generator. It
fails if `zoneinfo64.res` doesn't report the requested version. It also bumps
the crate's patch version (for example `0.77.3` to `0.77.4`). crates.io ignores
`+build` metadata, so every data change needs a new version. Commit everything
it changes.

## Tests

`cargo test` checks that:

- `src/icudtl.dat` and `TZDATA_VERSION` are exactly what the generator
  produces.
- The shipped package has the same items as the base, in the same order.
  The four tz items equal the files in `tzdata/`, and every other item is
  byte-identical to the base.
- The item prefix (`icudt77l/`) matches `ICU_VERSION`.
- `zoneinfo64.res`'s `TZVersion` is `TZDATA_VERSION`.
- Read with ICU4X's `zoneinfo64` crate, the shipped `zoneinfo64.res` gives
  the right UTC offsets at 2026-12-01T12:00Z for zones that changed in 2026.
  The base data gets them wrong.

`scripts/check-icupkg.sh` checks the same structure independently, with
`icupkg` extracting every item from both packages.

`cargo test -p e2e` loads `ICU_DATA` into V8 with the `v8` crate (149.x, built
with ICU 77). It checks that `Intl.DateTimeFormat` and `Temporal` report the
right offsets for the same zones. It downloads a prebuilt V8, so it isn't part
of a plain `cargo test`. CI runs it on every PR.

## Versioning and branches

The major and minor versions track the ICU major version of the data, like
`deno_core_icudata`: `0.77.x` is ICU 77. The patch version goes up with every
tzdata update.

`main` holds the ICU major version that Convex currently uses. To move to a
new ICU major version (say 78), consumers have to upgrade V8 at the same time,
so this is never automatic:

1. Branch `icu-77` from `main` so ICU 77 can still get tzdata updates.
2. On `main`, replace `base/icudt77l.dat` with the new base, for example
   `icudt78l.dat` from the new `deno_core_icudata` release or rusty_v8's
   `third_party/icu/common/icudtl.dat`. Update `base/README.md`, `BASE` in
   `xtask/src/lib.rs`, `ICU_VERSION`, and the `set_common_data_*` references.
   Set the crate version to `0.78.0`.
3. Run `cargo xtask generate` and `cargo test`.

## License

- The ICU data, including Unicode's time zone resources, is under the Unicode
  License v3, with the third-party notices in ICU's license. See
  [`LICENSE-ICU`](LICENSE-ICU), ICU 77.1's `LICENSE`.
- The IANA tz database the time zone resources are built from is in the
  public domain.
- The code is under the MIT license ([`LICENSE-MIT`](LICENSE-MIT)).
  `src/lib.rs` is based on `deno_core_icudata` by the Deno authors (MIT).

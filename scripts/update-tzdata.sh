#!/usr/bin/env bash
# Updates the crate to an IANA tzdata release, e.g. `scripts/update-tzdata.sh 2026g`:
# downloads Unicode's ICU time zone resources for it into `tzdata/`, regenerates
# `src/icudtl.dat` and `TZDATA_VERSION`, and bumps the crate's patch version if
# the tzdata version changed.
set -euo pipefail

version="${1:?usage: $0 <tzdata version, e.g. 2026g>}"
if [[ ! "$version" =~ ^[0-9]{4}[a-z]+$ ]]; then
    echo "error: '$version' is not a tzdata version like 2026g" >&2
    exit 1
fi
cd "$(dirname "$0")/.."

# The base's `metaZones.res` stays; see `TZ_RESOURCES` in `xtask/src/lib.rs`.
# `44` is the resource bundle format, readable by every ICU since 4.4, and `le`
# is little-endian.
url="https://raw.githubusercontent.com/unicode-org/icu-data/main/tzdata/icunew/$version/44/le"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
for resource in timezoneTypes windowsZones zoneinfo64; do
    curl --fail --silent --show-error --location --retry 3 \
        -o "$tmp/$resource.res" "$url/$resource.res"
done

tzdata_version() {
    sed -nE 's/^pub const TZDATA_VERSION: &str = "(.*)";$/\1/p' src/lib.rs
}
old_version="$(tzdata_version)"
cp "$tmp"/*.res tzdata/
cargo xtask generate
new_version="$(tzdata_version)"
if [[ "$new_version" != "$version" ]]; then
    echo "error: zoneinfo64.res for $version reports TZVersion $new_version" >&2
    exit 1
fi

if [[ "$new_version" != "$old_version" ]]; then
    # crates.io ignores build metadata when comparing versions, so every data
    # change needs a new patch version.
    crate_version="$(sed -nE 's/^version = "([0-9]+\.[0-9]+\.[0-9]+)"$/\1/p' Cargo.toml | head -n 1)"
    next_version="${crate_version%.*}.$((${crate_version##*.} + 1))"
    # Replaces the first `version = ...` line, the package's.
    OLD="$crate_version" NEW="$next_version" \
        perl -0777 -pi -e 's/^version = "\Q$ENV{OLD}\E"$/version = "$ENV{NEW}"/m' Cargo.toml
    cargo update --workspace --offline --quiet
    echo "Updated tzdata from $old_version to $new_version (icudtl $crate_version -> $next_version)"
else
    echo "tzdata is already $new_version"
fi

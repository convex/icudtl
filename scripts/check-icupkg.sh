#!/usr/bin/env bash
# Independently checks `src/icudtl.dat` with ICU's `icupkg` (Ubuntu package
# `icu-devtools`): it must have the same items as the base data, the time zone
# items must equal the files in `tzdata/`, and every other item must be
# unchanged.
set -euo pipefail
cd "$(dirname "$0")/.."

tz_resources=(timezoneTypes.res windowsZones.res zoneinfo64.res)
base="$(echo base/icudt*.dat)"  # e.g. base/icudt77l.dat
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

# icupkg derives the item name prefix from the file name.
mkdir "$tmp/shipped"
cp src/icudtl.dat "$tmp/shipped/$(basename "$base")"

# Chromium trims the data, so a few items depend on items that aren't in it.
# icupkg reports those and exits with 2 (U_MISSING_RESOURCE_ERROR); any other
# error fails the check.
icupkg_checked() {
    local status=0
    icupkg "$@" 2> "$tmp/stderr" || status=$?
    if [[ "$status" != 0 ]] && { [[ "$status" != 2 ]] ||
        grep -qv '^Item .* depends on missing item ' "$tmp/stderr"; }; then
        cat "$tmp/stderr" >&2
        echo "error: icupkg $* failed with status $status" >&2
        exit 1
    fi
}

for package in base shipped; do
    if [[ "$package" == base ]]; then file="$base"; else file="$tmp/shipped/$(basename "$base")"; fi
    icupkg_checked --list --outlist "$tmp/$package.txt" "$file"
    mkdir "$tmp/$package-items"
    icupkg_checked --extract "$tmp/$package.txt" --destdir "$tmp/$package-items" "$file"
done

if ! diff -u "$tmp/base.txt" "$tmp/shipped.txt"; then
    echo "error: the item lists differ" >&2
    exit 1
fi
echo "Both packages have the same $(wc -l < "$tmp/base.txt") items"

changed="$({ diff -rq "$tmp/base-items" "$tmp/shipped-items" || true; } |
    sed -nE "s|^Files $tmp/base-items/(.*) and .* differ$|\1|p" | sort)"
expected="$(printf '%s\n' "${tz_resources[@]}" | sort)"
if [[ "$changed" != "$expected" ]]; then
    echo "error: expected exactly the time zone items to differ from the base, got:" >&2
    diff -rq "$tmp/base-items" "$tmp/shipped-items" >&2 || true
    exit 1
fi
for resource in "${tz_resources[@]}"; do
    cmp "$tmp/shipped-items/$resource" "tzdata/$resource"
done
echo "Only ${tz_resources[*]} differ from the base, and they match tzdata/"

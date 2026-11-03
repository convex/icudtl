//! Replaces items in an ICU common data package (`.dat`).
//!
//! This is a custom implementation of the package format: no Rust crate writes
//! ICU4C packages, and ICU's own `icupkg` is a C++ command-line tool. CI checks
//! the output independently of this code: `scripts/check-icupkg.sh` uses
//! `icupkg` to extract every item and compares them byte for byte with the
//! base data and `tzdata/`.
//!
//! A package is a `DataHeader` followed by a table of contents: an item count,
//! then `(name_offset, data_offset)` pairs relative to the start of the table,
//! then the NUL-terminated item names, then the items themselves, each starting
//! on a 16-byte boundary. An item's length is the distance to the next item's
//! offset (or the end of the package). See ICU's `ucmndata.h` and
//! `tools/toolutil/package.cpp`.

use std::collections::BTreeMap;

use anyhow::{anyhow, ensure, Context};

const ALIGNMENT: usize = 16;

struct Header<'a> {
    bytes: &'a [u8],
    data_format: &'a [u8],
}

fn parse_header(data: &[u8]) -> anyhow::Result<Header<'_>> {
    ensure!(data.len() >= 24, "data is too short for an ICU data header");
    let header_size = u16::from_le_bytes([data[0], data[1]]) as usize;
    ensure!(
        data[2] == 0xda && data[3] == 0x27,
        "missing ICU data header magic"
    );
    ensure!(data[8] == 0, "only little-endian ICU data is supported");
    ensure!(
        header_size <= data.len(),
        "ICU data header overruns the data"
    );
    Ok(Header {
        bytes: &data[..header_size],
        data_format: &data[12..16],
    })
}

fn read_u32(data: &[u8], offset: usize) -> anyhow::Result<usize> {
    let bytes = data
        .get(offset..offset + 4)
        .with_context(|| format!("truncated ICU table of contents at {offset}"))?;
    Ok(u32::from_le_bytes(bytes.try_into()?) as usize)
}

/// A parsed ICU common data package.
pub struct Package<'a> {
    header: Header<'a>,
    /// The table of contents and the items that follow it.
    toc: &'a [u8],
    /// The offset of the first item in `toc`.
    first_item: usize,
    /// The item names, e.g. `icudt77l/zoneinfo64.res`.
    pub names: Vec<&'a str>,
    /// The item contents, in the same order as `names`.
    pub items: Vec<&'a [u8]>,
}

impl<'a> Package<'a> {
    pub fn parse(package: &'a [u8]) -> anyhow::Result<Self> {
        let header = parse_header(package)?;
        ensure!(
            header.data_format == b"CmnD",
            "not an ICU common data package"
        );
        let toc = &package[header.bytes.len()..];
        let count = read_u32(toc, 0)?;
        let mut names = Vec::with_capacity(count);
        let mut offsets = Vec::with_capacity(count);
        for i in 0..count {
            let name_offset = read_u32(toc, 4 + 8 * i)?;
            let name = toc
                .get(name_offset..)
                .and_then(|rest| rest.split(|&b| b == 0).next())
                .context("ICU item name is out of bounds")?;
            names.push(std::str::from_utf8(name)?);
            offsets.push(read_u32(toc, 8 + 8 * i)?);
        }
        let first_item = *offsets.first().context("ICU package has no items")?;
        ensure!(
            offsets.is_sorted() && toc.len() >= *offsets.last().unwrap(),
            "ICU item offsets are not ascending"
        );
        let items = (0..count)
            .map(|i| &toc[offsets[i]..offsets.get(i + 1).copied().unwrap_or(toc.len())])
            .collect();
        Ok(Self {
            header,
            toc,
            first_item,
            names,
            items,
        })
    }

    /// Returns the item whose name after the `icudt<version><endianness>/`
    /// prefix is `name`.
    pub fn position(&self, name: &str) -> Option<usize> {
        self.names
            .iter()
            .position(|item| item.split_once('/').map(|(_, base)| base) == Some(name))
    }
}

/// Returns the bytes of the package with each item whose name (after the
/// `icudt<version><endianness>/` prefix) is a key of `replacements` swapped
/// for the corresponding value. Every replacement must be a resource bundle
/// that names an existing item.
pub fn replace_items(
    package: &[u8],
    replacements: &BTreeMap<&str, Vec<u8>>,
) -> anyhow::Result<Vec<u8>> {
    let parsed = Package::parse(package)?;
    let mut items = parsed.items.clone();
    for (name, replacement) in replacements {
        ensure!(
            replacement.len() % ALIGNMENT == 0,
            "{name} is not a multiple of {ALIGNMENT} bytes"
        );
        let replacement_header = parse_header(replacement)?;
        ensure!(
            replacement_header.data_format == b"ResB",
            "{name} is not an ICU resource bundle"
        );
        let index = parsed
            .position(name)
            .ok_or_else(|| anyhow!("{name} is not in the ICU package"))?;
        items[index] = replacement;
    }

    let Package {
        header,
        toc,
        first_item,
        ..
    } = parsed;
    let count = items.len();
    let mut out = Vec::with_capacity(package.len());
    out.extend_from_slice(header.bytes);
    out.extend_from_slice(&toc[..4]);
    let mut offset = first_item;
    for (i, item) in items.iter().enumerate() {
        out.extend_from_slice(&toc[4 + 8 * i..8 + 8 * i]);
        out.extend_from_slice(&u32::try_from(offset)?.to_le_bytes());
        offset += item.len();
    }
    out.extend_from_slice(&toc[4 + 8 * count..first_item]);
    for item in items {
        out.extend_from_slice(item);
    }
    Ok(out)
}

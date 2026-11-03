# Base ICU data

`icudt77l.dat` is the ICU 77.1 common data package that the crate's
`src/icudtl.dat` is generated from. The generator replaces its time zone
resources and leaves every other item unchanged.

It is byte-identical to:

- `src/icudtl.dat` in the [`deno_core_icudata`](https://crates.io/crates/deno_core_icudata)
  0.77.0 crate, and
- Chromium's `third_party/icu/common/icudtl.dat` at ICU commit
  [`ee5f27adc`](https://chromium.googlesource.com/chromium/deps/icu/+/ee5f27adc),
  the copy that `rusty_v8` ships.

It is ICU 77.1 with IANA tzdata 2025b.

```
md5     5bfd3eefca4b5ef2d1a869396390a7de
sha256  1cf67874b5a87a8363a86fb3f81e3cbbed54d389062dab8fb52308d5cf8c8612
size    10822192 bytes
```

The file is named after its item prefix (`icudt77l`: ICU data, version 77,
little-endian) because ICU's `icupkg` derives the prefix from the file name.

Only replace it when moving to a new ICU major version (see the top-level
README). V8 must be built against the same ICU major version as the data.

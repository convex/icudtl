# Base ICU data

`icudt77l.dat` is the unmodified ICU 77.1 data (tzdata 2025b) that
`src/icudtl.dat` is generated from. It is byte-identical to `src/icudtl.dat`
in [`deno_core_icudata`](https://crates.io/crates/deno_core_icudata) 0.77.0
and to Chromium's `third_party/icu/common/icudtl.dat` at ICU commit
[`ee5f27adc`](https://chromium.googlesource.com/chromium/deps/icu/+/ee5f27adc),
the copy rusty_v8 ships.

```
md5     5bfd3eefca4b5ef2d1a869396390a7de
sha256  1cf67874b5a87a8363a86fb3f81e3cbbed54d389062dab8fb52308d5cf8c8612
size    10822192 bytes
```

The file name must match the item prefix (`icudt77l`) for `icupkg`. Tzdata
updates don't change this file. Replace it only when the upstream ICU data
changes (see "Versioning" in the top-level README).

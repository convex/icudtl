//! `cargo xtask generate [--check]`: writes `src/icudtl.dat` and
//! `TZDATA_VERSION` from `base/` and `tzdata/`. With `--check`, fails instead
//! if the committed files differ from what the generator would write.

use std::{env, fs};

use anyhow::{bail, ensure};
use xtask::{generate, root, LIB_RS, OUTPUT};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    let check = match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["generate"] => false,
        ["generate", "--check"] => true,
        _ => bail!("usage: cargo xtask generate [--check]"),
    };
    let root = root();
    let generated = generate(&root)?;
    let version = xtask::declared_tz_version(&generated.lib_rs)?;
    if check {
        ensure!(
            fs::read(root.join(OUTPUT))? == generated.data,
            "{OUTPUT} is out of date; run `cargo xtask generate`"
        );
        ensure!(
            fs::read_to_string(root.join(LIB_RS))? == generated.lib_rs,
            "TZDATA_VERSION in {LIB_RS} is out of date; run `cargo xtask generate`"
        );
        println!("{OUTPUT} is up to date (tzdata {version})");
    } else {
        fs::write(root.join(OUTPUT), &generated.data)?;
        fs::write(root.join(LIB_RS), &generated.lib_rs)?;
        println!("Wrote {OUTPUT} (tzdata {version})");
    }
    Ok(())
}

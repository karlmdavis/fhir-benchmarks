/// This build script uses [vergen](https://docs.rs/vergen/5.1.5/vergen/index.html) to pass Git & build
/// metadata to the compiler's environment variables.
use anyhow::Result;
use vergen::{vergen, Config};

fn main() -> Result<()> {
    // Generate the default 'cargo:' instruction output
    let mut config = Config::default();
    *config.git_mut().semver_dirty_mut() = Some("-dirty");
    vergen(config)
}

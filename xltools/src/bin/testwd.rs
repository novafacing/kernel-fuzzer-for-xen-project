use std::{error::Error, path::PathBuf};

use xltools::{checkroot, presets::windows_dev};

fn main() -> Result<(), Box<dyn Error>> {
    checkroot()?;

    windows_dev(
        PathBuf::from("/home/rhart/hub/winkfx/media/auto.iso"),
        PathBuf::from("/home/rhart/hub/winkfx/images/windev.img"),
        "user".to_string(),
        "password".to_string(),
    )?;

    Ok(())
}

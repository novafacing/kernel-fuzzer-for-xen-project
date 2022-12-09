use std::path::PathBuf;

use xltools::presets::windows_dev;

fn main() {
    windows_dev(
        PathBuf::from("/home/rhart/hub/winkfx/media/auto.iso"),
        PathBuf::from("/home/rhart/hub/winkfx/images/windev.img"),
        "user".to_string(),
        "password".to_string(),
    );
}

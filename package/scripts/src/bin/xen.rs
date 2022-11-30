//! Script to configure and install the Xen hypervisor

use std::{collections::HashSet, error::Error, path::PathBuf, process::Command};

use scripts::{append_line, read_os_release};

use num_cpus::get as nproc;

const BASE_CONFIGURE_OPTIONS: &[&str] = &[
    "--enable-systemd",
    "--disable-pvshim",
    "--enable-githttp",
    "--prefix=/usr",
];

fn configure_xen() -> Result<(), Box<dyn Error>> {
    let os_release = read_os_release()?;

    let mut configure_options: HashSet<String> = BASE_CONFIGURE_OPTIONS
        .iter()
        .map(|d| d.to_string())
        .collect();

    if match os_release.get("VERSION_CODENAME") {
        Some(codename) => codename.to_lowercase() != "jammy",
        None => {
            panic!("No version codename found in /etc/os-release");
        }
    } {
        configure_options.insert("--enable-ovmf".to_string());
    }

    Command::new("./configure")
        .args(configure_options)
        .spawn()
        .expect("Could not run configure command")
        .wait()
        .expect("configure command failed");

    let xenconfig_file = PathBuf::from("xen/.config");
    append_line(xenconfig_file.clone(), "CONFIG_EXPERT=y".to_string());
    append_line(xenconfig_file, "CONFIG_MEM_SHARING=y".to_string());

    Ok(())
}

fn build_xen() -> Result<(), Box<dyn Error>> {
    Command::new("make")
        .current_dir(PathBuf::from("xen"))
        .arg("olddefconfig")
        .spawn()
        .expect("Could not run make olddefconfig")
        .wait()
        .expect("make olddefconfig command failed");

    Command::new("make")
        .arg("-j")
        .arg(nproc().to_string())
        .arg("dist-xen")
        .spawn()
        .expect("Could not run make dist-xen")
        .wait()
        .expect("make dist-xen command failed");

    Command::new("make")
        .arg("-j")
        .arg(nproc().to_string())
        .arg("dist-tools")
        .spawn()
        .expect("Could not run make dist-tools")
        .wait()
        .expect("make dist-tools command failed");

    Command::new("make")
        .arg("-j")
        .arg(nproc().to_string())
        .arg("install-xen")
        .spawn()
        .expect("Could not run make install-xen")
        .wait()
        .expect("make install-xen command failed");

    Command::new("make")
        .arg("-j")
        .arg(nproc().to_string())
        .arg("install-tools")
        .spawn()
        .expect("Could not run make install-tools")
        .wait()
        .expect("make install-tools command failed");

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    configure_xen()?;
    build_xen()?;
    Ok(())
}

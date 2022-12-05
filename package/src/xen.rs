//! Script to configure and install the Xen hypervisor

use std::{
    collections::HashSet,
    error::Error,
    fs::{create_dir_all, remove_dir_all, File},
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

use crate::{
    append_line, check_command, copy_dir, dir_size, get_distro, get_dpkg_arch, get_version,
    read_os_release, write_file, DebControl,
};
use log::{error, info};

use num_cpus::get as nproc;
use tempdir::TempDir;
use walkdir::WalkDir;

const XEN_CFG_FILE: &[u8] = include_bytes!("../resource/etc/default/grub.d/xen.cfg");
const XEN_CONF_FILE: &[u8] = include_bytes!("../resource/etc/modules-load.d/xen.conf");
const KFX_FIND_XEN_DEFAULTS_FILE: &[u8] =
    include_bytes!("../resource/usr/bin/kfx-find-xen-defaults");
const POSTINST_FILE: &[u8] = include_bytes!("../resource/postinst");
const POSTRM_FILE: &[u8] = include_bytes!("../resource/postrm");

const BASE_CONFIGURE_OPTIONS: &[&str] = &[
    "--enable-systemd",
    "--disable-pvshim",
    "--enable-githttp",
    "--prefix=/usr",
];

fn get_xenversion(xen_path: &PathBuf) -> Result<String, Box<dyn Error>> {
    let boot_dir = xen_path.join("dist/install/boot");
    for entry in boot_dir.read_dir()? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let filename = path.file_name().unwrap().to_str().unwrap();
            if filename.starts_with("xen-") && filename.ends_with(".gz") {
                let version = filename
                    .strip_prefix("xen-")
                    .unwrap()
                    .strip_suffix(".gz")
                    .unwrap();
                return Ok(version.to_string());
            }
        }
    }
    Err("No xen version found in dist/install/boot")?
}

pub fn configure_xen(xen_path: &PathBuf) -> Result<(), Box<dyn Error>> {
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

    info!("Configuring Xen with options: {:?}", configure_options);

    check_command(
        Command::new("./configure")
            .args(configure_options)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir(&xen_path)
            .spawn()
            .expect("Could not run configure command")
            .wait_with_output(),
    )?;

    info!("Writing xen/.config");

    let xenconfig_file = xen_path.join("xen/.config");
    append_line(&xenconfig_file, "CONFIG_EXPERT=y".to_string())?;
    append_line(&xenconfig_file, "CONFIG_MEM_SHARING=y".to_string())?;

    Ok(())
}

pub fn build_xen(xen_path: &PathBuf) -> Result<(), Box<dyn Error>> {
    let xen_subdir_path = xen_path.join("xen");

    info!("Making olddefconfig");
    check_command(
        Command::new("make")
            .arg("olddefconfig")
            .current_dir(&xen_subdir_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Could not run make olddefconfig")
            .wait_with_output(),
    )?;

    info!("Making dist-xen");
    check_command(
        Command::new("make")
            .arg("-j")
            .arg(nproc().to_string())
            .arg("dist-xen")
            .current_dir(&xen_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Could not run make dist-xen")
            .wait_with_output(),
    )?;

    info!("Making dist-tools");
    check_command(
        Command::new("make")
            .arg("-j")
            .arg(nproc().to_string())
            .arg("dist-tools")
            .current_dir(&xen_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Could not run make dist-tools")
            .wait_with_output(),
    )?;

    info!("Making install-xen");
    check_command(
        Command::new("make")
            .arg("-j")
            .arg(nproc().to_string())
            .arg("install-xen")
            .current_dir(&xen_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Could not run make install-xen")
            .wait_with_output(),
    )?;

    info!("Making install-tools");
    check_command(
        Command::new("make")
            .arg("-j")
            .arg(nproc().to_string())
            .arg("install-tools")
            .current_dir(&xen_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Could not run make install-tools")
            .wait_with_output(),
    )?;

    Ok(())
}

pub fn make_deb(xen_path: &PathBuf, output_path: &PathBuf) -> Result<(), Box<dyn Error>> {
    let xenversion = get_xenversion(xen_path)?;
    let distro = get_distro()?;
    let version = get_version()?;
    let arch = get_dpkg_arch()?;

    let deb_name = format!("xen_{}-{}-{}.deb", &xenversion, &version, &arch);

    let install_dir = xen_path.join("dist/install");

    let tmpdir = TempDir::new("deb")?;
    let deb_dir = tmpdir.path().to_path_buf();

    // Copy everything in the install dir to the deb dir
    copy_dir(&install_dir, &deb_dir)?;

    // Create the debian directory
    let debian_dir = deb_dir.join("DEBIAN");
    // Create the grub.d and modules-load.d directories
    let grub_dir = deb_dir.join("etc/default/grub.d");
    let modules_dir = deb_dir.join("etc/modules-load.d");

    create_dir_all(&debian_dir)?;
    create_dir_all(&grub_dir)?;
    create_dir_all(&modules_dir)?;

    // Debian doesn't use lib64, ubuntu does
    match distro.as_str() {
        "debian" => {
            let lib_dir = deb_dir.join("usr/lib");
            copy_dir(&deb_dir.join("usr/lib64"), &lib_dir)?;

            remove_dir_all(&deb_dir.join("usr/lib64"))?;
        }
        _ => {}
    }

    write_file(&debian_dir.join("postinst"), POSTINST_FILE, 0o755)?;
    write_file(&debian_dir.join("postrm"), POSTRM_FILE, 0o755)?;
    write_file(&grub_dir.join("xen.cfg"), XEN_CFG_FILE, 0o644)?;
    write_file(&modules_dir.join("xen.conf"), XEN_CONF_FILE, 0o644)?;
    write_file(
        &deb_dir.join("usr/bin/kfx-find-xen-defaults"),
        KFX_FIND_XEN_DEFAULTS_FILE,
        0o755,
    )?;

    let deb_dir_size = dir_size(&deb_dir)?;

    assert!(deb_dir.exists(), "Install directory does not exist");

    let deb_control = DebControl::new(
        "xen-hypervisor".to_string(),
        "xen-hypervisor".to_string(),
        xenversion.clone(),
        arch.clone(),
        "Unmaintained <unmaintained@example.com>".to_string(),
        vec![
            "libpixman-1-0".to_string(),
            "libpng16-16".to_string(),
            "libnettle6 | libnettle7".to_string(),
            "libgnutls30".to_string(),
            "libfdt1".to_string(),
            "libyajl2".to_string(),
            "libaio1".to_string(),
        ],
        (9..16) // Add additional Xen versions here as they are released
            .map(|v| format!("xen-hypervisor-4.{}-{}", v, &arch))
            .collect(),
        "admin".to_string(),
        "optional".to_string(),
        deb_dir_size as usize,
        "Xen Hypervisor for KF/x".to_string(),
    );

    let deb_control_file = debian_dir.join("control");
    let mut deb_control_file = File::create(&deb_control_file)?;
    deb_control_file.write_all(deb_control.to_string().as_bytes())?;

    write_file(
        &debian_dir.join("control"),
        deb_control.to_string().as_bytes(),
        0o644,
    )?;

    let etc_dir = deb_dir.join("etc");

    let conffiles = WalkDir::new(&etc_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|p| p.file_type().is_file())
        .map(|p| {
            PathBuf::from("/etc")
                .join(p.path().strip_prefix(&etc_dir).unwrap())
                .to_string_lossy()
                .to_string()
        })
        .collect::<Vec<String>>()
        .join("\n")
        + "\n";

    write_file(&debian_dir.join("conffiles"), conffiles.as_bytes(), 0o644)?;

    // Amazingly, fs::chown is still experimental
    check_command(
        Command::new("chown")
            .arg("-R")
            .arg("root:root")
            .arg(&deb_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to run chown")
            .wait_with_output(),
    )?;

    check_command(
        Command::new("dpkg-deb")
            .arg("--build")
            .arg("-z0")
            .arg(&deb_dir)
            .arg(&output_path.join(&deb_name))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to run dpkg-deb")
            .wait_with_output(),
    )
    .map_err(|e| {
        error!("Failed to build deb package: {}", e);
        error!("Deb control file: {}", deb_control.to_string());
        error!("Deb conffiles file: {}", conffiles);
        e
    })?;

    Ok(())
}

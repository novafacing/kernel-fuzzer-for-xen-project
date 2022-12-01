//! Script to configure and install the Xen hypervisor

use std::{
    collections::HashSet,
    error::Error,
    fs::{create_dir_all, read_dir, remove_dir_all, rename, set_permissions, File, Permissions},
    io::Write,
    os::unix::prelude::PermissionsExt,
    path::PathBuf,
    process::{Command, Stdio},
};

use log::{error, info};
use scripts::{
    append_line, check_command, copy_dir, dir_size, get_dpkg_arch, init_logging, read_os_release,
    DebControl,
};

use num_cpus::get as nproc;
use walkdir::WalkDir;

const BASE_CONFIGURE_OPTIONS: &[&str] = &[
    "--enable-systemd",
    "--disable-pvshim",
    "--enable-githttp",
    "--prefix=/usr",
];

fn get_xenversion() -> Result<String, Box<dyn Error>> {
    let boot_dir = PathBuf::from("dist/install/boot");
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

    info!("Configuring Xen with options: {:?}", configure_options);

    check_command(
        Command::new("./configure")
            .args(configure_options)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Could not run configure command")
            .wait_with_output(),
    )?;

    info!("Writing xen/.config");

    let xenconfig_file = PathBuf::from("xen/.config");
    append_line(&xenconfig_file, "CONFIG_EXPERT=y".to_string())?;
    append_line(&xenconfig_file, "CONFIG_MEM_SHARING=y".to_string())?;

    Ok(())
}

fn build_xen() -> Result<(), Box<dyn Error>> {
    info!("Making olddefconfig");
    check_command(
        Command::new("make")
            .current_dir(PathBuf::from("xen"))
            .arg("olddefconfig")
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
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Could not run make install-tools")
            .wait_with_output(),
    )?;

    Ok(())
}

fn make_deb() -> Result<(), Box<dyn Error>> {
    const XEN_CFG_FILE: &[u8] = include_bytes!("../resource/etc/default/grub.d/xen.cfg");
    const XEN_CONF_FILE: &[u8] = include_bytes!("../resource/etc/modules-load.d/xen.conf");
    const KFX_FIND_XEN_DEFAULTS_FILE: &[u8] =
        include_bytes!("../resource/usr/bin/kfx-find-xen-defaults");
    const POSTINST_FILE: &[u8] = include_bytes!("../resource/postinst");
    const POSTRM_FILE: &[u8] = include_bytes!("../resource/postrm");

    let xenversion = get_xenversion()?;

    let distro = read_os_release()?
        .get("ID")
        .expect("No distro id in /etc/os-release")
        .to_lowercase();
    let version = read_os_release()?
        .get("VERSION_CODENAME")
        .expect("No version codename in /etc/os-release")
        .to_string()
        .to_lowercase();

    let deb_out_dir = PathBuf::from("/out");
    if !deb_out_dir.exists() {
        create_dir_all(&deb_out_dir)?;
    }

    let install_dir = PathBuf::from("dist/install");
    let deb_dir = PathBuf::from("/deb");
    // Rename install to deb
    rename(&install_dir, &deb_dir)?;

    // Create the debian directory
    let debian_dir = deb_dir.join("DEBIAN");
    create_dir_all(&debian_dir)?;
    // Create the grub.d and modules-load.d directories
    let grub_dir = deb_dir.join("etc/default/grub.d");
    create_dir_all(&grub_dir)?;
    let modules_dir = deb_dir.join("etc/modules-load.d");
    create_dir_all(&modules_dir)?;

    match distro.as_str() {
        "debian" => {
            let lib64_entries = read_dir(&deb_dir.join("usr/lib64"))?;
            let lib_dir = deb_dir.join("usr/lib");
            copy_dir(&deb_dir.join("usr/lib64"), &lib_dir)?;

            remove_dir_all(&deb_dir.join("usr/lib64"))?;
        }
        _ => {}
    }

    // Write the postinst and postrm files to the debian directory
    let postinst_file = debian_dir.join("postinst");
    let mut postinst = File::create(&postinst_file)?;
    postinst.write_all(POSTINST_FILE)?;
    set_permissions(&postinst_file, Permissions::from_mode(0o755))?;

    let postrm_file = debian_dir.join("postrm");
    let mut postrm = File::create(&postrm_file)?;
    postrm.write_all(POSTRM_FILE)?;
    set_permissions(&postrm_file, Permissions::from_mode(0o755))?;

    // Write the xen.cfg file to the grub.d directory
    let xen_cfg_file = grub_dir.join("xen.cfg");
    let mut xen_cfg_file = File::create(&xen_cfg_file)?;
    xen_cfg_file.write_all(XEN_CFG_FILE)?;
    // Write the xen.conf file to the modules-load.d directory
    let xen_conf_file = modules_dir.join("xen.conf");
    let mut xen_conf_file = File::create(&xen_conf_file)?;
    xen_conf_file.write_all(XEN_CONF_FILE)?;
    // Write the kfx-find-xen-defaults file to the usr/bin directory
    let kfx_find_xen_defaults = deb_dir.join("usr/bin/kfx-find-xen-defaults");
    let mut kfx_find_xen_defaults = File::create(&kfx_find_xen_defaults)?;
    kfx_find_xen_defaults.write_all(KFX_FIND_XEN_DEFAULTS_FILE)?;

    let deb_dir_size = dir_size(&deb_dir)?;
    let deb_dir_size_kb = deb_dir_size / 1024;
    let arch = get_dpkg_arch()?;

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
        deb_dir_size_kb as usize,
        "Xen Hypervisor for KF/x".to_string(),
    );

    let deb_control_file = debian_dir.join("control");
    let mut deb_control_file = File::create(&deb_control_file)?;
    deb_control_file.write_all(deb_control.to_string().as_bytes())?;
    deb_control_file.write_all(b"\n")?;

    let etc_dir = deb_dir.join("etc").canonicalize()?;

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
        .join("\n");

    let deb_conffiles_file = debian_dir.join("conffiles");
    let mut deb_conffiles_file = File::create(&deb_conffiles_file)?;
    deb_conffiles_file.write_all(conffiles.as_bytes())?;
    deb_conffiles_file.write_all(b"\n")?;

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
            .arg(&deb_out_dir.join(format!("xen_{}-{}-{}.deb", &xenversion, &version, &arch)))
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

fn main() -> Result<(), Box<dyn Error>> {
    init_logging()?;
    configure_xen()?;
    build_xen()?;
    make_deb()?;
    Ok(())
}

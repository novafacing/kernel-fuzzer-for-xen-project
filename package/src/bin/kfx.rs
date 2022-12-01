//! Build script for building kfx and its components

use std::{
    collections::HashMap,
    env::var,
    error::Error,
    fs::{copy, create_dir, create_dir_all, remove_dir_all, File},
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

use log::{error, info};
use num_cpus::get as nproc;

use package::{
    check_command, copy_dir, dir_size, get_dpkg_arch, init_logging, read_os_release, DebControl,
};

const INSTALL_PATH: &str = "/kfx/usr";
const DEB_PATH: &str = "/deb";
const OUT_DIR: &str = "/out";

fn build_dwarf2json() -> Result<(), Box<dyn Error>> {
    info!("Building dwarf2json");
    check_command(
        Command::new("/usr/local/go/bin/go")
            .arg("build")
            .current_dir(PathBuf::from("dwarf2json"))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Could not run go build")
            .wait_with_output(),
    )?;

    Ok(())
}

fn build_libvmi() -> Result<(), Box<dyn Error>> {
    info!("Building libvmi");

    let env: HashMap<String, String> = [
        (
            "LD_LIBRARY_PATH",
            format!(
                "{}:{}/usr/lib",
                var("LD_LIBRARY_PATH").unwrap_or("".to_string()),
                DEB_PATH,
            ),
        ),
        ("C_INCLUDE_PATH", format!("{}/usr/include", DEB_PATH)),
        ("CPLUS_INCLUDE_PATH", format!("{}/usr/include", DEB_PATH)),
        ("PKG_CONFIG_PATH", format!("{}/usr/lib/pkgconfig", DEB_PATH)),
        ("LDFLAGS", format!("-L{}/usr/lib", DEB_PATH)),
        ("CFLAGS", format!("-I{}/usr/include", DEB_PATH)),
    ]
    .iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect();

    let libvmi_dir = PathBuf::from("libvmi");
    check_command(
        Command::new("autoreconf")
            .arg("-vif")
            .envs(&env)
            .current_dir(&libvmi_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Could not run autoreconf")
            .wait_with_output(),
    )?;

    check_command(
        Command::new("./configure")
            .arg(format!("--prefix={}", INSTALL_PATH))
            .arg("--disable-kvm")
            .arg("--disable-bareflank")
            .arg("--disable-file")
            .envs(&env)
            .current_dir(&libvmi_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Could not run configure")
            .wait_with_output(),
    )?;

    check_command(
        Command::new("make")
            .arg(format!("-j{}", nproc().to_string()))
            .envs(&env)
            .current_dir(&libvmi_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Could not run make")
            .wait_with_output(),
    )?;

    check_command(
        Command::new("make")
            .arg("install")
            .envs(&env)
            .current_dir(&libvmi_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Could not run make install")
            .wait_with_output(),
    )?;

    check_command(
        Command::new("ldconfig")
            .current_dir(&libvmi_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Could not run ldconfig")
            .wait_with_output(),
    )?;

    Ok(())
}

fn build_capstone() -> Result<(), Box<dyn Error>> {
    info!("Building capstone");

    let capstone_build_dir = PathBuf::from("capstone/build");
    create_dir_all(&capstone_build_dir)?;

    check_command(
        Command::new("cmake")
            .arg(format!("-DCMAKE_INSTALL_PREFIX={}", INSTALL_PATH))
            .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
            .arg("-DCMAKE_BUILD_TYPE=Release")
            .arg("..")
            .current_dir(&capstone_build_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Could not run cmake")
            .wait_with_output(),
    )?;

    check_command(
        Command::new("make")
            .arg(format!("-j{}", nproc().to_string()))
            .current_dir(&capstone_build_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Could not run make")
            .wait_with_output(),
    )?;

    check_command(
        Command::new("make")
            .arg("install")
            .current_dir(&capstone_build_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Could not run make install")
            .wait_with_output(),
    )?;

    Ok(())
}

fn build_libxdc() -> Result<(), Box<dyn Error>> {
    info!("Building libxdc");
    // This one is tricky, because it'll use system capstone if we're not careful
    // We actually need to run *this* monstrosity to get it to link with our capstone built previously
    // make PREFIX="/install" LDFLAGS="-L/install/lib" CFLAGS="-Ofast -fPIC -fvisibility=hidden -flto
    // -finline-functions -I/install/include" install

    let libxdc_dir = PathBuf::from("libxdc");
    check_command(
        Command::new("make")
            .current_dir(&libxdc_dir)
            .arg(format!("PREFIX={}", INSTALL_PATH))
            .arg(format!(
                "CFLAGS=-I{}/include -Ofast -fPIC -fvisibility=hidden -flto -finline-functions",
                INSTALL_PATH
            ))
            .arg(format!("LDFLAGS=-L{}/lib", INSTALL_PATH))
            .arg("install")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Could not run make install")
            .wait_with_output(),
    )?;

    Ok(())
}

fn build_kfx() -> Result<(), Box<dyn Error>> {
    info!("Building kfx");

    let env: HashMap<String, String> = [
        (
            "LD_LIBRARY_PATH",
            format!(
                "{}:{}/lib:{}/usr/lib",
                var("LD_LIBRARY_PATH").unwrap_or("".to_string()),
                INSTALL_PATH,
                DEB_PATH
            ),
        ),
        (
            "C_INCLUDE_PATH",
            format!("{}/include:{}/usr/include", INSTALL_PATH, DEB_PATH),
        ),
        (
            "CPLUS_INCLUDE_PATH",
            format!("{}/include:{}/usr/include", INSTALL_PATH, DEB_PATH),
        ),
        (
            "PKG_CONFIG_PATH",
            format!(
                "{}/lib/pkgconfig:{}/usr/lib/pkgconfig",
                INSTALL_PATH, DEB_PATH
            ),
        ),
        (
            "LDFLAGS",
            format!("-L{}/lib -L{}/usr/lib", INSTALL_PATH, DEB_PATH),
        ),
        (
            "CFLAGS",
            format!("-I{}/include -I{}/usr/include", INSTALL_PATH, DEB_PATH),
        ),
    ]
    .iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect();

    check_command(
        Command::new("autoreconf")
            .arg("-vif")
            .envs(&env)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Could not run autoreconf")
            .wait_with_output(),
    )?;

    check_command(
        Command::new("./configure")
            .arg(format!("--prefix={}", INSTALL_PATH))
            .envs(&env)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Could not run configure")
            .wait_with_output(),
    )?;

    check_command(
        Command::new("make")
            .arg(format!("-j{}", nproc().to_string()))
            .envs(&env)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Could not run make")
            .wait_with_output(),
    )?;

    check_command(
        Command::new("make")
            .arg("install")
            .envs(&env)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Could not run make install")
            .wait_with_output(),
    )?;

    Ok(())
}

fn make_bundle_deb() -> Result<(), Box<dyn Error>> {
    info!("Making deb for kfx bundle");
    let version = var("KFX_VERSION")?;
    let arch = get_dpkg_arch()?;
    let distro = read_os_release()?
        .get("VERSION_CODENAME")
        .expect("No version codename in /etc/os-release")
        .to_string();

    let deb_dir = PathBuf::from(DEB_PATH);
    let usr_dir = deb_dir.join("usr");
    let debian_dir = deb_dir.join("DEBIAN");

    let deb_out_dir = PathBuf::from(OUT_DIR);
    create_dir_all(&deb_out_dir)?;
    let deb_out_file =
        deb_out_dir.join(format!("kfx-bundle_{}-{}-{}.deb", &version, &distro, &arch));

    let install_dir = PathBuf::from(INSTALL_PATH);

    info!("Creating directories for deb");

    copy_dir(&install_dir, &usr_dir)?;

    copy(
        &PathBuf::from("dwarf2json/dwarf2json"),
        &usr_dir.join("bin").join("dwarf2json"),
    )?;

    info!("Done copying files to deb");

    let deb_dir_size = dir_size(&deb_dir)?;

    info!("Deb directory size: {} KB", deb_dir_size);

    let deb_control = DebControl::new(
        "kfx".to_string(),
        "kfx".to_string(),
        version.clone(),
        arch.clone(),
        "Unmaintained <unmaintained@example.com>".to_string(),
        vec![
            "libglib2.0-dev".to_string(),
            "libjson-c3 | libjson-c4 | libjson-c5".to_string(),
            "libpixman-1-0".to_string(),
            "libpng16-16".to_string(),
            "libnettle6 | libnettle7".to_string(),
            "libgnutls30".to_string(),
            "libfdt1".to_string(),
            "libyajl2".to_string(),
            "libaio1".to_string(),
            // Dependencies for kfx packages
            "libc6".to_string(),
            "libfuse2".to_string(),
            "liblzma5".to_string(),
            "libpcre3".to_string(),
            "libunwind8".to_string(),
            "zlib1g".to_string(),
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
    deb_control_file.write_all(b"\n")?;

    info!("Setting permissions for deb");

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

    info!("Creating deb for {} {}", &version, &arch);

    check_command(
        Command::new("dpkg-deb")
            .arg("--build")
            .arg("-z0")
            .arg(&deb_dir)
            .arg(&deb_out_file)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to run dpkg-deb")
            .wait_with_output(),
    )
    .map_err(|e| {
        error!("Failed to build deb package: {}", e);
        error!("Deb control file: {}", deb_control.to_string());
        e
    })?;

    info!("Done! Created deb at {}", deb_out_file.display());

    remove_dir_all(&deb_dir)?;

    Ok(())
}

/// Create a deb package for all KF/x components *except* Xen itself
/// This has to be run after `make_bundle_deb` because it reuses the
/// same directory and expects it to be gone
fn make_kfx_deb() -> Result<(), Box<dyn Error>> {
    info!("Making deb for kfx bundle");
    let version = var("KFX_VERSION")?;
    let arch = get_dpkg_arch()?;
    let distro = read_os_release()?
        .get("VERSION_CODENAME")
        .expect("No version codename in /etc/os-release")
        .to_string();

    let deb_dir = PathBuf::from(DEB_PATH);
    create_dir_all(&deb_dir)?;
    let usr_dir = deb_dir.join("usr");
    create_dir_all(&usr_dir)?;
    let debian_dir = deb_dir.join("DEBIAN");
    create_dir_all(&debian_dir)?;

    let deb_out_dir = PathBuf::from(OUT_DIR);
    create_dir_all(&deb_out_dir)?;
    let deb_out_file = deb_out_dir.join(format!("kfx_{}-{}-{}.deb", &version, &distro, &arch));

    let install_dir = PathBuf::from(INSTALL_PATH);

    info!("Creating directories for deb");

    copy_dir(&install_dir, &usr_dir)?;

    copy(
        &PathBuf::from("dwarf2json/dwarf2json"),
        &usr_dir.join("bin").join("dwarf2json"),
    )?;

    info!("Done copying files to deb");

    let deb_dir_size = dir_size(&deb_dir)?;

    info!("Deb directory size: {} KB", deb_dir_size);

    let deb_control = DebControl::new(
        "kfx".to_string(),
        "kfx".to_string(),
        version.clone(),
        arch.clone(),
        "Unmaintained <unmaintained@example.com>".to_string(),
        vec![
            // Dependencies are:
            // libc.so.6: libc6
            // libcapstone.so.4: provided by this package
            // libfuse.so.2: libfuse2
            // libglib-2.0.so.0: libglib2.0-0
            // libjson-c.so.5: libjson-c3 | libjson-c4 | libjson-c5
            // liblzma.so.5: liblzma5
            // libm.so.6: libc6
            // libpcre.so.3: libpcre3
            // libunwind-x86_64.so.8: libunwind8
            // libunwind.so.8: libunwind8
            // libvmi.so.0: provided by this package
            // libxenctrl.so.4.16: provided by the xen package or bundle version
            // libxenforeignmemory.so.1: provided by the xen package or bundle version
            // libxenlight.so.4.16: provided by the xen package or bundle version
            // libxenstore.so.4: provided by the xen package or bundle version
            // libz.so.1: zlib1g
            // linux-vdso.so.1: provided by the kernel
            "libc6".to_string(),
            "libfuse2".to_string(),
            "libglib2.0-0".to_string(),
            "libjson-c3 | libjson-c4 | libjson-c5".to_string(),
            "liblzma5".to_string(),
            "libpcre3".to_string(),
            "libunwind8".to_string(),
            "zlib1g".to_string(),
        ],
        vec![],
        "admin".to_string(),
        "optional".to_string(),
        deb_dir_size as usize,
        "Xen Hypervisor for KF/x".to_string(),
    );

    let deb_control_file = debian_dir.join("control");
    let mut deb_control_file = File::create(&deb_control_file)?;
    deb_control_file.write_all(deb_control.to_string().as_bytes())?;
    deb_control_file.write_all(b"\n")?;

    info!("Setting permissions for deb");

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

    info!("Creating deb for {} {}", &version, &arch);

    check_command(
        Command::new("dpkg-deb")
            .arg("--build")
            .arg("-z0")
            .arg(&deb_dir)
            .arg(&deb_out_file)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to run dpkg-deb")
            .wait_with_output(),
    )
    .map_err(|e| {
        error!("Failed to build deb package: {}", e);
        error!("Deb control file: {}", deb_control.to_string());
        e
    })?;

    info!("Done! Created deb at {}", deb_out_file.display());

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    init_logging()?;
    build_dwarf2json()?;
    build_libvmi()?;
    build_capstone()?;
    build_libxdc()?;
    build_kfx()?;
    make_bundle_deb()?;
    make_kfx_deb()?;
    Ok(())
}

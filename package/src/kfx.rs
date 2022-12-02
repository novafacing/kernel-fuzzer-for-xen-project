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
use tempdir::TempDir;

use crate::{
    check_command, copy_dir, dir_size, get_dpkg_arch, get_version, init_logging, read_os_release,
    unpack_deb, write_file, DebControl,
};

pub fn build_dwarf2json(kfx_path: &PathBuf) -> Result<(), Box<dyn Error>> {
    info!("Building dwarf2json");
    check_command(
        Command::new("/usr/local/go/bin/go")
            .arg("build")
            .current_dir(kfx_path.join("dwarf2json"))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Could not run go build")
            .wait_with_output(),
    )?;

    Ok(())
}

pub fn build_libvmi(kfx_path: &PathBuf, build_path: &PathBuf) -> Result<(), Box<dyn Error>> {
    let libvmi_dir = kfx_path.join("libvmi");

    info!("Building libvmi");

    // Libvmi just needs include paths for:
    // * xenctrl.h: tools/include/
    // * xen/hvm/save.h: xen/include/public/
    // * xen/io/ring.h: xen/include/public/
    // * xen/memory.h: xen/include/public/
    // * xenstore.h: tools/include/
    // * xs.h: tools/include/xenstore-compat/
    let libvmi_include_paths = vec![
        "tools/include/",
        "xen/include/public",
        "tools/include/xenstore-compat/",
    ]
    .iter()
    .map(|p| {
        let path = kfx_path.join(p);
        path.to_string_lossy().to_string()
    })
    .collect::<Vec<_>>();

    let env: HashMap<String, String> = [
        ("C_INCLUDE_PATH", &libvmi_include_paths.join(":")),
        ("CPLUS_INCLUDE_PATH", &libvmi_include_paths.join(":")),
        ("CPLUS_INCLUDE_PATH", &libvmi_include_paths.join(":")),
        (
            "CFLAGS",
            &libvmi_include_paths
                .iter()
                .map(|p| format!("-I{}", p))
                .collect::<Vec<_>>()
                .join(" "),
        ),
        (
            "CXXFLAGS",
            &libvmi_include_paths
                .iter()
                .map(|p| format!("-I{}", p))
                .collect::<Vec<_>>()
                .join(" "),
        ),
    ]
    .iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect();

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
            .arg(format!("--prefix={}", build_path.to_string_lossy()))
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

pub fn build_capstone(kfx_path: &PathBuf, build_path: &PathBuf) -> Result<(), Box<dyn Error>> {
    info!("Building capstone");

    let capstone_build_dir = kfx_path.join("capstone/build");
    create_dir_all(&capstone_build_dir)?;

    check_command(
        Command::new("cmake")
            .arg(format!(
                "-DCMAKE_INSTALL_PREFIX={}",
                build_path.to_string_lossy()
            ))
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

pub fn build_libxdc(kfx_path: &PathBuf, build_path: &PathBuf) -> Result<(), Box<dyn Error>> {
    info!("Building libxdc");
    // This one is tricky, because it'll use system capstone if we're not careful
    // We actually need to run *this* monstrosity to get it to link with our capstone built previously
    // make PREFIX="/install" LDFLAGS="-L/install/lib" CFLAGS="-Ofast -fPIC -fvisibility=hidden -flto
    // -finline-functions -I/install/include" install

    let libxdc_dir = kfx_path.join("libxdc");
    check_command(
        Command::new("make")
            .arg(format!("PREFIX={}", build_path.to_string_lossy()))
            .arg(format!(
                "CFLAGS=-I{}/include -Ofast -fPIC -fvisibility=hidden -flto -finline-functions",
                build_path.to_string_lossy()
            ))
            .arg(format!("LDFLAGS=-L{}/lib", build_path.to_string_lossy()))
            .arg("install")
            .current_dir(&libxdc_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Could not run make install")
            .wait_with_output(),
    )?;

    Ok(())
}

pub fn build_kfx(kfx_path: &PathBuf, build_path: &PathBuf) -> Result<(), Box<dyn Error>> {
    info!("Building kfx");

    // KF/x just needs the following includes:
    // * xenctrl.h: xen/include/public/
    // * xenforeignmemory.h: tools/include
    // * xenstore.h: tools/include/
    // * xen/xen.h: xen/include/public/
    // * xs.h: tools/include/xenstore-compat/
    let kfx_include_paths = vec![
        "tools/include/",
        "xen/include/public",
        "tools/include/xenstore-compat/",
    ]
    .iter()
    .map(|p| {
        let path = kfx_path.join(p);
        path.to_string_lossy().to_string()
    })
    .collect::<Vec<_>>();

    let env: HashMap<String, String> = [
        (
            "LD_LIBRARY_PATH",
            format!(
                "{}:{}",
                var("LD_LIBRARY_PATH").unwrap_or("".to_string()),
                build_path.join("lib").to_string_lossy(),
            ),
        ),
        (
            "C_INCLUDE_PATH",
            format!(
                "{}/include:{}",
                build_path.join("include").to_string_lossy(),
                &kfx_include_paths.join(":")
            ),
        ),
        (
            "CPLUS_INCLUDE_PATH",
            format!(
                "{}/include:{}",
                build_path.join("include").to_string_lossy(),
                &kfx_include_paths.join(":")
            ),
        ),
        (
            "PKG_CONFIG_PATH",
            build_path
                .join("lib/pkgconfig")
                .to_string_lossy()
                .to_string(),
        ),
        (
            "LDFLAGS",
            format!("-L{}/lib", build_path.join("lib").to_string_lossy()),
        ),
        (
            "CFLAGS",
            format!("-I{}/include", build_path.join("include").to_string_lossy()),
        ),
    ]
    .iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect();

    check_command(
        Command::new("autoreconf")
            .arg("-vif")
            .envs(&env)
            .current_dir(&kfx_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Could not run autoreconf")
            .wait_with_output(),
    )?;

    check_command(
        Command::new("./configure")
            .arg(format!("--prefix={}", build_path.to_string_lossy()))
            .envs(&env)
            .current_dir(&kfx_path)
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
            .current_dir(&kfx_path)
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
            .current_dir(&kfx_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Could not run make install")
            .wait_with_output(),
    )?;

    Ok(())
}

pub fn make_bundle_deb(
    output_path: &PathBuf,
    build_path: &PathBuf,
    xen_deb_path: &PathBuf,
) -> Result<(), Box<dyn Error>> {
    info!("Making deb for kfx bundle");
    let kfx_version = var("KFX_VERSION")?;
    let arch = get_dpkg_arch()?;
    let distro_version = get_version()?;

    let deb_name = format!(
        "kfx-bundle_{}-{}-{}.deb",
        &kfx_version, &distro_version, &arch
    );

    let tmpdir = TempDir::new("deb")?;
    let deb_dir = tmpdir.path().to_path_buf();
    unpack_deb(&xen_deb_path, &deb_dir)?;

    let usr_dir = deb_dir.join("usr");
    let debian_dir = deb_dir.join("DEBIAN");

    info!("Creating directories for deb");

    copy_dir(&build_path, &usr_dir)?;

    copy(
        &build_path.join("dwarf2json/dwarf2json"),
        &usr_dir.join("bin").join("dwarf2json"),
    )?;

    info!("Done copying files to deb");

    let deb_dir_size = dir_size(&deb_dir)?;

    info!("Deb directory size: {} KB", deb_dir_size);

    let mut deb_control = DebControl::from_file(&debian_dir.join("control"))?;
    deb_control.package = "kfx-bundle".to_string();
    deb_control.source = "kfx-bundle".to_string();
    deb_control.version = kfx_version.clone();
    deb_control.depends.extend(vec![
        // Dependencies for kfx packages
        "libc6".to_string(),
        "libfuse2".to_string(),
        "liblzma5".to_string(),
        "libpcre3".to_string(),
        "libunwind8".to_string(),
        "zlib1g".to_string(),
    ]);
    deb_control.installed_size = deb_dir_size;

    write_file(
        &debian_dir.join("control"),
        deb_control.to_string().as_bytes(),
        0o644,
    )?;

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

    info!("Creating deb for {} {}", &kfx_version, &arch);

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
        e
    })?;

    info!(
        "Done! Created deb at {}",
        output_path.join(deb_name).display()
    );

    remove_dir_all(&deb_dir)?;

    Ok(())
}

/// Create a deb package for all KF/x components *except* Xen itself
/// This has to be run after `make_bundle_deb` because it reuses the
/// same directory and expects it to be gone
pub fn make_kfx_deb(output_path: &PathBuf, build_path: &PathBuf) -> Result<(), Box<dyn Error>> {
    info!("Making deb for kfx bundle");
    let kfx_version = var("KFX_VERSION")?;
    let arch = get_dpkg_arch()?;
    let distro_version = get_version()?;

    let deb_name = format!("kfx_{}-{}-{}.deb", &kfx_version, &distro_version, &arch);

    let tmpdir = TempDir::new("deb")?;

    let deb_dir = tmpdir.path().to_path_buf();
    let usr_dir = deb_dir.join("usr");
    let debian_dir = deb_dir.join("DEBIAN");

    create_dir_all(&deb_dir)?;
    create_dir_all(&usr_dir)?;
    create_dir_all(&debian_dir)?;

    info!("Creating directories for deb");

    copy_dir(&build_path, &usr_dir)?;

    copy(
        &build_path.join("dwarf2json/dwarf2json"),
        &usr_dir.join("bin").join("dwarf2json"),
    )?;

    info!("Done copying files to deb");

    let deb_dir_size = dir_size(&deb_dir)?;

    info!("Deb directory size: {} KB", deb_dir_size);

    let deb_control = DebControl::new(
        "kfx".to_string(),
        "kfx".to_string(),
        kfx_version.clone(),
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

    info!("Creating deb for {} {}", &kfx_version, &arch);

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
        e
    })?;

    info!(
        "Done! Created deb at {}",
        output_path.join(deb_name).display()
    );

    Ok(())
}

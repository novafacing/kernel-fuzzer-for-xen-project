//! Script to install dependencies for building:
//! * Xen
//! * AFL
//! * dwarf2json
//! * libvmi
//! * capstone
//! * libxdc
//! * KF/x

use std::{
    collections::HashSet,
    env::temp_dir,
    error::Error,
    path::PathBuf,
    process::{Command, Stdio},
};

use scripts::{append_line, download, read_os_release, replace_text, unpack_tgz};

/// List of base dependencies for KF/x install. Some distros may add or
/// remove from this list depending on their available packages.
const BASE_DEPENDENCIES: &[&str] = &[
    "autoconf",
    "autoconf",
    "autoconf-archive",
    "automake",
    "bc",
    "bcc",
    "bin86",
    "binutils",
    "bison",
    "bridge-utils",
    "build-essential",
    "bzip2",
    "cabextract",
    "cmake",
    "e2fslibs-dev",
    "flex",
    "gawk",
    "gcc-multilib",
    "gettext",
    "git",
    "iasl",
    "iproute2",
    "kpartx",
    "libaio-dev",
    "libbz2-dev",
    "libc6-dev",
    "libc6-dev-i386",
    "libcurl4-openssl-dev",
    "libfdt-dev",
    "libfuse-dev",
    "libglib2.0-dev",
    "libgnutls28-dev",
    "libjson-c-dev",
    "liblzma-dev",
    "libncurses5-dev",
    "libpci-dev",
    "libpixman-1-dev",
    "libsdl-dev",
    "libsdl1.2-dev",
    "libssl-dev",
    "libsystemd-dev",
    "libtool",
    "libunwind-dev",
    "libvncserver-dev",
    "libx11-dev",
    "libyajl-dev",
    "linux-libc-dev",
    "nasm",
    "ninja-build",
    "ocaml",
    "ocaml-findlib",
    "patch",
    "python3-dev",
    "python3-pip",
    "snap",
    "tightvncserver",
    "uuid-dev",
    "uuid-runtime",
    "wget",
    "x11vnc",
    "xtightvncviewer",
    "xz-utils",
    "zlib1g-dev",
];

/// Check if this distro has a `python-is-python2` package
fn has_python_is_python2() -> Result<bool, Box<dyn Error>> {
    Ok(String::from_utf8_lossy(
        &Command::new("apt-cache")
            .arg("search")
            .arg("--names-only")
            .arg("^python-is-python2$")
            .stdout(Stdio::piped())
            .spawn()
            .expect("Could not run apt-cache command")
            .wait_with_output()
            .expect("apt-cache command failed")
            .stdout,
    )
    .to_lowercase()
    .contains("python-is-python2"))
}

/// Run the apt install process including autoremove and clean to reduce image size
fn run_apt(dependencies: &HashSet<String>) -> Result<(), Box<dyn Error>> {
    Command::new("apt-get")
        .arg("-y")
        .arg("update")
        .spawn()
        .expect("Failed to run apt-get update")
        .wait()
        .expect("apt-get update failed");

    Command::new("apt-get")
        .arg("-y")
        .arg("install")
        .args(dependencies)
        .spawn()
        .expect("Failed to run apt-get install")
        .wait()
        .expect("apt-get install failed");

    Command::new("apt-get")
        .arg("-y")
        .arg("build-dep")
        .arg("xen")
        .spawn()
        .expect("Failed to run apt-get build-dep")
        .wait()
        .expect("apt-get build-dep failed");

    Command::new("apt-get")
        .arg("-y")
        .arg("autoremove")
        .spawn()
        .expect("Failed to run apt-get autoremove")
        .wait()
        .expect("apt-get autoremove failed");

    Command::new("apt-get")
        .arg("-y")
        .arg("clean")
        .spawn()
        .expect("Failed to run apt-get clean")
        .wait()
        .expect("apt-get clean failed");

    Ok(())
}

/// Configure apt dependencies for the current distro and install them
fn install_apt_deps() -> Result<(), Box<dyn Error>> {
    let os_release = read_os_release()?;
    let distro = os_release
        .get("ID")
        .expect("No distro in os release file.")
        .to_lowercase();
    let version = os_release
        .get("VERSION_CODENAME")
        .expect("No version codename in os release file.")
        .to_lowercase();

    let mut dependencies: HashSet<String> =
        BASE_DEPENDENCIES.iter().map(|d| d.to_string()).collect();

    match (distro.as_str(), version.as_str()) {
        ("debian", _) => {
            append_line(
                PathBuf::from("/etc/apt/sources.list"),
                format!("deb-src http://deb.debian.org/debian {} main", version),
            )?;
        }
        ("ubuntu", "jammy") => {
            replace_text(
                PathBuf::from("/etc/apt/sources.list"),
                "# deb-src",
                "deb-src",
            )?;
        }
        ("ubuntu", _) => {
            replace_text(
                PathBuf::from("/etc/apt/sources.list"),
                "# deb-src",
                "deb-src",
            )?;
            dependencies.remove("libsdl-dev");
        }
        _ => {}
    }

    if has_python_is_python2()? {
        dependencies.insert("python-is-python2".to_string());
    }

    run_apt(&dependencies)?;

    Ok(())
}

/// Download and unpack golang tarball
fn install_golang() -> Result<(), Box<dyn Error>> {
    const GO_URL: &str = "https://golang.org/dl/go1.15.3.linux-amd64.tar.gz";
    let go_file = temp_dir().join("go.tar.gz");
    download(GO_URL, go_file.clone())?;
    unpack_tgz(go_file, PathBuf::from("/usr/local"))?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    install_apt_deps()?;
    install_golang()?;
    Ok(())
}

//! Script to create an autounattend ISO from a windows ISO and an autounattend.xml
//! file

use std::{
    fs::copy,
    path::PathBuf,
    process::{Command, Stdio},
};

use clap::Parser;
use log::{error, info};
use sys_mount::{Mount, MountFlags, SupportedFilesystems, Unmount, UnmountFlags};
use tempfile::tempdir;
use xltools::{check_command, logging_config, util::fs::copy_dir};

#[derive(Parser)]
struct Args {
    /// The path to the ISO file (should be a default ISO from Microsoft)
    pub iso: PathBuf,
    /// The path to the autounattend.xml file (it will be renamed if not named this)
    pub answerfile: PathBuf,
    /// The path to output the new windows ISO
    pub output: PathBuf,
}

fn main() {
    let args = Args::parse();

    logging_config().expect("Could not configure logging");

    if !args.iso.exists() {
        error!("Input ISO {} does not exist.", args.iso.to_string_lossy());
        return;
    }

    if !args.answerfile.exists() {
        error!(
            "Input answer file {} does not exist.",
            args.answerfile.to_string_lossy()
        );
        return;
    }

    if args.output.exists() {
        error!(
            "Refusing to overwrite existing output file {}",
            args.output.to_string_lossy()
        );
        return;
    }

    let mountdir = tempdir().expect("Could not create mount directory");
    let extdir = tempdir().expect("Could not create extracted ISO directory.");

    let supported = SupportedFilesystems::new().expect("Could not get supported filesystems");

    info!(
        "Mounting '{}' to '{}'",
        args.iso.to_string_lossy(),
        mountdir.path().to_string_lossy()
    );

    let mount = Mount::new(args.iso, &mountdir, &supported, MountFlags::empty(), None)
        .expect("Failed to mount ISO");

    info!("Mounted ISO, setting to unmount on drop");

    let mount = mount.into_unmount_drop(UnmountFlags::DETACH);

    copy_dir(&mountdir.path().to_path_buf(), &extdir.path().to_path_buf())
        .expect("Failed to copy files to extracted directory");

    drop(mount);

    copy(args.answerfile, extdir.path().join("autounattend.xml"))
        .expect("Could not copy autounattend.xml");

    let proc = Command::new("mkisofs")
        .arg("-bboot/etfsboot.com")
        .arg("-no-emul-boot")
        .arg("-boot-load-seg")
        .arg("1984")
        .arg("-boot-load-size")
        .arg("8")
        .arg("-iso-level")
        .arg("2")
        .arg("-J")
        .arg("-l")
        .arg("-D")
        .arg("-N")
        .arg("-joliet-long")
        .arg("-allow-limited-size")
        .arg("-relaxed-filenames")
        .arg("-V")
        .arg(r#""WIN10""#)
        .arg("-o")
        .arg(args.output)
        .arg(extdir.into_path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Could not spawn mkisofs");

    check_command(proc.wait_with_output()).expect("mkisofs command failed");
}

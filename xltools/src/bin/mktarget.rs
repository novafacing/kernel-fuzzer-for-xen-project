//! This script creates a Windows dev machine

use std::path::PathBuf;

use anyhow::{bail, Result};
use clap::Parser;
use log::info;
use openssh::Stdio;
use tokio;

use xltools::{
    checkroot, dom_mac, logging_config, new_domnaname, new_img,
    ssh::ssh_domname,
    xen::xlcfg::{
        XlCfg, XlCfgBuilder, XlDiskCfgBuilder, XlDiskFormat, XlDiskVdev, XlGuestType,
        XlNetCfgBuilder, XlSerialDev, XlVgaDev,
    },
    xen::{
        xl::{create, list},
        xs::dom_disks,
    },
};

const WINDEV_VMNAME: &str = "wintgt";
const WINDEV_IMG_SIZE: u64 = 25;

#[derive(Parser)]
struct Args {
    /// The username to log in to the windows machine remotely
    pub user: String,
    /// The password to log in to the windows machine remotely
    pub password: String,
    /// The path to save the image to
    pub img: PathBuf,
    /// An ISO file that will automatically install windows and start an SSH
    /// server on the windows machine. If not provided and `img` is, the existing image
    /// will be started.
    pub iso: Option<PathBuf>,
}

fn make_cfg(iso: Option<PathBuf>, img: PathBuf) -> Result<XlCfg> {
    let name = new_domnaname(WINDEV_VMNAME.to_string())?;
    let mut disks = Vec::new();
    if let Some(iso) = iso {
        if !iso.exists() {
            bail!("ISO file does not exist!");
        }
        disks.push(
            XlDiskCfgBuilder::default()
                .target(iso)
                .format(XlDiskFormat::Raw)
                .cdrom(true)
                .vdev(XlDiskVdev::Hd("c".to_string()))
                .build()
                .unwrap(),
        );
    }

    let img = match img.exists() {
        true => img,
        false => new_img(img, WINDEV_IMG_SIZE)?,
    };

    disks.push(
        XlDiskCfgBuilder::default()
            .target(img)
            .format(XlDiskFormat::Raw)
            .vdev(XlDiskVdev::Xvd("a".to_string()))
            .build()
            .unwrap(),
    );

    let cfg = XlCfgBuilder::default()
        .name(name)
        .type_(XlGuestType::HVM)
        .memory(4096)
        .vcpus(1)
        .vga(XlVgaDev::StdVga)
        .videoram(32u32)
        .serial(XlSerialDev::Pty)
        .vif(vec![XlNetCfgBuilder::default()
            .bridge("xenbr0")
            .build()
            .unwrap()])
        .disk(disks)
        // .vnc(true)
        // .vnclisten(XlVncAddr::new(Ipv4Addr::new(0, 0, 0, 0), 5900))
        // Set to 64MB, we likely do not need anywhere near that much space though
        .vm_trace_buf(64u64 * 1024u64)
        .build()?;

    Ok(cfg)
}

fn vm_using_img(img: PathBuf) -> Result<Option<String>> {
    Ok(list()?
        .iter()
        .map(|li| li.name.clone())
        .filter_map(|name| dom_disks(&name).ok().map(|disks| (name, disks)))
        .filter_map(|(name, disks)| {
            img.canonicalize()
                .ok()
                .map(|p| (name, p.to_string_lossy().to_string()))
                .filter(|(_name, p)| disks.contains(&p))
                .map(|(name, _p)| name)
        })
        .take(1)
        .next())
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    logging_config().expect("Could not configure logging!");
    checkroot().expect("Must be run as root!");
    let cfg = make_cfg(args.iso, args.img.clone()).expect("Unable to create config.");
    let name = cfg.name.clone();
    // Check if we already have a vm using the given image
    let name = match vm_using_img(args.img).expect("Could not find vm using img") {
        Some(vm) => vm,
        None => {
            info!("Creating new VM");
            create(cfg).expect("Unable to create VM");
            name
        }
    };

    for mac in dom_mac(&name).await.unwrap() {
        println!("{}", mac.to_string());
    }

    //let ssh = ssh_domname(name, 22, 600, args.user, args.password)
    //    .await
    //    .expect("Unable to connect to VM");

    //let child = ssh
    //    .raw_command("powershell whoami")
    //    .stdout(Stdio::piped())
    //    .stderr(Stdio::piped())
    //    .spawn()
    //    .await
    //    .expect("Could not execute command");
    //let result = child.wait_with_output().await.expect("Command failed");
    //println!("whoami: {}", String::from_utf8_lossy(&result.stdout));
}

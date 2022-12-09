//! Define preset machine configurations and bringup sequences

use std::net::Ipv4Addr;
use std::{error::Error, path::PathBuf};

use crate::xl::create;
use crate::xlcfg::{
    XlCfgBuilder, XlDiskCfgBuilder, XlDiskFormat, XlDiskVdev, XlGuestType, XlNetCfgBuilder,
    XlSerialDev, XlVgaDev,
};
use crate::{new_domnaname, new_img, next_vnc_port};

const WINDEV_VMNAME: &str = "windev";

/// Defines a windows dev machine with:
pub fn windows_dev(
    auto_iso: PathBuf,
    img: PathBuf,
    username: String,
    password: String,
) -> Result<(), Box<dyn Error>> {
    let name = new_domnaname(WINDEV_VMNAME.to_string())?;
    let cfg = XlCfgBuilder::default()
        .name(name)
        .type_(XlGuestType::HVM)
        .memory(4096)
        .vcpus(2)
        .vga(XlVgaDev::StdVga)
        .videoram(32u32)
        .serial(XlSerialDev::Pty)
        .vif(vec![XlNetCfgBuilder::default()
            .bridge("xenbr0")
            .build()
            .unwrap()])
        .disk(vec![
            XlDiskCfgBuilder::default()
                .target(auto_iso)
                .format(XlDiskFormat::Raw)
                .cdrom(true)
                .vdev(XlDiskVdev::Hd("c".to_string()))
                .build()
                .unwrap(),
            XlDiskCfgBuilder::default()
                .target(new_img(img, 40)?)
                .format(XlDiskFormat::Raw)
                .vdev(XlDiskVdev::Xvd("a".to_string()))
                .build()
                .unwrap(),
        ])
        .vnc(true)
        .vnclisten((Ipv4Addr::new(0, 0, 0, 0), next_vnc_port()?))
        .build()?;

    create(cfg)?;

    Ok(())
}

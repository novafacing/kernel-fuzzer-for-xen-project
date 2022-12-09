//! This module implements a subset of the Xl command set as wrappers to the CLI
//! program to conveniently access the functionality from code

use std::{
    collections::HashSet, error::Error, io::BufRead, path::PathBuf, process::Command, str::FromStr,
};

use log::error;
use macaddr::MacAddr6;

use crate::{check_command, xlcfg::XlCfg};

pub fn create(cfg: XlCfg) -> Result<(), Box<dyn Error>> {
    check_command(
        Command::new("xl")
            .arg("create")
            .arg(cfg.to_string())
            .spawn()
            .expect("Failed to spawn xl create")
            .wait_with_output(),
    )?;
    Ok(())
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum XlDomainState {
    Running,
    Blocked,
    Paused,
    Shutdown,
    Crashed,
    Dying,
}

impl XlDomainState {
    pub fn from_str(s: &str) -> Result<XlDomainState, Box<dyn Error>> {
        match s {
            "r" => Ok(XlDomainState::Running),
            "b" => Ok(XlDomainState::Blocked),
            "p" => Ok(XlDomainState::Paused),
            "s" => Ok(XlDomainState::Shutdown),
            "c" => Ok(XlDomainState::Crashed),
            "d" => Ok(XlDomainState::Dying),
            _ => Err("Unknown domain state")?,
        }
    }
}

pub struct XlListInfo {
    pub name: String,
    pub id: u32,
    pub mem: u32,
    pub vcpus: u32,
    pub state: HashSet<XlDomainState>,
    pub time: f32,
}

impl FromStr for XlListInfo {
    type Err = Box<dyn Error>;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split_whitespace();
        Ok(XlListInfo {
            name: parts.next().ok_or("Missing name")?.to_string(),
            id: parts.next().ok_or("Missing id")?.parse()?,
            mem: parts.next().ok_or("Missing mem")?.parse()?,
            vcpus: parts.next().ok_or("Missing vcpus")?.parse()?,
            state: parts
                .next()
                .ok_or("Missing state")?
                .chars()
                .filter_map(|c| match c {
                    '-' => None,
                    _ => Some(XlDomainState::from_str(&c.to_string()).unwrap()),
                })
                .collect(),
            time: parts.next().ok_or("Missing time")?.parse()?,
        })
    }
}
pub fn list() -> Result<Vec<XlListInfo>, Box<dyn Error>> {
    let output = check_command(
        Command::new("xl")
            .arg("list")
            .spawn()
            .expect("Failed to spawn xl list")
            .wait_with_output(),
    )?;
    output
        .stdout
        .lines()
        .skip(1)
        .filter_map(|l| match l {
            Ok(s) => Some(XlListInfo::from_str(s.as_str())),
            Err(e) => {
                error!("Error parsing xl list output: {}", e);
                None
            }
        })
        .collect()
}
pub fn destroy(domid: u32) -> Result<(), Box<dyn Error>> {
    check_command(
        Command::new("xl")
            .arg("destroy")
            .arg(domid.to_string())
            .spawn()
            .expect("Failed to spawn xl destroy")
            .wait_with_output(),
    )
    .map(|_| ())
}

pub fn domid(domname: String) -> Result<u32, Box<dyn Error>> {
    check_command(
        Command::new("xl")
            .arg("domid")
            .arg(domname)
            .spawn()
            .expect("Failed to spawn xl domid")
            .wait_with_output(),
    )
    .map(|o| String::from_utf8(o.stdout).unwrap().trim().parse().unwrap())
}
pub fn domname(domid: u32) -> Result<String, Box<dyn Error>> {
    check_command(
        Command::new("xl")
            .arg("domname")
            .arg(domid.to_string())
            .spawn()
            .expect("Failed to spawn xl domname")
            .wait_with_output(),
    )
    .map(|o| String::from_utf8(o.stdout).unwrap().trim().to_string())
}
pub fn rename(domid: u32, name: String) -> Result<(), Box<dyn Error>> {
    check_command(
        Command::new("xl")
            .arg("rename")
            .arg(domid.to_string())
            .arg(name)
            .spawn()
            .expect("Failed to spawn xl rename")
            .wait_with_output(),
    )
    .map(|_| ())
}
pub fn dump_core(domid: u32, filename: String) -> Result<(), Box<dyn Error>> {
    check_command(
        Command::new("xl")
            .arg("dump-core")
            .arg(domid.to_string())
            .arg(filename)
            .spawn()
            .expect("Failed to spawn xl dump-core")
            .wait_with_output(),
    )
    .map(|_| ())
}
pub fn pause(domid: u32) -> Result<(), Box<dyn Error>> {
    check_command(
        Command::new("xl")
            .arg("pause")
            .arg(domid.to_string())
            .spawn()
            .expect("Failed to spawn xl pause")
            .wait_with_output(),
    )
    .map(|_| ())
}
pub fn reboot(domid: u32, force: bool) -> Result<(), Box<dyn Error>> {
    check_command(
        Command::new("xl")
            .arg("reboot")
            .arg(if force { "-F" } else { "" })
            .arg(domid.to_string())
            .spawn()
            .expect("Failed to spawn xl reboot")
            .wait_with_output(),
    )
    .map(|_| ())
}
pub fn save(
    domid: u32,
    stay_running: bool,
    pause: bool,
    checkpoint_file: PathBuf,
    config_file: Option<PathBuf>,
) -> Result<(), Box<dyn Error>> {
    let mut args = Vec::new();
    args.push("save".to_string());
    if stay_running {
        args.push("-c".to_string());
    }
    if pause {
        args.push("-p".to_string());
    }
    let domid = domid.to_string();
    args.push(domid);
    let checkpoint_file = checkpoint_file.to_string_lossy().to_string();
    args.push(checkpoint_file);
    if let Some(config_file) = config_file {
        let config_file = config_file.to_string_lossy().to_string();
        args.push(config_file);
    }
    check_command(
        Command::new("xl")
            .args(args)
            .spawn()
            .expect("Failed to spawn xl save")
            .wait_with_output(),
    )
    .map(|_| ())
}
pub fn restore(
    pause: bool,
    checkpoint_file: PathBuf,
    config_file: Option<PathBuf>,
) -> Result<(), Box<dyn Error>> {
    let mut args = Vec::new();
    args.push("restore".to_string());
    if pause {
        args.push("-p".to_string());
    }
    if let Some(config_file) = config_file {
        args.push(config_file.to_string_lossy().to_string());
    }
    args.push(checkpoint_file.to_string_lossy().to_string());
    check_command(
        Command::new("xl")
            .args(args)
            .spawn()
            .expect("Failed to spawn xl restore")
            .wait_with_output(),
    )
    .map(|_| ())
}

pub enum XlShutdownTarget {
    All,
    DomId(u32),
}

pub fn shutdown(system: XlShutdownTarget, wait: bool, force: bool) -> Result<(), Box<dyn Error>> {
    let mut args = Vec::new();
    if wait {
        args.push("-w".to_string());
    }
    if force {
        args.push("-F".to_string());
    }
    args.push(match system {
        XlShutdownTarget::All => "-a".to_string(),
        XlShutdownTarget::DomId(domid) => domid.to_string(),
    });
    check_command(
        Command::new("xl")
            .args(args)
            .spawn()
            .expect("Failed to spawn xl shutdown")
            .wait_with_output(),
    )
    .map(|_| ())
}
pub fn unpause(domid: u32) -> Result<(), Box<dyn Error>> {
    check_command(
        Command::new("xl")
            .arg("unpause")
            .arg(domid.to_string())
            .spawn()
            .expect("Failed to spawn xl unpause")
            .wait_with_output(),
    )
    .map(|_| ())
}

pub struct XlNetworkListEntry {
    pub idx: i32,
    pub be: i32,
    pub mac: MacAddr6,
    pub handle: i32,
    pub state: i32,
    pub evt_ch: i32,
    pub tx: i32,
    pub rx: i32,
    pub be_path: String,
}

impl FromStr for XlNetworkListEntry {
    type Err = Box<dyn Error>;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split_whitespace();
        Ok(XlNetworkListEntry {
            idx: parts.next().unwrap().parse()?,
            be: parts.next().unwrap().parse()?,
            mac: parts.next().unwrap().parse()?,
            handle: parts.next().unwrap().parse()?,
            state: parts.next().unwrap().parse()?,
            evt_ch: parts.next().unwrap().parse()?,
            tx: parts.next().unwrap().parse()?,
            rx: parts.next().unwrap().parse()?,
            be_path: parts.next().unwrap().to_string(),
        })
    }
}

pub fn network_list(domid: u32) -> Result<Vec<XlNetworkListEntry>, Box<dyn Error>> {
    check_command(
        Command::new("xl")
            .arg("network-list")
            .arg(domid.to_string())
            .spawn()
            .expect("Failed to spawn xl network-list")
            .wait_with_output(),
    )
    .map(|o| {
        o.stdout
            .lines()
            .skip(1)
            .filter_map(|l| match l {
                Ok(l) => Some(l.parse().unwrap()),
                Err(_) => {
                    error!("Failed to parse network-list output");
                    None
                }
            })
            .collect()
    })
}

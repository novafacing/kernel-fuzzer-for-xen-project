//! This module implements a subset of the Xl command set as wrappers to the CLI
//! program to conveniently access the functionality from code

use std::{
    collections::HashSet,
    io::{BufRead, Write},
    path::PathBuf,
    process::{Command, Stdio},
    str::FromStr,
};

use anyhow::{bail, Context, Error, Result};
use log::error;
use macaddr::MacAddr6;
use tempfile::NamedTempFile;

use crate::{check_command, xen::xlcfg::XlCfg};

pub fn create(cfg: XlCfg) -> Result<()> {
    // We need to create a dummy config file
    let mut tmp_path = NamedTempFile::new()?;
    // Make it empty
    write!(tmp_path, "")?;

    check_command(
        Command::new("xl")
            .arg("create")
            .arg(tmp_path.path())
            .arg(cfg.to_string())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to spawn xl create")
            .wait_with_output(),
    )?;

    // Temp file will be dropped and deleted here
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
    pub fn from_str(s: &str) -> Result<XlDomainState> {
        match s {
            "r" => Ok(XlDomainState::Running),
            "b" => Ok(XlDomainState::Blocked),
            "p" => Ok(XlDomainState::Paused),
            "s" => Ok(XlDomainState::Shutdown),
            "c" => Ok(XlDomainState::Crashed),
            "d" => Ok(XlDomainState::Dying),
            _ => bail!("Unknown domain state"),
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
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        let mut parts = s.split_whitespace();
        Ok(XlListInfo {
            name: parts.next().context("Missing name")?.to_string(),
            id: parts.next().context("Missing id")?.parse()?,
            mem: parts.next().context("Missing mem")?.parse()?,
            vcpus: parts.next().context("Missing vcpus")?.parse()?,
            state: parts
                .next()
                .context("Missing state")?
                .chars()
                .filter_map(|c| match c {
                    '-' => None,
                    _ => Some(XlDomainState::from_str(&c.to_string()).unwrap()),
                })
                .collect(),
            time: parts.next().context("Missing time")?.parse()?,
        })
    }
}
pub fn list() -> Result<Vec<XlListInfo>> {
    let output = check_command(
        Command::new("xl")
            .arg("list")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
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
pub fn destroy(domid: u32) -> Result<()> {
    check_command(
        Command::new("xl")
            .arg("destroy")
            .arg(domid.to_string())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to spawn xl destroy")
            .wait_with_output(),
    )
    .map(|_| ())
}

pub fn domid(domname: String) -> Result<u32> {
    check_command(
        Command::new("xl")
            .arg("domid")
            .arg(domname)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to spawn xl domid")
            .wait_with_output(),
    )
    .map(|o| String::from_utf8(o.stdout).unwrap().trim().parse().unwrap())
}
pub fn domname(domid: u32) -> Result<String> {
    check_command(
        Command::new("xl")
            .arg("domname")
            .arg(domid.to_string())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to spawn xl domname")
            .wait_with_output(),
    )
    .map(|o| String::from_utf8(o.stdout).unwrap().trim().to_string())
}
pub fn rename(domid: u32, name: String) -> Result<()> {
    check_command(
        Command::new("xl")
            .arg("rename")
            .arg(domid.to_string())
            .arg(name)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to spawn xl rename")
            .wait_with_output(),
    )
    .map(|_| ())
}
pub fn dump_core(domid: u32, filename: String) -> Result<()> {
    check_command(
        Command::new("xl")
            .arg("dump-core")
            .arg(domid.to_string())
            .arg(filename)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to spawn xl dump-core")
            .wait_with_output(),
    )
    .map(|_| ())
}
pub fn pause(domid: u32) -> Result<()> {
    check_command(
        Command::new("xl")
            .arg("pause")
            .arg(domid.to_string())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to spawn xl pause")
            .wait_with_output(),
    )
    .map(|_| ())
}
pub fn reboot(domid: u32, force: bool) -> Result<()> {
    check_command(
        Command::new("xl")
            .arg("reboot")
            .arg(if force { "-F" } else { "" })
            .arg(domid.to_string())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
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
) -> Result<()> {
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
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to spawn xl save")
            .wait_with_output(),
    )
    .map(|_| ())
}
pub fn restore(pause: bool, checkpoint_file: PathBuf, config_file: Option<PathBuf>) -> Result<()> {
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
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
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

pub fn shutdown(system: XlShutdownTarget, wait: bool, force: bool) -> Result<()> {
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
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to spawn xl shutdown")
            .wait_with_output(),
    )
    .map(|_| ())
}
pub fn unpause(domid: u32) -> Result<()> {
    check_command(
        Command::new("xl")
            .arg("unpause")
            .arg(domid.to_string())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
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
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        let mut parts = s.split_whitespace().map(|p| p.trim());
        let idx = parts.next().unwrap().parse()?;
        let be = parts.next().unwrap().parse()?;
        let mac = parts.next().unwrap().parse()?;
        let handle = parts.next().unwrap().parse()?;
        let state = parts.next().unwrap().parse()?;
        let evt_ch = parts.next().unwrap().parse()?;
        let tx_rx: Vec<i32> = parts
            .next()
            .unwrap()
            .split("/")
            .map(|p| p.parse().unwrap())
            .collect();
        Ok(XlNetworkListEntry {
            idx,
            be,
            mac,
            handle,
            state,
            evt_ch,
            tx: tx_rx[0],
            rx: tx_rx[1],
            be_path: parts.next().unwrap().to_string(),
        })
    }
}

pub fn network_list(domid: u32) -> Result<Vec<XlNetworkListEntry>> {
    check_command(
        Command::new("xl")
            .arg("network-list")
            .arg(domid.to_string())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
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

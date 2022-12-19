use std::{
    collections::HashSet,
    error::Error,
    io::{self, BufRead, BufReader, Cursor},
    net::Ipv4Addr,
    path::PathBuf,
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};

use anyhow::{anyhow, bail, Result};
use log::{debug, error, info, warn, LevelFilter};
use macaddr::MacAddr6;
use nix::unistd::Uid;
use simple_logger::SimpleLogger;
use tokio::time::sleep;
use xen::xl::{domid, network_list};

pub mod ssh;
pub mod util;
pub mod xen;

use crate::xen::xl::list as xl_list;

pub fn check_command(result: Result<Output, io::Error>) -> Result<Output> {
    match result {
        Ok(output) => {
            if output.status.success() {
                Ok(output)
            } else {
                error!("Command failed. Output:");

                BufReader::new(Cursor::new(output.stdout))
                    .lines()
                    .filter_map(|l| l.map_err(|e| e).ok())
                    .for_each(|l| {
                        error!("out: {}", l);
                    });

                BufReader::new(Cursor::new(output.stderr))
                    .lines()
                    .filter_map(|l| l.map_err(|e| e).ok())
                    .for_each(|l| {
                        error!("out: {}", l);
                    });

                bail!("Error running command");
            }
        }
        Err(e) => Err(e)?,
    }
}

pub fn new_domnaname(prefix: String) -> Result<String> {
    let doms = xl_list()?;
    let suffixes = doms
        .iter()
        .filter_map(|d| {
            if d.name.starts_with(&prefix) {
                match d.name.trim_start_matches(&prefix).parse::<u32>() {
                    Ok(n) => Some(n),
                    // Errors just mean it's not a number which is ok and valid
                    Err(_) => None,
                }
            } else {
                None
            }
        })
        .collect::<Vec<u32>>();
    let max = suffixes.iter().max().unwrap_or(&0);
    Ok(format!("{}{}", prefix, max + 1))
}

fn gigabytes_to_bytes(gb: u64) -> u64 {
    gb * 1024 * 1024 * 1024
}

/// Create a new image at path with a size in GB
pub fn new_img(path: PathBuf, size: u64) -> Result<PathBuf> {
    let file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(&path)?;
    file.set_len(gigabytes_to_bytes(size))?;
    Ok(path)
}

pub fn checkroot() -> Result<()> {
    if nix::unistd::geteuid() != Uid::from_raw(0) {
        bail!("Must be run as root");
    }

    Ok(())
}

pub fn logging_config() -> Result<(), Box<dyn Error>> {
    SimpleLogger::new()
        .env()
        .with_level(LevelFilter::Debug)
        .init()
        .unwrap();
    info!("Logging configured");
    Ok(())
}

pub struct Neighbor {
    pub ip: Ipv4Addr,
    pub dev: String,
    pub lladdr: Option<MacAddr6>,
    pub state: String,
}
/// iproute2 has no rust bindings :/
pub fn ip_neighbors() -> Result<Vec<Neighbor>> {
    Ok(check_command(
        Command::new("ip")
            .arg("neighbor")
            .arg("show")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Could not run the ip command")
            .wait_with_output(),
    )?
    .stdout
    .lines()
    .filter_map(|l| l.ok())
    .filter(|l| !l.trim().is_empty())
    .map(|l| {
        debug!("Interpreting Neighbor from {}", l);
        let mut parts = l.split_whitespace().rev();

        let state = parts.next().unwrap().to_string();
        let mut lladdr = None;
        match state.as_str() {
            "FAILED" => {}
            _ => {
                lladdr = Some(parts.next().unwrap().parse().unwrap());
                // Ignore the 'lladdr' string
                parts.next().unwrap_or("");
            }
        }
        let dev = parts.next().unwrap().to_string();
        parts.next().unwrap();
        let ip = parts.next().unwrap().parse().unwrap();
        // lladdr <MAC> can be missing

        Neighbor {
            ip,
            dev,
            lladdr,
            state,
        }
    })
    .collect())
}

pub async fn dom_mac(domname: String) -> Result<HashSet<MacAddr6>> {
    let networks = network_list(domid(domname.to_string()).unwrap()).unwrap();
    Ok(networks.iter().map(|e| e.mac).collect::<HashSet<_>>())
}

async fn domip_once(domname: String) -> Result<Ipv4Addr> {
    let networks = network_list(domid(domname.to_string()).unwrap()).unwrap();
    let macs = networks.iter().map(|e| e.mac).collect::<HashSet<_>>();
    info!("Searching for VM IP from macs: {:?}", macs);
    let neighbors = ip_neighbors().unwrap();
    let ips: HashSet<Ipv4Addr> = neighbors
        .iter()
        .filter(|n| match n.lladdr {
            Some(lladdr) => macs.contains(&lladdr),
            None => false,
        })
        .map(|n| n.ip)
        .collect();
    match ips.iter().take(1).next() {
        Some(ip) => Ok(*ip),
        None => Err(anyhow!("Unable to get IP")),
    }
}

pub async fn domip(domname: String, timeout: u64) -> Result<Ipv4Addr> {
    let start = Instant::now();
    let mut sleepct = 1;
    loop {
        match domip_once(domname.clone()).await {
            Ok(domip) => return Ok(domip),
            Err(e) => {
                warn!(
                    "Unable to retrieve ip for DOM {}: {}. Retrying.",
                    &domname, e
                );
                if start.elapsed().as_secs() > timeout {
                    break;
                }
                info!("Waiting {} seconds to retry.", sleepct);
                sleep(Duration::from_secs(sleepct)).await;
            }
        }
        // Backoff exponentially, if we don't have it in a few seconds it will probably take some minutes
        sleepct *= 2;
    }
    bail!(
        "Unable to get IP for DOM {} in {} seconds",
        domname,
        timeout
    );
}

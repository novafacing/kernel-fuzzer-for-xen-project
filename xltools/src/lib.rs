use std::{
    cell::RefCell,
    collections::HashSet,
    error::Error,
    io::{self, BufRead, BufReader, Cursor},
    net::Ipv4Addr,
    path::PathBuf,
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};

use anyhow::{anyhow, bail, Context, Result};
use etherparse::{InternetSlice, LinkSlice, SlicedPacket};
use futures::{future::select_all, stream::iter, StreamExt};
use log::{debug, error, info, warn, LevelFilter};
use macaddr::MacAddr6;
use nix::unistd::Uid;
use pcap::{Active, Capture, Device, Error as PCAPError, Packet, PacketCodec, PacketStream};
use simple_logger::SimpleLogger;
use tokio::time::{sleep, timeout as tokio_timeout, Timeout};
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

pub async fn dom_mac(domname: &str) -> Result<HashSet<MacAddr6>> {
    let networks = network_list(domid(domname.to_string()).unwrap()).unwrap();
    Ok(networks.iter().map(|e| e.mac).collect::<HashSet<_>>())
}

pub struct IPSearchCodec;

impl PacketCodec for IPSearchCodec {
    type Item = Option<((MacAddr6, Ipv4Addr), (MacAddr6, Ipv4Addr))>;

    fn decode(&mut self, packet: Packet) -> Self::Item {
        match SlicedPacket::from_ethernet(packet.data) {
            Ok(pkt) => match pkt.ip {
                Some(InternetSlice::Ipv4(iphdr, _ipext)) => match pkt.link {
                    Some(LinkSlice::Ethernet2(ehdr)) => Some((
                        (MacAddr6::from(ehdr.source()), iphdr.source_addr()),
                        (MacAddr6::from(ehdr.destination()), iphdr.destination_addr()),
                    )),
                    _ => None,
                },
                _ => None,
            },
            Err(e) => {
                debug!("Error reading IP packet {:?}: {}", packet, e);
                None
            }
        }
    }
}

async fn dom_ip_search_mac(
    mac: &MacAddr6,
    stream: &RefCell<PacketStream<Active, IPSearchCodec>>,
) -> Result<Ipv4Addr> {
    while let Some(p) = stream.borrow_mut().next().await {
        match p {
            Ok(p) => match p {
                Some(((smac, sip), (dmac, dip))) => {
                    if mac == &smac {
                        debug!(
                            "smac: {:?} == search mac: {:?}. sip: {:?} dip: {:?}",
                            smac, mac, sip, dip
                        );
                        return Ok(sip);
                    } else if mac == &dmac {
                        debug!(
                            "dmac: {:?} == search mac: {:?}. dip: {:?} sip: {:?}",
                            dmac, mac, dip, sip
                        );
                        return Ok(dip);
                    } else {
                        debug!("smac: {:?} != search mac: {:?}", smac, mac);
                    }
                }
                None => {
                    // debug!("Did not get well-formed input");
                }
            },
            Err(e) => {
                error!("Failed to capture on device: {}", e);
            }
        }
    }
    bail!("Did not find IP for mac address {}", mac);
}

async fn dom_ip_inner(domname: &str) -> Result<Ipv4Addr> {
    let devices = Device::list()?;
    let devices: HashSet<String> = devices
        .iter()
        .map(|d| d.name.clone())
        // .chain(devnames.iter().map(|ni| ni.name.clone()))
        // .chain(odevnames.iter().map(|i| i.name.clone()))
        .collect();
    devices.iter().for_each(|dn| {
        info!("Listening on device: {}", dn);
    });
    let macs = dom_mac(domname).await?;
    let mac = macs
        .iter()
        .take(1)
        .next()
        .context(format!("Could not find a MAC address for DOM {}", &domname))?;
    let streams: Vec<RefCell<PacketStream<Active, IPSearchCodec>>> = iter(devices)
        .map(|d| async move {
            Ok(Capture::from_device(Device::from(d.as_str()))?
                .promisc(true)
                .immediate_mode(true)
                .open()?
                .setnonblock()?
                .stream(IPSearchCodec {})?)
        })
        .buffer_unordered(64)
        .filter_map(
            |sr: Result<PacketStream<Active, IPSearchCodec>, PCAPError>| async move {
                match sr {
                    Ok(ps) => Some(RefCell::new(ps)),
                    Err(e) => {
                        error!("Unable to get packet stream: {}", e);
                        None
                    }
                }
            },
        )
        .collect()
        .await;

    let mut tasks = streams
        .iter()
        .map(|s| dom_ip_search_mac(&mac, s))
        .map(Box::pin)
        .collect::<Vec<_>>();

    while !tasks.is_empty() {
        match select_all(tasks).await {
            (Ok(v), _idx, _remaining) => {
                return Ok(v);
            }
            (Err(e), _idx, remaining) => {
                warn!("Error getting Ipv4 address. Ignoring: {}", e);
                tasks = remaining;
            }
        }
    }
    bail!("Did not obtain any IP for {}", domname);
}

pub async fn dom_ip(domname: &str, timeout: u64) -> Result<Ipv4Addr> {
    tokio_timeout(Duration::from_secs(timeout), dom_ip_inner(domname)).await?
}

use std::{
    error::Error,
    io::{self, BufRead, BufReader, Cursor},
    path::PathBuf,
    process::Output,
};

use log::error;

pub mod presets;
pub mod xl;
pub mod xlcfg;

use crate::xl::list as xl_list;

pub fn check_command(result: Result<Output, io::Error>) -> Result<Output, Box<dyn Error>> {
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

                Err("Error running command")?
            }
        }
        Err(e) => Err(e)?,
    }
}

pub fn new_domnaname(prefix: String) -> Result<String, Box<dyn Error>> {
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

// Check currently listening ports and return the next available VNC port (5900 + X)
pub fn next_vnc_port() -> Result<u16, Box<dyn Error>> {
    // Just iterate from 5900 and try to bind. If we can, release the port and return it
    for port in 5900..65535 {
        match std::net::TcpListener::bind(format!("0.0.0.0:{}", port)) {
            Ok(listener) => {
                drop(listener);
                return Ok(port);
            }
            Err(_) => {}
        }
    }
    Err("No available VNC ports")?
}

/// Create a new image at path with a size in GB
pub fn new_img(path: PathBuf, size: u32) -> Result<PathBuf, Box<dyn Error>> {
    let file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(path)?;
    file.set_len((size * 1024 * 1024 * 1024) as u64)?;
    Ok(path)
}

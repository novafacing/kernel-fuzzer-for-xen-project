//! Common functionality for KF/x install scripts
//!
use std::{
    collections::HashMap,
    error::Error,
    fs::{File, OpenOptions},
    io::{copy, BufRead, BufReader, Write},
    path::PathBuf,
};

use flate2::read::GzDecoder;
use regex::Regex;
use reqwest::blocking::get;
use tar::Archive;

/// Read the /etc/os-release file, which is present on (at least):
/// * Debian Buster
/// * Debian Bullseye
/// * Ubuntu Bionic
/// * Ubuntu Focal
/// * Ubuntu Jammy
pub fn read_os_release() -> Result<HashMap<String, String>, Box<dyn Error>> {
    const OS_RELEASE_PATH: &str = "/etc/os-release";
    let os_release_file = File::open(PathBuf::from(OS_RELEASE_PATH))?;

    Ok(BufReader::new(os_release_file)
        .lines()
        .filter_map(|l| l.map_err(|e| e).ok())
        .filter_map(|l| {
            let mut entry = l.split("=");
            if let Some(key) = entry.next() {
                if let Some(val) = entry.next() {
                    return Some((key.to_string(), val.to_string()));
                }
            }
            None
        })
        .collect::<HashMap<String, String>>())
}

pub fn append_line(file: PathBuf, line: String) -> Result<(), Box<dyn Error>> {
    let mut f = OpenOptions::new().write(true).append(true).open(file)?;
    f.write(&line.into_bytes())?;
    f.write(b"\n")?;
    Ok(())
}

pub fn replace_text(file: PathBuf, pattern: &str, replacement: &str) -> Result<(), Box<dyn Error>> {
    let mut f = OpenOptions::new().read(true).write(true).open(file)?;
    let regex = Regex::new(pattern)?;
    let newlines: Vec<String> = BufReader::new(&f)
        .lines()
        .filter_map(|l| l.map_err(|e| e).ok())
        .map(|l| regex.replace(l.as_str(), replacement).to_string())
        .collect();
    f.write_all(&newlines.join("\n").as_bytes())?;

    Ok(())
}

pub fn download(url: &str, path: PathBuf) -> Result<(), Box<dyn Error>> {
    let response = get(url)?;
    let mut f = File::create(path)?;
    let content = response.text()?;
    copy(&mut content.as_bytes(), &mut f)?;
    Ok(())
}

pub fn unpack_tgz(compressed: PathBuf, dest: PathBuf) -> Result<(), Box<dyn Error>> {
    let f = File::open(compressed)?;
    let gz = GzDecoder::new(f);
    let mut tar = Archive::new(gz);
    tar.unpack(dest)?;
    Ok(())
}

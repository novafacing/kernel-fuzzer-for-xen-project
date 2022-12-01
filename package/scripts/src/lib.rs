//! Common functionality for KF/x install scripts
use std::{
    collections::HashMap,
    error::Error,
    fs::{copy as fs_copy, create_dir_all, File, OpenOptions},
    io::{self, copy, BufRead, BufReader, Cursor, Read, Write},
    path::PathBuf,
    process::{Command, Output, Stdio},
};

use flate2::read::GzDecoder;
use log::{error, LevelFilter};
use regex::Regex;
use reqwest::blocking::get;
use simple_logger::SimpleLogger;
use tar::Archive;
use walkdir::WalkDir;

/// Read the /etc/os-release file, which is present on (at least):
/// * Debian Buster
/// * Debian Bullseye
/// * Ubuntu Bionic
/// * Ubuntu Focal
/// * Ubuntu Jammy
pub fn read_os_release() -> Result<HashMap<String, String>, Box<dyn Error>> {
    const OS_RELEASE_PATH: &str = "/etc/os-release";
    let os_release_file = File::open(PathBuf::from(OS_RELEASE_PATH)).map_err(|e| {
        error!("Error reading /etc/os-release: {}", e);
        e
    })?;

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

/// Append a line to a file, creating it if it does not exist
pub fn append_line(file: &PathBuf, line: String) -> Result<(), Box<dyn Error>> {
    let mut f = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(file)?;
    f.write(&line.into_bytes())?;
    f.write(b"\n")?;
    Ok(())
}

// Replace text in lines in a file, works similarly to `sed`
pub fn replace_text(
    file: &PathBuf,
    pattern: &str,
    replacement: &str,
) -> Result<(), Box<dyn Error>> {
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

/// Inner download function
fn download_one(url: &str, path: &PathBuf) -> Result<(), Box<dyn Error>> {
    let response = get(url)?;
    let mut f = File::create(path)?;
    let mut content = Cursor::new(response.bytes()?);
    copy(&mut content, &mut f)?;
    Ok(())
}

/// Download a file to a path, retrying up to `RETRY_LIMIT` times
pub fn download(url: &str, path: &PathBuf) -> Result<(), Box<dyn Error>> {
    const RETRY_LIMIT: usize = 5;
    let mut err = None;

    for _ in 0..RETRY_LIMIT {
        match download_one(url, path) {
            Ok(_) => return Ok(()),
            Err(e) => {
                eprintln!("error downloading {}, retrying: {}", url, e);
                err = Some(e)
            }
        }
    }

    Err(err.unwrap())
}

/// Unpack a tarball to a destination
pub fn unpack_tgz(compressed: &PathBuf, dest: &PathBuf) -> Result<(), Box<dyn Error>> {
    let f = File::open(compressed)?;
    let gz = GzDecoder::new(f);
    let mut tar = Archive::new(gz);
    tar.unpack(dest)?;
    Ok(())
}

/// Initialize logging
pub fn init_logging() -> Result<(), Box<dyn Error>> {
    SimpleLogger::new()
        .env()
        .with_level(LevelFilter::Info)
        .init()?;

    Ok(())
}

/// Check the output of a process::Command execution and log the full output of the
/// program if an error occurred. Returns an error if the command failed or an error
/// occurred
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

/// Get the architecture string, appropriate for use in DEBIAN/control files
pub fn get_dpkg_arch() -> Result<String, Box<dyn Error>> {
    Ok(String::from_utf8_lossy(
        &check_command(
            Command::new("dpkg")
                .arg("--print-architecture")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("Failed to run dpkg")
                .wait_with_output(),
        )?
        .stdout,
    )
    .trim()
    .to_string())
}

/// Get the size of a directory in KB
pub fn dir_size(path: &PathBuf) -> Result<u64, Box<dyn Error>> {
    let mut size = 0;
    for entry in WalkDir::new(path) {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if metadata.is_file() {
            size += metadata.len();
        }
    }
    Ok(size / 1024)
}

/// Copy all files and directories in a directory to another directory
pub fn copy_dir(src: &PathBuf, dest: &PathBuf) -> Result<(), Box<dyn Error>> {
    for entry in WalkDir::new(src) {
        let entry = entry?;
        let metadata = entry.metadata()?;
        let path = entry.path();
        let dest_path = dest.join(path.strip_prefix(src)?);
        if metadata.is_file() {
            // Copy the file to the destination
            fs_copy(path, &dest_path)?;
        } else if metadata.is_dir() {
            create_dir_all(dest_path)?;
        }
    }
    Ok(())
}

pub struct DebControl {
    pub package: String,
    pub source: String,
    pub version: String,
    pub architecture: String,
    pub maintainer: String,
    pub depends: Vec<String>,
    pub conflicts: Vec<String>,
    pub section: String,
    pub priority: String,
    pub installed_size: usize,
    pub description: String,
}

impl DebControl {
    pub fn from_file(path: &PathBuf) -> Result<DebControl, Box<dyn Error>> {
        let mut f = File::open(path)?;
        let mut contents = String::new();
        f.read_to_string(&mut contents)?;

        let mut package = String::new();
        let mut source = String::new();
        let mut version = String::new();
        let mut architecture = String::new();
        let mut maintainer = String::new();
        let mut depends = Vec::new();
        let mut conflicts = Vec::new();
        let mut section = String::new();
        let mut priority = String::new();
        let mut installed_size = 0;
        let mut description = String::new();

        for line in contents.lines() {
            if line.starts_with("Package:") {
                package = line.split(":").nth(1).unwrap().trim().to_string();
            } else if line.starts_with("Source:") {
                source = line.split(":").nth(1).unwrap().trim().to_string();
            } else if line.starts_with("Version:") {
                version = line.split(":").nth(1).unwrap().trim().to_string();
            } else if line.starts_with("Architecture:") {
                architecture = line.split(":").nth(1).unwrap().trim().to_string();
            } else if line.starts_with("Maintainer:") {
                maintainer = line.split(":").nth(1).unwrap().trim().to_string();
            } else if line.starts_with("Depends:") {
                depends = line
                    .split(":")
                    .nth(1)
                    .unwrap()
                    .trim()
                    .split(",")
                    .map(|s| s.trim().to_string())
                    .collect();
            } else if line.starts_with("Conflicts:") {
                conflicts = line
                    .split(":")
                    .nth(1)
                    .unwrap()
                    .trim()
                    .split(",")
                    .map(|s| s.trim().to_string())
                    .collect();
            } else if line.starts_with("Section:") {
                section = line.split(":").nth(1).unwrap().trim().to_string();
            } else if line.starts_with("Priority:") {
                priority = line.split(":").nth(1).unwrap().trim().to_string();
            } else if line.starts_with("Installed-Size:") {
                installed_size = line.split(":").nth(1).unwrap().trim().parse()?;
            } else if line.starts_with("Description:") {
                description = line.split(":").nth(1).unwrap().trim().to_string();
            }
        }
        Ok(DebControl::new(
            package,
            source,
            version,
            architecture,
            maintainer,
            depends,
            conflicts,
            section,
            priority,
            installed_size,
            description,
        ))
    }

    pub fn to_string(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!("Package: {}\n", self.package));
        s.push_str(&format!("Source: {}\n", self.source));
        s.push_str(&format!("Version: {}\n", self.version));
        s.push_str(&format!("Architecture: {}\n", self.architecture));
        s.push_str(&format!("Maintainer: {}\n", self.maintainer));
        s.push_str(&format!(
            "Depends: {}\n",
            self.depends.join(", ").to_string()
        ));
        s.push_str(&format!(
            "Conflicts: {}\n",
            self.conflicts.join(", ").to_string()
        ));
        s.push_str(&format!("Section: {}\n", self.section));
        s.push_str(&format!("Priority: {}\n", self.priority));
        s.push_str(&format!("Installed-Size: {}\n", self.installed_size));
        s.push_str(&format!("Description: {}\n", self.description));
        s
    }

    pub fn new(
        package: String,
        source: String,
        version: String,
        architecture: String,
        maintainer: String,
        depends: Vec<String>,
        conflicts: Vec<String>,
        section: String,
        priority: String,
        installed_size: usize,
        description: String,
    ) -> Self {
        Self {
            package,
            source,
            version,
            architecture,
            maintainer,
            depends,
            conflicts,
            section,
            priority,
            installed_size,
            description,
        }
    }
}

//! Utilities for SSH key management

use std::{
    env::var,
    fs::{create_dir_all, read_to_string, set_permissions, write, Permissions},
    os::unix::prelude::PermissionsExt,
    path::PathBuf,
};

use anyhow::{bail, Result};
use log::{debug, info, warn};
use openssh_keys::PublicKey;
use openssl::rsa::Rsa;

pub fn generate_rsa(ssh_dir: PathBuf) -> Result<String> {
    warn!("No public keys found, generating RSA key pair.");
    let rsa = Rsa::generate(4096)?;
    let private = rsa.private_key_to_pem()?;
    let mut public = PublicKey::from_rsa(rsa.e().to_vec(), rsa.n().to_vec());
    public.set_comment("xltools");
    info!(
        "Generated RSA key pair with public fingerprint {}",
        public.to_key_format()
    );
    // We didn't find a public key but we need to make sure we don't overwrite a private key
    let priv_path = ssh_dir.join("id_rsa");
    if priv_path.exists() {
        bail!("Refusing to overwrite existing RSA key");
    }
    let pub_path = ssh_dir.join("id_rsa.pub");
    write(&priv_path, private)?;
    set_permissions(priv_path, Permissions::from_mode(0o600))?;
    write(&pub_path, public.to_key_format())?;
    set_permissions(pub_path, Permissions::from_mode(0o644))?;
    Ok(public.to_key_format())
}

pub fn get_local_keys() -> Result<Vec<String>> {
    let ssh_dir = PathBuf::from(var("HOME")?).join(".ssh");

    if !&ssh_dir.exists() {
        create_dir_all(&ssh_dir)?;
        set_permissions(&ssh_dir, Permissions::from_mode(0o700))?;
    }

    debug!("Reading public keys from {}", ssh_dir.to_string_lossy());
    let mut keys: Vec<String> = ssh_dir
        .read_dir()?
        .filter_map(|e| e.ok())
        .filter(|e| {
            debug!("Checking file {}", e.file_name().to_string_lossy());
            let ft = e.file_type();
            match ft {
                Ok(ft) => {
                    ft.is_file()
                        && match e.path().extension() {
                            Some(ext) => ext == "pub",
                            None => false,
                        }
                }
                Err(_) => false,
            }
        })
        .map(|e| e.path().to_path_buf())
        .filter_map(|p| read_to_string(p).ok())
        .collect();
    info!("Found {} existing public keys.", keys.len());

    if keys.is_empty() {
        let public = generate_rsa(ssh_dir)?;
        keys.push(public);
    }

    Ok(keys)
}

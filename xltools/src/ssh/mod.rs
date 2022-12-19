use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    time::Duration,
};

use crate::dom_ip;

use self::{bootstrap::Session as BootstrapSession, keys::get_local_keys};

use anyhow::Result;
use log::{debug, warn};
use openssh::{KnownHosts, Session, SessionBuilder};

pub mod bootstrap;
pub mod keys;

/// Send the key using the russh ssh module, which is less capable but supports password auth
async fn ssh_sendkeys(
    addr: SocketAddr,
    timeout: u64,
    username: String,
    password: String,
) -> Result<()> {
    let timeout = Duration::from_secs(timeout);
    let mut ssh = BootstrapSession::connect(&username, &password, addr, timeout).await?;

    ssh.execute_chk(
        r#"powershell New-Item -Force -ItemType Directory -Path $env:USERPROFILE\.ssh"#,
    )
    .await?;
    for key in get_local_keys()? {
        debug!("Sending key {}", key);
        ssh.execute_chk(&format!(
            r#"powershell Add-Content -Force -Path $env:USERPROFILE\.ssh\authorized_keys -Value '{}'"#,
            key
        ))
        .await?;
    }

    Ok(())
}

async fn ssh_session(addr: Ipv4Addr, port: u16, timeout: u64, username: String) -> Result<Session> {
    let remote = format!("{}@{}", username, addr);
    let session = SessionBuilder::default()
        .user(username)
        .port(port)
        .connect_timeout(Duration::from_secs(timeout))
        .known_hosts_check(KnownHosts::Accept)
        .compression(true)
        .connect_mux(remote)
        .await?;

    Ok(session)
}

pub async fn ssh_domname(
    domname: &str,
    port: u16,
    timeout: u64,
    username: String,
    password: String,
) -> Result<Session> {
    let ip = dom_ip(domname, timeout).await?;
    let addr = SocketAddr::V4(SocketAddrV4::new(ip, port));

    let session = match ssh_session(ip, port, timeout, username.clone()).await {
        Ok(session) => session,
        Err(e) => {
            warn!("Error: {}", e);
            warn!("Error connecting to session with key authentication, attempting to send keys and reconnect.");
            // There was some error in connecting, likely because we do not have a remote key
            // try to send it
            ssh_sendkeys(addr, timeout, username.clone(), password.clone()).await?;
            ssh_session(ip, port, timeout, username).await?
        }
    };

    Ok(session)
}

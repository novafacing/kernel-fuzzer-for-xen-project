//! Implements SSH utilities for SSH that uses password authentication
//! to bootstrap a keyed session

use std::{io::Write, net::SocketAddr, sync::Arc, time::Duration};

use anyhow::{bail, Result};
use futures::future::Ready;
use log::{debug, error};
use russh::{
    client::{self, connect, Config, Handle, Handler},
    ChannelMsg, Disconnect,
};
use russh_keys::key::PublicKey;

pub struct CommandResult {
    output: Vec<u8>,
    code: Option<u32>,
}

impl CommandResult {
    pub fn output(&self) -> String {
        String::from_utf8_lossy(&self.output).into()
    }

    pub fn success(&self) -> bool {
        self.code == Some(0)
    }
}

pub struct Client {}

impl Handler for Client {
    type Error = russh::Error;
    type FutureUnit = Ready<Result<(Self, client::Session), Self::Error>>;
    type FutureBool = Ready<Result<(Self, bool), Self::Error>>;

    fn finished_bool(self, b: bool) -> Self::FutureBool {
        futures::future::ready(Ok((self, b)))
    }
    fn finished(self, session: client::Session) -> Self::FutureUnit {
        futures::future::ready(Ok((self, session)))
    }
    fn check_server_key(self, _server_public_key: &PublicKey) -> Self::FutureBool {
        self.finished_bool(true)
    }
}

pub struct Session {
    session: Handle<Client>,
}

impl Session {
    pub async fn connect(
        user: impl Into<String>,
        password: impl Into<String>,
        addrs: SocketAddr,
        timeout: Duration,
    ) -> Result<Self> {
        let config = Config {
            connection_timeout: Some(timeout),
            ..<_>::default()
        };
        let config = Arc::new(config);
        let sh = Client {};
        let mut session = connect(config, addrs, sh).await?;
        let _auth_res = session.authenticate_password(user, password).await?;
        Ok(Self { session })
    }

    pub async fn execute(&mut self, command: &str) -> Result<CommandResult> {
        let mut channel = self.session.channel_open_session().await?;
        channel.exec(true, command).await?;
        let mut output = Vec::new();
        let mut code = None;
        while let Some(msg) = channel.wait().await {
            match msg {
                ChannelMsg::Data { ref data } => {
                    output.write_all(data).unwrap();
                }
                ChannelMsg::ExitStatus { exit_status } => {
                    code = Some(exit_status);
                }
                _ => {}
            }
        }
        debug!("execute: {}", String::from_utf8_lossy(&output));
        debug!("execute: {}", code.unwrap_or(0));
        Ok(CommandResult { output, code })
    }

    pub async fn execute_chk(&mut self, command: &str) -> Result<CommandResult> {
        let result = self.execute(command).await?;

        match result.success() {
            true => Ok(result),
            false => {
                error!("Remote command failed: '{}'", command);
                for line in result.output().lines() {
                    error!("output: {}", line);
                }
                bail!("Remote command failed.");
            }
        }
    }

    pub async fn close(&mut self) -> Result<()> {
        self.session
            .disconnect(Disconnect::ByApplication, "", "English")
            .await?;
        Ok(())
    }
}

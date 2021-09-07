//! The `autohost` applet.
//!
//! It sets up the system hostname as well as `/etc/hosts`.

use std::path::PathBuf;

use async_trait::async_trait;
use serde::Deserialize;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncBufReadExt, AsyncSeekExt, AsyncWriteExt, BufReader, SeekFrom};

use crate::config::Config;
use crate::error::Result;
use super::{Applet, Sender, Message};

/// `autohost` applet configuration.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct AutohostConfig {
    /// Whether to enable the applet or not.
    enable: bool,

    /// Path to the hosts file to update (normally /etc/hosts).
    etc_hosts: PathBuf,
}

impl Default for AutohostConfig {
    fn default() -> Self {
        Self {
            enable: true,
            etc_hosts: PathBuf::from("/etc/hosts"),
        }
    }
}

/// The `autohost` applet.
#[derive(Debug)]
pub struct Autohost {
    config: Config,
    tx: Sender,
}

impl Autohost {
    pub(super) async fn new(config: Config, tx: Sender) -> Result<Box<dyn Applet>> {
        Ok(Box::new(Self {
            config,
            tx,
        }))
    }
}

#[async_trait]
impl Applet for Autohost {
    async fn main(&self) -> Result<()> {
        let mut rx = self.tx.subscribe();

        if !self.config.autohost.enable {
            log::info!("autohost applet disabled in config");
            return Ok(());
        }

        loop {
            let message = rx.recv().await.unwrap();
            match message {
                Message::Shutdown(_) => {
                    break;
                }

                Message::UpdateCanonical(fqdn, ipv4) => {
                    log::info!("Updating system hostname...");

                    hostname::set(&fqdn)?;

                    // We add an entry to /etc/hosts so it can be resolved
                    // instantly
                    let file = OpenOptions::new()
                        .read(true)
                        .write(true)
                        .create(true)
                        .open(&self.config.autohost.etc_hosts)
                        .await?;

                    let (mut file, existing_hosts) = {
                        let mut bytes = Vec::new();

                        let reader = BufReader::new(file);
                        let mut lines = reader.lines();

                        // Read everything until our marker
                        while let Some(line) = lines.next_line().await? {
                            if line.find("miniond").is_some() {
                                break;
                            }

                            bytes.extend_from_slice(line.as_bytes());
                            bytes.push('\n' as u8);
                        }

                        (lines.into_inner().into_inner(), bytes)
                    };

                    file.seek(SeekFrom::Start(0)).await?;
                    file.set_len(0).await?;

                    file.write_all(&existing_hosts).await?;
                    file.write_all("# the following is generated by miniond\n".as_bytes()).await?;
                    file.write_all(format!("{} {}\n", ipv4, fqdn).as_bytes()).await?;
                }

                _ => {}
            }
        }

        Ok(())
    }
}

//! Get the IP of a virtual machine by name

use anyhow::Context;
use clap::Parser;
use tokio;
use xltools::{dom_ip, logging_config};

#[derive(Parser)]
struct Args {
    /// The name of the dom to get the IP for
    name: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    logging_config().expect("Could not configure logging");
    println!(
        "{}",
        dom_ip(&args.name, 30)
            .await
            .context("Could not get DOM IP")
            .unwrap()
    );
}

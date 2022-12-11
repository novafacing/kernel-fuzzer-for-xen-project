//! Get the IP of a virtual machine by name

use anyhow::Context;
use clap::Parser;
use tokio;
use xltools::domip;

#[derive(Parser)]
struct Args {
    /// The name of the dom to get the IP for
    name: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    println!(
        "{}",
        domip(args.name, 30)
            .await
            .context("Could not get DOM IP")
            .unwrap()
    );
}

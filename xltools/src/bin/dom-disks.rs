use xltools::{checkroot, logging_config, xen::xs::dom_disks};

use clap::Parser;

#[derive(Parser)]
/// Get a list of disk files attached to a DOM
struct Args {
    /// The name of the DOM to get disk information for
    domname: String,
}

fn main() {
    let args = Args::parse();
    checkroot().expect("Must be run as root");
    logging_config().expect("Could not configure logging");
    // cc: https://github.com/cgfandia-tii/libmicrovmi/blob/master/src/driver/xen.rs
    dom_disks(&args.domname)
        .expect(&format!("Could not get disks for dom '{}'", &args.domname))
        .iter()
        .for_each(|d| {
            println!("{}", d);
        });
}

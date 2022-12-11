use xltools::{checkroot, logging_config, xen::xs::dom_disks};

fn main() {
    checkroot().expect("Must be run as root");
    logging_config().expect("Could not configure logging");
    // cc: https://github.com/cgfandia-tii/libmicrovmi/blob/master/src/driver/xen.rs
    println!("{:?}", dom_disks("windev1".to_string()).unwrap());
}

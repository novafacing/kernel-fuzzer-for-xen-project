//! Xenstore convenience functions

use anyhow::Result;
use log::{debug, error};
use xenstore_rs::{XBTransaction, Xs, XsOpenFlags};

pub fn dom_disks(domname: &str) -> Result<Vec<String>> {
    let xs = Xs::new(XsOpenFlags::ReadOnly).expect("Could not open xenstore");
    Ok(xs
        .directory(XBTransaction::Null, "/local/domain")?
        .iter()
        .filter_map(|domid| {
            xs.read(
                XBTransaction::Null,
                &format!("/local/domain/{}/name", domid),
            )
            .map_err(|e| {
                error!("Error getting domain names: {}", e);
                e
            })
            .ok()
            .map(|name| (name, domid))
        })
        .filter(|(name, _id)| name == domname)
        .map(|(_name, id)| {
            debug!("Checking for virtual devices for domain '{}'", id);
            Ok(xs
                .directory(XBTransaction::Null, &format!("/libxl/{}/device/vbd", id))?
                .iter()
                .filter_map(|vbdid| {
                    xs.read(
                        XBTransaction::Null,
                        &format!("/libxl/{}/device/vbd/{}/params", id, vbdid),
                    )
                    .map_err(|e| {
                        error!("Could not read vbd device params: {}", e);
                    })
                    .ok()
                })
                .collect::<Vec<_>>())
        })
        .filter_map(|r: Result<Vec<String>>| r.ok())
        .flat_map(|devs| devs)
        .collect())
}

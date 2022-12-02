//! Packaging script for KF/x and KF/x-Xen

use std::{error::Error, fs::create_dir_all, path::PathBuf};

use clap::{Parser, Subcommand};
use package::{
    deps::{install_apt_deps, install_golang},
    init_logging,
    kfx::{
        build_capstone, build_dwarf2json, build_kfx, build_libvmi, build_libxdc, make_bundle_deb,
        make_kfx_deb,
    },
    xen::{build_xen, configure_xen, make_deb},
};
use tempdir::TempDir;

#[derive(Debug, Subcommand)]
pub enum Action {
    /// Determine requirements and install dependencies
    Dependencies,
    /// Build KF/x Xen
    BuildXen(BuildXenArgs),
    /// Build KF/x
    BuildKFx(BuildKFxArgs),
}

#[derive(Parser, Debug)]
pub struct Args {
    /// The command to run
    #[clap(subcommand)]
    pub action: Action,
}

#[derive(Parser, Debug)]
pub struct BuildXenArgs {
    /// The path to the KF/x source directory
    pub xen_path: PathBuf,
    /// The path to output build artifacts
    pub output_path: PathBuf,
}

#[derive(Parser, Debug)]
pub struct BuildKFxArgs {
    /// The path to the KF/x source directory
    pub kfx_path: PathBuf,
    /// The path to output build artifacts
    pub output_path: PathBuf,
    /// An optional path to an existing Xen deb to use to produce a bundled deb
    pub xen_deb: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    init_logging()?;

    match args.action {
        Action::Dependencies => {
            install_apt_deps()?;
            install_golang()?;
        }
        Action::BuildXen(command) => {
            let xen_path = command.xen_path;
            let output_path = command.output_path;
            create_dir_all(&output_path)?;

            configure_xen(&xen_path)?;
            build_xen(&xen_path)?;
            make_deb(&xen_path, &output_path)?;
        }
        Action::BuildKFx(command) => {
            let kfx_path = command.kfx_path;
            let output_path = command.output_path;
            let build_dir = TempDir::new("build")?;
            let build_path = build_dir.path().to_path_buf();

            create_dir_all(&output_path)?;

            build_dwarf2json(&kfx_path)?;
            build_libvmi(&kfx_path, &build_path)?;
            build_capstone(&kfx_path, &build_path)?;
            build_libxdc(&kfx_path, &build_path)?;
            build_kfx(&kfx_path, &build_path)?;
            match command.xen_deb {
                Some(xen_deb) => make_bundle_deb(&output_path, &build_path, &xen_deb)?,
                _ => {}
            }
            make_kfx_deb(&output_path, &build_path)?;
        }
    }

    Ok(())
}

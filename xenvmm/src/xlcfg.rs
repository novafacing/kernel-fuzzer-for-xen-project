//! This module implements the xen.cfg file format and allows reading, writing, and building
//! xen cfg files with code.
//!

use std::{
    collections::HashMap,
    default,
    fmt::{Display, Formatter, Result},
    net::Ipv4Addr,
    path::PathBuf,
};

use derive_builder::Builder;
use macaddr::MacAddr6;
use serde_json::to_string;

/// The type of guest VM
#[derive(Clone)]
pub enum GuestType {
    /// Paravirtualized guest aware of the Xen host
    PV,
    /// Similar to HVM but without most emulated devices, requires PVH aware kernel
    PVH,
    /// Hardware virtual machine with full emulated BIOS and devices
    HVM,
}

impl Display for GuestType {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "{}",
            match self {
                GuestType::PV => "pv",
                GuestType::PVH => "pvh",
                GuestType::HVM => "hvm",
            }
        )
    }
}

/// Actions that can be taken on events such as poweroff or restart
#[derive(Clone)]
pub enum EventAction {
    Destroy,
    Restart,
    RenameRestart,
    Preserve,
    CoredumpDestroy,
    CoredumpRestart,
    SoftReset,
}

impl Display for EventAction {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "{}",
            match self {
                EventAction::Destroy => "destroy",
                EventAction::Restart => "restart",
                EventAction::RenameRestart => "rename-restart",
                EventAction::Preserve => "preserve",
                EventAction::CoredumpDestroy => "coredump-destroy",
                EventAction::CoredumpRestart => "coredump-restart",
                EventAction::SoftReset => "soft-reset",
            }
        )
    }
}

#[derive(Clone)]
pub enum PvFirmware {
    PvGrub32,
    PvGrub64,
}

impl Display for PvFirmware {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "{}",
            match self {
                PvFirmware::PvGrub32 => "pvgrub32",
                PvFirmware::PvGrub64 => "pvgrub64",
            }
        )
    }
}

#[derive(Clone, Default)]
enum XlDiskFormat {
    #[default]
    Raw,
    Qcow,
    Qcow2,
    Vhd,
    Qed,
}

impl Display for XlDiskFormat {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "{}",
            match self {
                XlDiskFormat::Raw => "raw",
                XlDiskFormat::Qcow => "qcow",
                XlDiskFormat::Qcow2 => "qcow2",
                XlDiskFormat::Vhd => "vhd",
                XlDiskFormat::Qed => "qed",
            }
        )
    }
}

#[derive(Clone)]
enum XlDiskVdev {
    Xvd(String),
    Hd(String),
    Sd(String),
}

impl Default for XlDiskVdev {
    fn default() -> Self {
        Self::Xvd("a".to_string())
    }
}

impl Display for XlDiskVdev {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "{}",
            match self {
                XlDiskVdev::Xvd(id) => format!("xvd{}", id),
                XlDiskVdev::Hd(id) => format!("hd{}", id),
                XlDiskVdev::Sd(id) => format!("sd{}", id),
            }
        )
    }
}

#[derive(Clone, Default)]
enum XlDiskAccess {
    #[default]
    RW,
    RO,
}

impl Display for XlDiskAccess {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "{}",
            match self {
                XlDiskAccess::RO => "ro",
                XlDiskAccess::RW => "rw",
            }
        )
    }
}

/// Xl Disk configuration format used for specifying disks to boot with
/// See https://xenbits.xen.org/docs/unstable/man/xl-disk-configuration.5.html
#[derive(Builder, Clone)]
struct XlDiskCfg {
    /// The path on disk to the Xl disk
    target: PathBuf,
    /// The disk format
    format: XlDiskFormat,
    /// Virtual device seen by the guest
    vdev: XlDiskVdev,
    /// Access
    access: XlDiskAccess,
    /// Whether this device is a cdrom
    cdrom: bool,
    /// Target translator script
    script: Option<PathBuf>,
}

impl Display for XlDiskCfg {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let opt = format!(
            "format={},vdev={},access={},{}{}{}",
            self.format.to_string(),
            self.vdev.to_string(),
            self.access.to_string(),
            if self.cdrom { "cdrom," } else { "" },
            match &self.script {
                Some(script) => script.to_string_lossy().to_string(),
                None => "".to_string(),
            },
            self.target.to_string_lossy(),
        );
        write!(f, "{}", opt)
    }
}

#[derive(Clone, Default)]
enum XlVifType {
    #[default]
    Ioemu,
    Vif,
}

impl Display for XlVifType {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "{}",
            match self {
                XlVifType::Ioemu => "ioemu",
                XlVifType::Vif => "vif",
            }
        )
    }
}

#[derive(Clone, Default)]
enum XlVifModel {
    #[default]
    Rtl8139,
    E1000,
    Other(String),
}

impl Display for XlVifModel {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "{}",
            match self {
                XlVifModel::Rtl8139 => "rtl8139",
                XlVifModel::E1000 => "e1000",
                XlVifModel::Other(other) => other,
            }
        )
    }
}

#[derive(Builder, Clone)]
struct XlNetCfg {
    /// The MAC address to use in the guest
    mac: Option<MacAddr6>,
    /// The bridge interface to use in the guest
    bridge: Option<String>,
    /// Name of network interface the VIF should communicate with
    gatewaydev: Option<String>,
    /// Type of device to use for HVM guests
    type_: Option<XlVifType>,
    /// Model of device to use for HVM guests
    model: Option<XlVifModel>,
    /// Name of the backend device for the virtual device
    vifname: Option<String>,
    /// Script to run to configure the device and add it to the correct bridge
    /// defaults to `/etc/xen/scripts/vif-bridge`
    script: Option<PathBuf>,
    /// IP Address to use for the guest VM
    ip: Option<Ipv4Addr>,
}

impl Display for XlNetCfg {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let mut options = HashMap::new();
        if let Some(mac) = self.mac {
            options.insert("mac", mac.to_string());
        }
        if let Some(bridge) = &self.bridge {
            options.insert("bridge", bridge.to_string());
        }
        if let Some(gatewaydev) = &self.gatewaydev {
            options.insert("gatewaydev", gatewaydev.to_string());
        }
        if let Some(type_) = &self.type_ {
            options.insert("type", type_.to_string());
        }
        if let Some(model) = &self.model {
            options.insert("model", model.to_string());
        }
        if let Some(vifname) = &self.vifname {
            options.insert("vifname", vifname.to_string());
        }
        if let Some(script) = &self.script {
            options.insert("script", script.to_string_lossy().to_string());
        }
        if let Some(ip) = self.ip {
            options.insert("ip", ip.to_string());
        }
        write!(
            f,
            "{}",
            options
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                // Sadly, iter_intersperse is still unstable
                .collect::<Vec<_>>()
                .join(",")
        )
    }
}

#[derive(Clone, Default)]
enum XlVgaDev {
    None,
    #[default]
    StdVga,
    Cirrus,
    Qxl,
}

impl Display for XlVgaDev {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "{}",
            match self {
                XlVgaDev::None => "none",
                XlVgaDev::StdVga => "stdvga",
                XlVgaDev::Cirrus => "cirrus",
                XlVgaDev::Qxl => "qxl",
            }
        )
    }
}

/// Xl.Cfg format, see https:///xenbits.xen.org/docs/unstable/man/xl.cfg.5.html for more
/// details
#[derive(Builder)]
struct XlCfg {
    /// The name of the virtual machine, must be unique (or at least not currently extant)
    name: String,
    /// The guest type of the virtual machine
    /// Reserved name, sorry :)
    type_: GuestType,
    /// Put the guest's vCPUs into this named pool
    pool: Option<String>,
    /// Number of vCPUs this guest has, for KF/x VMs this must be 1
    vcpus: Option<i64>,
    /// Maximum number of vCPUs the guest is allowed to utilize
    maxvcpus: Option<i64>,
    /// CPU list that the guest is allowed to use.
    cpus: Option<String>,
    /// Same as `cpus` but for soft affinity instead of pinning
    cpus_soft: Option<String>,
    /// Weight for scheduling
    cpu_weight: Option<i64>,
    /// % CPU utilization cap a VM is allowed
    cap: Option<i64>,
    /// Megabytes of memory a guest starts with
    memory: Option<i64>,
    /// Maximum megabytes of memory a guest is allowed to acquire
    maxmem: Option<i64>,
    /// VNUMA configuration, see spec for details
    vnuma: Option<Vec<Vec<String>>>,
    /// Action to take on power off (defaults to destroy)
    on_poweroff: Option<EventAction>,
    /// Action to take on reboot (defaults to destroy)
    on_reboot: Option<EventAction>,
    /// Action to take if Xen watchdog timeout shuts down the VM (defaults to destroy)
    on_watchdog: Option<EventAction>,
    /// Action to take if the VM crashes (defaults to destroy)
    on_crash: Option<EventAction>,
    /// Action to take on soft reset (defaults to soft-reset)
    on_soft_reset: Option<EventAction>,
    /// Kernel to use for direct boot
    kernel: Option<PathBuf>,
    /// Ramdisk (initramfs) to use for direct boot
    ramdisk: Option<PathBuf>,
    /// Command line to append to the kernel command line
    cmdline: Option<String>,
    /// Appends 'root=XXXXX' to the kernel command line
    root: Option<String>,
    /// String that is appended to the kernel command line
    extra: Option<String>,
    /// Disks that should be provided to the guest
    disk: Vec<XlDiskCfg>,
    /// Virtual network interfaces that should be provided to the guest
    vif: Vec<XlNetCfg>,
    /// A usb device to add. Generally, you want "tablet"
    usbdevice: Vec<String>,
    /// VGA device to emulate
    vga: Option<XlVgaDev>,
    /// Megabytes of VRAM to provide
    videoram: Option<u32>,
    /// Whether to enable VNC or not
    vnc: Option<bool>,
    // Address to listen on for VNC connections
    vnclisten: Option<Ipv4Addr>,
    // TODO:
    // pvshim
    // pvshim_path
    // pvshim_cmdline
    // pvshim_extra
    // uuid
    // seclabel
    // init_seclabel
    // max_grant_frames
    // max_maptrack_frames
    // max_grant_version
    // nomigrate
    // driver_domain
    // device_tree
    // passthrough
    // xend_suspend_evtchn_compat
    // vmtrace_buf_kb
    // vpmu
    // vtpm
    // p9
    // pvcalls
    // vfb
    // channel
    // rdm
    // usbctrl
    // usbdev
    // pci
    // pci_permissive
    // pci_msitranslate
    // pci_seize
    // pci_power_mgmt
    // gfx_passthru
    // rdm_mem_boundary
    // dtdev
    // ioports
    // iomem
    // irqs
    // max_event_channels
    // vdispl
    // dm_restrict
    // device_model_user
    // vsnd
    // vkb
    // tee
    // bootloader
    // bootloader_args
    // e820_host
    // boot
    // hdtype
    // hap
    // oos
    // shadow_memory
    // bios
    // bios_path_override
    // pae
    // acpi
    // acpi_s3
    // acpi_s4
    // acpi_laptop_slate
    // apic
    // nx
    // hpet
    // altp2m
    // altp2mhvm
    // nestedhvm
    // cpuid
    // acpi_firmware
    // smbios_firmware
    // ms_vm_genid
    // tsc_mode
    // localtime
    // rtc_timeoffset
    // vpt_align
    // timer_mode
    // mmio_hole
    // xen_platform_pci
    // viridian
    // vncdisplay
    // vncunused
    // vncpassword
    // keymap
    // sdl
    // opengl
    // nographic
    // spice
    // spicehost
    // spiceport
    // spicetls_port
    // spicedisable_ticketing
    // spicepasswd
    // spiceagent_mouse
    // spicevdagent
    // spice_clipboard_sharing
    // spiceusbredirection
    // spice_image_compression
    // spice_streaming_video
    // serial
    // soundhw
    // vkb_device
    // usb
    // usbversion
    // vendor_device
    // nestedhvm
    // bootloader
    // bootloader_args
    // timer_mode
    // hap
    // oos
    // shadow_memory
    // device_model_version
    // device_model_override
    // stubdomain_kernel
    // stubdomain_cmdline
    // stubdomain_ramdisk
    // stubdomain_memory
    // device_model_stubdomain_override
    // device_model_stubdomain_seclabel
    // device_model_args
    // device_model_args_pv
    // device_model_args_hvm
    // gic_version
    // vuart
    // mca_caps
    // msr_relaxed
}

impl Display for XlCfg {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let mut options = HashMap::new();
        options.insert("name", self.name.clone());
        options.insert("type", self.type_.to_string());
        if let Some(pool) = &self.pool {
            options.insert("pool", pool.to_string());
        }
        if let Some(vcpus) = self.vcpus {
            options.insert("vcpus", vcpus.to_string());
        }
        if let Some(maxvcpus) = self.maxvcpus {
            options.insert("maxvcpus", maxvcpus.to_string());
        }
        if let Some(cpus) = &self.cpus {
            options.insert("cpus", cpus.to_string());
        }
        if let Some(cpus_soft) = &self.cpus {
            options.insert("cpus_soft", cpus_soft.to_string());
        }
        if let Some(cpu_weight) = self.cpu_weight {
            options.insert("cpu_weight", cpu_weight.to_string());
        }
        if let Some(cap) = self.cap {
            options.insert("cap", cap.to_string());
        }
        if let Some(memory) = self.memory {
            options.insert("memory", memory.to_string());
        }
        if let Some(maxmem) = self.maxmem {
            options.insert("maxmem", maxmem.to_string());
        }
        if let Some(vnuma) = &self.vnuma {
            options.insert("vnuma", to_string(vnuma).unwrap());
        }
        if let Some(on_poweroff) = self.on_poweroff {
            options.insert("on_poweroff", on_poweroff.to_string());
        }
        if let Some(on_reboot) = self.on_reboot {
            options.insert("on_reboot", on_reboot.to_string());
        }
        if let Some(on_watchdog) = self.on_watchdog {
            options.insert("on_watchdog", on_watchdog.to_string());
        }
        if let Some(on_crash) = self.on_crash {
            options.insert("on_crash", on_crash.to_string());
        }

        write!(
            f,
            "{}",
            options
                .iter()
                .map(|(k, v)| format!("{} = {}", k, v))
                .collect::<Vec<_>>()
                .join("; ")
        )
    }
}

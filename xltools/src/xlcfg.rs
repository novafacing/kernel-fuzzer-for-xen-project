//! This module implements the xen.cfg file format and allows reading, writing, and building
//! xen cfg files with code.
//!
use std::{
    collections::BTreeMap,
    fmt::{self, Display, Formatter},
    net::Ipv4Addr,
    path::PathBuf,
};

use derive_builder::Builder;
use macaddr::MacAddr6;
use serde::{Serialize, Serializer};
use serde_json::to_string;

/// The type of guest VM
#[derive(Clone, Default)]
pub enum XlGuestType {
    /// Paravirtualized guest aware of the Xen host
    PV,
    /// Similar to HVM but without most emulated devices, requires PVH aware kernel
    PVH,
    /// Hardware virtual machine with full emulated BIOS and devices
    #[default]
    HVM,
}

impl Serialize for XlGuestType {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl Display for XlGuestType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                XlGuestType::PV => "pv",
                XlGuestType::PVH => "pvh",
                XlGuestType::HVM => "hvm",
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

impl Serialize for EventAction {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl Display for EventAction {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
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

impl Serialize for PvFirmware {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl Display for PvFirmware {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
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
pub enum XlDiskFormat {
    #[default]
    Raw,
    Qcow,
    Qcow2,
    Vhd,
    Qed,
}

impl Serialize for XlDiskFormat {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl Display for XlDiskFormat {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
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
pub enum XlDiskVdev {
    Xvd(String),
    Hd(String),
    Sd(String),
}

impl Serialize for XlDiskVdev {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl Default for XlDiskVdev {
    fn default() -> Self {
        Self::Xvd("a".to_string())
    }
}

impl Display for XlDiskVdev {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
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
pub enum XlDiskAccess {
    #[default]
    RW,
    RO,
}

impl Serialize for XlDiskAccess {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl Display for XlDiskAccess {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
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
#[derive(Builder, Clone, Default)]
#[builder(setter(into, strip_option), default)]
pub struct XlDiskCfg {
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

impl Serialize for XlDiskCfg {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl Display for XlDiskCfg {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let opt = format!(
            "format={},vdev={},access={},{}{}{}",
            self.format.to_string(),
            self.vdev.to_string(),
            self.access.to_string(),
            if self.cdrom { "devtype=cdrom," } else { "" },
            match &self.script {
                Some(script) => format!("script={}", script.to_string_lossy().to_string()),
                None => "".to_string(),
            },
            format!("target={}", self.target.to_string_lossy()),
        );
        write!(f, "{}", opt)
    }
}

#[derive(Clone, Default)]
pub enum XlVifType {
    #[default]
    Ioemu,
    Vif,
}

impl Serialize for XlVifType {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl Display for XlVifType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
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
pub enum XlVifModel {
    #[default]
    Rtl8139,
    E1000,
    Other(String),
}

impl Serialize for XlVifModel {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl Display for XlVifModel {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
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

#[derive(Clone, Default)]
pub struct XlMacAddr6(MacAddr6);

impl Serialize for XlMacAddr6 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl Display for XlMacAddr6 {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.to_string())
    }
}

#[derive(Builder, Clone, Default)]
#[builder(setter(into, strip_option), default)]
pub struct XlNetCfg {
    /// The MAC address to use in the guest
    mac: Option<XlMacAddr6>,
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

impl Serialize for XlNetCfg {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl Display for XlNetCfg {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut options = BTreeMap::new();
        if let Some(mac) = &self.mac {
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
pub enum XlVgaDev {
    None,
    #[default]
    StdVga,
    Cirrus,
    Qxl,
}

impl Serialize for XlVgaDev {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl Display for XlVgaDev {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
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

#[derive(Clone)]
pub enum XlRemoteHost {
    Hostname(String),
    Ip(Ipv4Addr),
}

impl Serialize for XlRemoteHost {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl Display for XlRemoteHost {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                XlRemoteHost::Hostname(hostname) => hostname.to_string(),
                XlRemoteHost::Ip(ip) => ip.to_string(),
            }
        )
    }
}

#[derive(Clone)]
pub struct XlUdpConn {
    remote_host: Option<XlRemoteHost>,
    remote_port: u16,
    src_ip: Option<Ipv4Addr>,
    src_port: Option<u16>,
}

impl Display for XlUdpConn {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "udp:{}:{}{}",
            match &self.remote_host {
                Some(remote_host) => remote_host.to_string(),
                None => "".to_string(),
            },
            self.remote_port,
            if let Some(src_port) = self.src_port {
                if let Some(src_ip) = self.src_ip {
                    format!("@{}:{}", src_ip, src_port)
                } else {
                    format!("@:{}", src_port)
                }
            } else {
                "".to_string()
            },
        )
    }
}

#[derive(Clone)]
pub struct XlTcpConn {
    remote_host: Option<XlRemoteHost>,
    remote_port: u16,
    server: bool,
    wait: bool,
    nodelay: bool,
    reconnect: Option<u32>,
}

impl Display for XlTcpConn {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "tcp:{}:{}{}{}{}{}",
            match &self.remote_host {
                Some(remote_host) => remote_host.to_string(),
                None => "".to_string(),
            },
            self.remote_port,
            if self.server { ",server=on" } else { "" },
            if self.wait { ",wait=on" } else { "" },
            if self.nodelay { ",nodelay=on" } else { "" },
            if let Some(reconnect) = self.reconnect {
                format!(",reconnect={}", reconnect)
            } else {
                "".to_string()
            },
        )
    }
}

#[derive(Clone)]
pub struct XlTelnetConn {
    remote_host: XlRemoteHost,
    remote_port: u16,
    server: bool,
    wait: bool,
    nodelay: bool,
}

impl Display for XlTelnetConn {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "telnet:{}:{}{}{}{}",
            self.remote_host,
            self.remote_port,
            if self.server { ",server=on" } else { "" },
            if self.wait { ",wait=on" } else { "" },
            if self.nodelay { ",nodelay=on" } else { "" },
        )
    }
}

#[derive(Clone)]
pub struct XlWebsocketConn {
    remote_host: XlRemoteHost,
    remote_port: u16,
    wait: bool,
    nodelay: bool,
}

impl Display for XlWebsocketConn {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "websocket:{}:{},server=on{}{}",
            self.remote_host,
            self.remote_port,
            if self.wait { ",wait=on" } else { "" },
            if self.nodelay { ",nodelay=on" } else { "" },
        )
    }
}

#[derive(Clone)]
pub struct XlUnixConn {
    path: String,
    server: bool,
    wait: bool,
    reconnect: Option<u32>,
}

impl Display for XlUnixConn {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unix:{}{}{}{}",
            self.path,
            if self.server { ",server=on" } else { "" },
            if self.wait { ",wait=on" } else { "" },
            if let Some(reconnect) = self.reconnect {
                format!(",reconnect={}", reconnect)
            } else {
                "".to_string()
            },
        )
    }
}

#[derive(Clone)]
pub enum XlSerialDev {
    Vc(Option<(usize, usize)>),
    Pty,
    None,
    Null,
    Chardev(String),
    Dev(String),
    Parport(usize),
    File(String),
    Stdio,
    Pipe(String),
    Com(usize),
    Udp(XlUdpConn),
    Tcp(XlTcpConn),
    Telnet(XlTelnetConn),
    Websocket(XlWebsocketConn),
    Unix(XlUnixConn),
    Mon(String),
    Braille,
    MsMouse,
}

impl Serialize for XlSerialDev {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl Display for XlSerialDev {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            XlSerialDev::Vc(Some((x, y))) => write!(f, "vc:{}:{}", x, y),
            XlSerialDev::Vc(None) => write!(f, "vc"),
            XlSerialDev::Pty => write!(f, "pty"),
            XlSerialDev::None => write!(f, "none"),
            XlSerialDev::Null => write!(f, "null"),
            XlSerialDev::Chardev(name) => write!(f, "chardev:{}", name),
            XlSerialDev::Dev(name) => write!(f, "dev:{}", name),
            XlSerialDev::Parport(port) => write!(f, "parport:{}", port),
            XlSerialDev::File(path) => write!(f, "file:{}", path),
            XlSerialDev::Stdio => write!(f, "stdio"),
            XlSerialDev::Pipe(path) => write!(f, "pipe:{}", path),
            XlSerialDev::Com(port) => write!(f, "com:{}", port),
            XlSerialDev::Udp(conn) => write!(f, "udp:{}", conn),
            XlSerialDev::Tcp(conn) => write!(f, "tcp:{}", conn),
            XlSerialDev::Telnet(conn) => write!(f, "telnet:{}", conn),
            XlSerialDev::Websocket(conn) => write!(f, "websocket:{}", conn),
            XlSerialDev::Unix(conn) => write!(f, "unix:{}", conn),
            XlSerialDev::Mon(path) => write!(f, "mon:{}", path),
            XlSerialDev::Braille => write!(f, "braille"),
            XlSerialDev::MsMouse => write!(f, "msmouse"),
        }
    }
}

/// Xl.Cfg format, see https:///xenbits.xen.org/docs/unstable/man/xl.cfg.5.html for more
/// details
#[derive(Builder, Default)]
#[builder(setter(into, strip_option), default)]
pub struct XlCfg {
    /// The name of the virtual machine, must be unique (or at least not currently extant)
    name: String,
    /// The guest type of the virtual machine
    /// Reserved name, sorry :)
    type_: XlGuestType,
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
    vnclisten: Option<(Ipv4Addr, u16)>,
    /// Serial device to provide to the guest
    serial: Option<XlSerialDev>, // TODO:
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
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut options = BTreeMap::new();
        options.insert("name", to_string(&self.name).unwrap());
        options.insert("type", to_string(&self.type_).unwrap());
        if let Some(pool) = &self.pool {
            options.insert("pool", to_string(&pool).unwrap());
        }
        if let Some(vcpus) = self.vcpus {
            options.insert("vcpus", to_string(&vcpus).unwrap());
        }
        if let Some(maxvcpus) = self.maxvcpus {
            options.insert("maxvcpus", to_string(&maxvcpus).unwrap());
        }
        if let Some(cpus) = &self.cpus {
            options.insert("cpus", to_string(&cpus).unwrap());
        }
        if let Some(cpus_soft) = &self.cpus_soft {
            options.insert("cpus_soft", to_string(&cpus_soft).unwrap());
        }
        if let Some(cpu_weight) = self.cpu_weight {
            options.insert("cpu_weight", to_string(&cpu_weight).unwrap());
        }
        if let Some(cap) = self.cap {
            options.insert("cap", to_string(&cap).unwrap());
        }
        if let Some(memory) = self.memory {
            options.insert("memory", to_string(&memory).unwrap());
        }
        if let Some(maxmem) = self.maxmem {
            options.insert("maxmem", to_string(&maxmem).unwrap());
        }
        if let Some(vnuma) = &self.vnuma {
            options.insert("vnuma", to_string(vnuma).unwrap());
        }
        if let Some(on_poweroff) = &self.on_poweroff {
            options.insert("on_poweroff", to_string(&on_poweroff).unwrap());
        }
        if let Some(on_reboot) = &self.on_reboot {
            options.insert("on_reboot", to_string(&on_reboot).unwrap());
        }
        if let Some(on_watchdog) = &self.on_watchdog {
            options.insert("on_watchdog", to_string(&on_watchdog).unwrap());
        }
        if let Some(on_crash) = &self.on_crash {
            options.insert("on_crash", to_string(&on_crash).unwrap());
        }
        if let Some(on_soft_reset) = &self.on_soft_reset {
            options.insert("on_soft_reset", to_string(&on_soft_reset).unwrap());
        }
        if let Some(kernel) = &self.kernel {
            options.insert("kernel", to_string(&kernel).unwrap());
        }
        if let Some(ramdisk) = &self.ramdisk {
            options.insert("ramdisk", to_string(&ramdisk).unwrap());
        }
        if let Some(cmdline) = &self.cmdline {
            options.insert("cmdline", to_string(&cmdline).unwrap());
        }
        if let Some(root) = &self.root {
            options.insert("root", to_string(&root).unwrap());
        }
        if let Some(extra) = &self.extra {
            options.insert("extra", to_string(&extra).unwrap());
        }
        if !self.disk.is_empty() {
            options.insert("disk", to_string(&self.disk).unwrap());
        }
        if !self.vif.is_empty() {
            options.insert("vif", to_string(&self.vif).unwrap());
        }
        if !self.usbdevice.is_empty() {
            options.insert("usbdevice", to_string(&self.usbdevice).unwrap());
        }
        if let Some(vga) = &self.vga {
            options.insert("vga", to_string(&vga).unwrap());
        }
        if let Some(videoram) = self.videoram {
            options.insert("videoram", to_string(&videoram).unwrap());
        }
        if let Some(vnc) = &self.vnc {
            options.insert("vnc", if *vnc { 1 } else { 0 }.to_string());
        }
        if let Some((addr, port)) = &self.vnclisten {
            options.insert(
                "vnclisten",
                to_string(&format!("{}:{}", addr.to_string(), port)).unwrap(),
            );
        }
        if let Some(serial) = &self.serial {
            options.insert("serial", to_string(&serial).unwrap());
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

#[test]
fn test_basic() {
    let cfg = XlCfgBuilder::default()
        .name("agent".to_string())
        .type_(XlGuestType::HVM)
        .build()
        .unwrap();

    assert_eq!(
        cfg.to_string(),
        r#"name = "agent"; type = "hvm""#.to_string()
    );
}

#[test]
fn test_win_agent() {
    let img = XlDiskCfgBuilder::default()
        .target(PathBuf::from("/test/tmp/disk1.img"))
        .format(XlDiskFormat::Raw)
        .vdev(XlDiskVdev::Xvd("a".to_string()))
        .access(XlDiskAccess::RW)
        .build()
        .unwrap();

    let cd = XlDiskCfgBuilder::default()
        .target(PathBuf::from("/test/tmp/disk2.iso"))
        .format(XlDiskFormat::Raw)
        .cdrom(true)
        .vdev(XlDiskVdev::Hd("c".to_string()))
        .build()
        .unwrap();

    let cfg = XlCfgBuilder::default()
        .name("agent".to_string())
        .type_(XlGuestType::HVM)
        .memory(4096)
        .vcpus(1)
        .usbdevice(vec!["tablet".to_string()])
        .vga(XlVgaDev::StdVga)
        .videoram(32u32)
        .serial(XlSerialDev::Pty)
        .vif(vec![XlNetCfgBuilder::default()
            .bridge("xenbr0".to_string())
            .build()
            .unwrap()])
        .disk(vec![img, cd])
        .vnc(true)
        .vnclisten((Ipv4Addr::new(0, 0, 0, 0), 3))
        .build()
        .unwrap();

    assert_eq!(
        cfg.to_string(),
        r#"disk = ["format=raw,vdev=xvda,access=rw,target=/test/tmp/disk1.img","format=raw,vdev=hdc,access=rw,devtype=cdrom,target=/test/tmp/disk2.iso"]; memory = 4096; name = "agent"; serial = "pty"; type = "hvm"; usbdevice = ["tablet"]; vcpus = 1; vga = "stdvga"; videoram = 32; vif = ["bridge=xenbr0"]; vnc = 1; vnclisten = "0.0.0.0:3""#.to_string()
    );
}

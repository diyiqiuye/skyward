use anyhow::{ensure, Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// Recovery ramdisk files Skyward reads, adds or changes, as archive paths
/// (they sit under `/` once the kernel unpacks the ramdisk).
pub mod ramdisk {
    /// The device configuration, copied from `devices/<codename>.toml`.
    pub const CONFIG: &str = "skyward/device.toml";
    /// The UI typeface.
    pub const FONT: &str = "skyward/MiSansVF.ttf";
    /// The terminal's monospaced typeface.
    pub const MONO: &str = "skyward/KirkMono-Regular.ttf";
    /// The monospaced typeface's licence, which travels with every copy of
    /// it.
    pub const MONO_LICENSE: &str = "skyward/KirkMono-OFL.txt";
    /// The recovery service binary init starts.
    pub const RECOVERY: &str = "system/bin/recovery";
    /// The first program of a system installed beside the installed one,
    /// which Skyward puts in that system's init_boot image: linked
    /// statically, since it runs before any system's libraries are there.
    pub const SYSTEM_INIT: &str = "skyward/system-init";
    /// The commands install scripts written for other recoveries run that
    /// Skyward answers itself: 7za, avbctl, resetprop and zstd, which a
    /// stock recovery does not carry, and unzip, whose stock version holds
    /// all that `-p` takes out in memory. Each is a link to the recovery
    /// service binary beside it, which runs the command it is started as.
    pub const COMMANDS: [&str; 5] =
        ["system/bin/7za", "system/bin/avbctl", "system/bin/resetprop", "system/bin/unzip", "system/bin/zstd"];
    /// The first property file init loads in recovery mode.
    pub const PROP_DEFAULT: &str = "prop.default";
    /// The scheduling profiles libprocessgroup reads.
    pub const TASK_PROFILES: &str = "system/etc/task_profiles.json";
    /// The init rules that set up file transfer's USB function.
    pub const MTP_RC: &str = "system/etc/init/skyward-mtp.rc";

    /// Whether the running root is Skyward's recovery ramdisk.
    pub fn in_recovery() -> bool {
        std::path::Path::new("/").join(CONFIG).exists()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeviceConfig {
    pub device: Device,
    #[serde(default)]
    pub display: Display,
    #[serde(default)]
    pub modules: Modules,
    #[serde(default)]
    pub touch: Touch,
    #[serde(default)]
    pub partitions: Partitions,
    /// Where the firmware the phone's drivers ask for is, on phones whose
    /// recovery leaves it out of their reach.
    #[serde(default)]
    pub firmware: Option<FirmwarePartition>,
    #[serde(default, rename = "service")]
    pub services: Vec<Service>,
    /// The vibration motor's click, on phones that have one.
    #[serde(default)]
    pub haptics: Option<Haptics>,
    /// The recovery's own logd, on phones whose recovery leaves out
    /// libraries it needs.
    #[serde(default)]
    pub logd: Option<Logd>,
    /// Files the phone's own init hands to an account as it boots, before
    /// the programs that open them start; a recovery runs none of init's
    /// boot actions.
    #[serde(default)]
    pub owners: Vec<Owners>,
    #[serde(default)]
    pub wipe: Wipe,
    #[serde(default)]
    pub backup: Backup,
    /// The phone's WLAN, on phones whose driver the recovery brings up.
    #[serde(default)]
    pub wlan: Option<Wlan>,
    /// Where the phone maker answers which system is the newest, on phones
    /// whose server the recovery knows.
    #[serde(default)]
    pub update: Option<Update>,
    /// The stock build the recovery is made on.
    #[serde(default)]
    pub stock: Option<Stock>,
}

/// The stock build a recovery is made on: the phone maker's full update
/// package of it, the partition the recovery is built on top of, and that
/// partition as it should come out of the package.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Stock {
    /// The build's version, such as OS4.0.4.0.XPACNXM.
    pub version: String,
    /// Where the package is downloaded from, over HTTPS.
    pub package: String,
    /// The partition the image is taken from, as the package names it:
    /// `recovery` on phones that have one, and `boot` on the phones that
    /// keep their recovery in the boot image, as thyme and alioth do.
    #[serde(default = "default_stock_partition")]
    pub partition: String,
    /// That partition's SHA-256, in lowercase hex.
    pub recovery: String,
}

/// The partition a recovery is usually made on, which a package names so.
fn default_stock_partition() -> String {
    "recovery".into()
}

/// What brings the phone's WLAN up inside a recovery, which loads none of
/// it: the driver, what it reads as it starts, and the phone's own
/// wpa_supplicant, which joins networks.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Wlan {
    /// The driver's kernel modules; what they depend on loads first.
    pub modules: Vec<String>,
    /// What is written once they are in, each value to its file, such as
    /// what tells a platform driver it may start the chip.
    #[serde(default)]
    pub writes: Vec<Write>,
    /// Files the driver asks the kernel's firmware loader for that are not
    /// where the loader looks in a recovery.
    #[serde(default)]
    pub firmware: Vec<Firmware>,
    /// The network interface the driver makes.
    pub interface: String,
    /// The phone's own wpa_supplicant.
    pub supplicant: String,
    /// Directories for its LD_LIBRARY_PATH, searched in order and before
    /// the libraries of the vendor interface the phone was built against.
    #[serde(default)]
    pub libraries: Vec<String>,
    /// The service it registers with the service directory as it starts,
    /// which the directory takes only from a program the running system
    /// describes.
    pub service: String,
    /// The page the stock system asks over plain HTTP to tell whether a
    /// network reaches the internet, which answers 204 with nothing.
    pub probe: String,
}

/// A value written to a file, such as a switch in sysfs.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Write {
    pub path: String,
    pub value: String,
}

/// The partition holding the firmware images of a phone's processors and
/// radios, which their drivers ask the kernel's firmware loader for, and of
/// the apps its secure world runs. It is mounted where and as the phone's
/// own table mounts it, and shown as well at the folder whose image folder
/// the recovery's own ueventd looks in.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct FirmwarePartition {
    /// The partition's name, without the slot it is of.
    pub partition: String,
}

/// A file a driver asks the firmware loader for, and where it is.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Firmware {
    /// The name the driver asks for, from the top of the loader's folders.
    pub name: String,
    /// Where the file is: a path in the running recovery, or from the top
    /// of `partition`.
    pub path: String,
    /// A partition the recovery does not keep mounted, which is mounted
    /// read-only while the file is read.
    #[serde(default)]
    pub partition: Option<String>,
}

/// The phone maker's update server, which the stock updater asks about the
/// installed system.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Update {
    /// Where the questions are sent.
    pub server: String,
}

/// What a backup takes in and leaves out beyond what every Android phone
/// keeps the same.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct Backup {
    /// The partitions that keep the baseband's settings, with the phone's
    /// own identity, and the sensors' calibration.
    #[serde(default)]
    pub efs: Vec<String>,
    /// Folders in the data partition, from its top, that the phone's own
    /// secure world keeps in step with itself, as its own storage and the
    /// fingerprint templates: an older copy written back would not open.
    /// A `*` stands for any one name.
    #[serde(default)]
    pub data_skipped: Vec<String>,
}

/// What erasing on the wipe page takes in beyond the places every Android
/// phone keeps the same.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct Wipe {
    /// Folders in the data partition where the phone's own programs keep
    /// their logs and dumps: their files go with the logs and crash
    /// records, the folders staying.
    #[serde(default)]
    pub logs: Vec<String>,
}

/// The data partition, which the folders erasing takes in are in.
const DATA: &str = "/data";

/// The vibration motor, played through the kernel's force-feedback events:
/// its click, a waveform its driver keeps or one made from samples at the
/// motor's resonant frequency, which the driver plays as they are given,
/// and the ticks of the screen going off by the power key, made too.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Haptics {
    /// The driver's number for a click waveform it keeps, on phones whose
    /// driver plays the waveforms it keeps.
    #[serde(default)]
    pub click: Option<i16>,
    /// The file the driver says the motor's resonant frequency in, in
    /// tenths of a hertz whatever its label.
    pub resonance: String,
    pub strength: Strength,
}

/// How strongly the click plays, each as a share of the most its waveform
/// drives the motor with. The strength slider goes from the lightest to the
/// strongest, each step the same factor stronger.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct Strength {
    pub lightest: f32,
    pub strongest: f32,
}

/// What the recovery's own logd, which keeps what programs log for
/// logcat, needs from the installed system before it can start.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Logd {
    /// Libraries of the installed system it loads that the recovery's own
    /// library folder lacks, put there before it is started.
    pub libraries: Vec<String>,
}

/// Files the phone's own init hands to one account as it boots, each given
/// the same mode, as a run of its chown and chmod lines does: device nodes,
/// and the kernel's switches that the phone's own programs open.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Owners {
    pub user: String,
    pub group: String,
    /// The mode init's chmod lines give them, written as an octal number.
    pub mode: u32,
    pub paths: Vec<String>,
}

/// A name as each language the pages speak writes it: Chinese in
/// simplified and in traditional characters, and English.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Names {
    pub hans: String,
    pub hant: String,
    pub en: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Device {
    pub codename: String,
    #[serde(default)]
    pub brand: String,
    #[serde(default)]
    pub model: String,
    #[serde(default = "default_true")]
    pub ab: bool,
    /// SoC marketing name shown on the About page.
    #[serde(default)]
    pub soc: Option<Names>,
    /// System property that carries the vendor OS version.
    #[serde(default = "default_os_version_prop")]
    pub os_version_prop: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Display {
    #[serde(default)]
    pub width: u32,
    #[serde(default)]
    pub height: u32,
    #[serde(default)]
    pub density: u32,
    /// Marketed screen diagonal in inches.
    #[serde(default)]
    pub diagonal: Option<f32>,
    /// How round the screen's corners are, as a radius in pixels; a page
    /// sliding over another rounds its leading corners to it.
    #[serde(default)]
    pub corner_radius: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Modules {
    /// Partitions the phone keeps its kernel modules on, searched in order.
    #[serde(default = "default_module_sources")]
    pub sources: Vec<String>,
    /// Modules the recovery loads at startup that first-stage init leaves
    /// out, such as the touchscreen driver. Dependencies come from the
    /// source's modules.dep and load first.
    #[serde(default)]
    pub load: Vec<String>,
}

impl Default for Modules {
    fn default() -> Self {
        Self { sources: default_module_sources(), load: Vec::new() }
    }
}

/// The touchscreen, on phones whose touch controller is to be told
/// something before a recovery can read touches from it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct Touch {
    /// What is written once the touchscreen's input device has appeared,
    /// each value to its file: the driver makes that device as the last
    /// step of starting the controller.
    #[serde(default)]
    pub writes: Vec<Write>,
}

/// Partitions as the phone itself mounts them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct Partitions {
    /// Partitions the phone's own programs and drivers use where its own
    /// table mounts them, mounted there as that table says before any of
    /// them starts.
    #[serde(default)]
    pub mounted: Vec<String>,
}

/// A program from the phone's own partitions that the recovery keeps
/// running, such as the touch processing of a panel that hands raw frames
/// to the host.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Service {
    pub path: String,
    /// What the program is given after its own path, as init gives a
    /// service the words after its path.
    #[serde(default)]
    pub args: Vec<String>,
    /// Directories for LD_LIBRARY_PATH, searched in order.
    #[serde(default)]
    pub libraries: Vec<String>,
    /// The program runs once, to its end, as a service of init's marked
    /// oneshot does: it is not started again, the programs after it start
    /// once it has ended, and it has to end well.
    #[serde(default)]
    pub oneshot: bool,
    /// The program is of the installed system's own build: it is run
    /// through that system's loader, in a place where the files it names
    /// are that system's own and not the recovery's, which are of an
    /// older build and would not hold it up.
    #[serde(default)]
    pub installed: bool,
    /// A property the program sets once it is ready for the rest. Programs
    /// are started in the order they are written, and one that says it has
    /// something to wait for holds up the ones after it.
    #[serde(default)]
    pub ready_property: Option<String>,
    /// A service the program registers with the phone's service directory
    /// once it is ready.
    #[serde(default)]
    pub ready_service: Option<String>,
    /// Services the program registers besides that one, which the
    /// directory takes only when they are described to it as well.
    #[serde(default)]
    pub registers: Vec<String>,
    /// The file that describes what the program registers, where the phone
    /// keeps that file apart from its other descriptions, as one its init
    /// puts in place of another on some builds of the hardware.
    #[serde(default)]
    pub description: Option<String>,
    /// A service the program registers with the other directory, the one
    /// the phone's hardware services of the older kind use. It is written
    /// as the interface and then which of them it is.
    #[serde(default)]
    pub ready_hardware_service: Option<String>,
    /// Where one model's hardware comes in more than one build, the
    /// installed system's own properties say which build it is: the program
    /// runs only where the property has this value.
    #[serde(default)]
    pub when: Option<Property>,
    /// The user the program runs as, as the phone's own init runs it, by
    /// the name Android gives the account; root where none is given. What
    /// the program makes in the data partition is then that user's, as it
    /// is when the phone runs it, and the phone's own copy of the program
    /// can still read and remove it.
    #[serde(default)]
    pub user: Option<String>,
    /// Its group and then the others it is in, as init's group line names
    /// them.
    #[serde(default)]
    pub groups: Vec<String>,
    /// What it may do of what only root may, as init's capabilities line
    /// names it: it keeps these alone, through its change of user.
    #[serde(default)]
    pub capabilities: Vec<String>,
}

/// The kernel's capabilities, by the names init's capabilities lines give
/// them, in the order the kernel numbers them.
const CAPABILITIES: [&str; 41] = [
    "CHOWN",
    "DAC_OVERRIDE",
    "DAC_READ_SEARCH",
    "FOWNER",
    "FSETID",
    "KILL",
    "SETGID",
    "SETUID",
    "SETPCAP",
    "LINUX_IMMUTABLE",
    "NET_BIND_SERVICE",
    "NET_BROADCAST",
    "NET_ADMIN",
    "NET_RAW",
    "IPC_LOCK",
    "IPC_OWNER",
    "SYS_MODULE",
    "SYS_RAWIO",
    "SYS_CHROOT",
    "SYS_PTRACE",
    "SYS_PACCT",
    "SYS_ADMIN",
    "SYS_BOOT",
    "SYS_NICE",
    "SYS_RESOURCE",
    "SYS_TIME",
    "SYS_TTY_CONFIG",
    "MKNOD",
    "LEASE",
    "AUDIT_WRITE",
    "AUDIT_CONTROL",
    "SETFCAP",
    "MAC_OVERRIDE",
    "MAC_ADMIN",
    "SYSLOG",
    "WAKE_ALARM",
    "BLOCK_SUSPEND",
    "AUDIT_READ",
    "PERFMON",
    "BPF",
    "CHECKPOINT_RESTORE",
];

/// The kernel's number for a capability, by the name init's capabilities
/// line gives it.
pub fn capability(name: &str) -> Option<u32> {
    CAPABILITIES.iter().position(|c| *c == name).map(|n| n as u32)
}

/// A property of the installed system, and a value it may have.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Property {
    pub property: String,
    pub value: String,
}

impl Service {
    /// Whether the program, or a directory it loads libraries from, is
    /// under `dir`: without what is mounted there it cannot start again.
    pub fn uses(&self, dir: &Path) -> bool {
        Path::new(&self.path).starts_with(dir) || self.libraries.iter().any(|l| Path::new(l).starts_with(dir))
    }

    /// Whether the program is this phone's to run, by the installed
    /// system's own properties.
    pub fn runs_on(&self, props: &HashMap<String, String>) -> bool {
        self.when.as_ref().is_none_or(|p| props.get(&p.property) == Some(&p.value))
    }
}

fn default_true() -> bool {
    true
}

fn default_os_version_prop() -> String {
    "ro.build.display.id".into()
}

fn default_module_sources() -> Vec<String> {
    vec!["vendor_boot".into(), "vendor_dlkm".into(), "vendor".into()]
}

/// The phone a device configuration is for, read on its own: the
/// configuration a newer build of Skyward carries may hold what this build
/// does not read.
pub fn codename(text: &str) -> Result<String> {
    #[derive(Deserialize)]
    struct Named {
        device: NamedDevice,
    }
    #[derive(Deserialize)]
    struct NamedDevice {
        codename: String,
    }
    let named: Named = toml::from_str(text).context("parse device config")?;
    Ok(named.device.codename)
}

impl DeviceConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let p = path.as_ref();
        let text = std::fs::read_to_string(p)
            .with_context(|| format!("read config {}", p.display()))?;
        Self::parse(&text)
    }

    pub fn parse(text: &str) -> Result<Self> {
        let cfg: DeviceConfig = toml::from_str(text).context("parse device config")?;
        for log in &cfg.wipe.logs {
            let path = Path::new(log);
            let plain = path.components().all(|c| matches!(c, std::path::Component::RootDir | std::path::Component::Normal(_)));
            ensure!(
                path.has_root() && plain && path.starts_with(DATA) && path != Path::new(DATA),
                "[wipe] logs: {log} is not a folder inside {DATA}"
            );
        }
        for name in &cfg.backup.efs {
            ensure!(!name.is_empty() && !name.contains('/'), "[backup] efs: {name} is not a partition's name");
        }
        if let Some(firmware) = &cfg.firmware {
            let name = &firmware.partition;
            ensure!(!name.is_empty() && !name.contains('/'), "[firmware] partition: {name} is not a partition's name");
        }
        for name in &cfg.partitions.mounted {
            ensure!(!name.is_empty() && !name.contains('/'), "[partitions] mounted: {name} is not a partition's name");
        }
        for s in cfg.services.iter().filter(|s| s.oneshot) {
            let waits = s.ready_property.is_some() || s.ready_service.is_some() || s.ready_hardware_service.is_some();
            ensure!(
                !waits && s.registers.is_empty() && s.description.is_none(),
                "[[service]] {}: a program run once is ready when it ends, and registers nothing",
                s.path
            );
        }
        for s in &cfg.services {
            ensure!(s.user.is_none() || !s.groups.is_empty(), "[[service]] {}: a program given a user is given its group too", s.path);
            for name in s.user.iter().chain(&s.groups) {
                ensure!(!name.is_empty() && !name.contains(char::is_whitespace), "[[service]] {}: {name:?} is not an account's name", s.path);
            }
            for name in &s.capabilities {
                ensure!(capability(name).is_some(), "[[service]] {}: {name} is not a capability init knows", s.path);
            }
        }
        for folder in &cfg.backup.data_skipped {
            let path = Path::new(folder);
            let plain = path.components().all(|c| matches!(c, std::path::Component::Normal(_)));
            ensure!(plain && !folder.is_empty(), "[backup] data_skipped: {folder} is not a path from the top of {DATA}");
        }
        if let Some(haptics) = &cfg.haptics {
            let file = &haptics.resonance;
            ensure!(Path::new(file).has_root(), "[haptics] resonance: {file} is not an absolute path");
            let Strength { lightest, strongest } = haptics.strength;
            ensure!(
                0.0 < lightest && lightest < strongest && strongest <= 1.0,
                "[haptics] strength: {lightest} and {strongest} are not shares rising from above 0 to at most 1"
            );
        }
        if let Some(logd) = &cfg.logd {
            for library in &logd.libraries {
                let path = Path::new(library);
                let plain = path.components().all(|c| matches!(c, std::path::Component::RootDir | std::path::Component::Normal(_)));
                ensure!(path.has_root() && plain && path.file_name().is_some(), "[logd] libraries: {library} is not an absolute path to a file");
            }
        }
        for owners in &cfg.owners {
            for name in [&owners.user, &owners.group] {
                ensure!(!name.is_empty() && !name.contains(char::is_whitespace), "[[owners]] {name:?} is not an account's name");
            }
            ensure!(owners.mode <= 0o7777, "[[owners]] mode {:o} is not a file's mode", owners.mode);
            ensure!(!owners.paths.is_empty(), "[[owners]] for {}:{} names no file", owners.user, owners.group);
            for file in &owners.paths {
                let path = Path::new(file);
                let plain = path.components().all(|c| matches!(c, std::path::Component::RootDir | std::path::Component::Normal(_)));
                ensure!(path.has_root() && plain && path.file_name().is_some(), "[[owners]] paths: {file} is not an absolute path to a file");
            }
        }
        if let Some(stock) = &cfg.stock {
            let name = stock.package.rsplit('/').next().unwrap_or_default();
            ensure!(
                stock.package.starts_with("https://") && name.ends_with(".zip") && name.contains(&stock.version) && !stock.version.is_empty(),
                "[stock] package: {} is not an HTTPS address of the full package of {}",
                stock.package,
                stock.version
            );
            ensure!(
                !stock.partition.is_empty() && !stock.partition.contains('/') && !stock.partition.contains(char::is_whitespace),
                "[stock] partition: {} is not a partition's name",
                stock.partition
            );
            ensure!(
                stock.recovery.len() == 64 && stock.recovery.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
                "[stock] recovery: {} is not a SHA-256 in lowercase hex",
                stock.recovery
            );
        }
        if let Some(wlan) = &cfg.wlan {
            ensure!(wlan.probe.starts_with("http://"), "[wlan] probe: {} is not a plain HTTP address", wlan.probe);
            for file in &wlan.firmware {
                let name = Path::new(&file.name);
                let plain = name.components().all(|c| matches!(c, std::path::Component::Normal(_)));
                ensure!(plain && !file.name.is_empty(), "[wlan] firmware: {} is not a name from the top of the loader's folders", file.name);
                let path = Path::new(&file.path);
                match &file.partition {
                    Some(partition) => {
                        let plain = path.components().all(|c| matches!(c, std::path::Component::Normal(_)));
                        ensure!(
                            !partition.is_empty() && !partition.contains('/'),
                            "[wlan] firmware: {partition} is not a partition's name"
                        );
                        ensure!(plain && !file.path.is_empty(), "[wlan] firmware: {} is not a path from the top of {partition}", file.path);
                    }
                    None => ensure!(path.has_root(), "[wlan] firmware: {} is not an absolute path", file.path),
                }
            }
        }
        Ok(cfg)
    }
}

#[cfg(test)]
mod tests;

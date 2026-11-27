//! The tests of the phones' own files, and of what a configuration may
//! hold beyond the shortest one, kept apart from the ones about what every
//! file is read as.

use super::*;

#[test]
fn parses_ishtar() {
    let cfg = DeviceConfig::parse(include_str!("../../../../devices/ishtar.toml")).unwrap();
    assert_eq!(cfg.device.codename, "ishtar");
    assert_eq!((cfg.display.width, cfg.display.height, cfg.display.density), (1440, 3200, 560));
    assert_eq!(cfg.modules.load, ["fts_touch_spi"]);
    assert!(cfg.touch.writes.is_empty(), "its own touch processing runs");
    let [touch_owners] = &cfg.owners[..] else { panic!("one run of the touch layer's files") };
    assert_eq!((touch_owners.user.as_str(), touch_owners.group.as_str(), touch_owners.mode), ("system", "system", 0o664));
    assert!(
        touch_owners.paths.iter().any(|p| p == "/dev/xiaomi-touch"),
        "the touch processing, run as system, opens the touch layer's node"
    );
    assert!(cfg.firmware.is_none(), "its recovery mounts the modem partition itself");
    let paths: Vec<&str> = cfg.services.iter().map(|s| s.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "/odm/bin/touch_report",
            "/odm/bin/touchsensor",
            "/vendor/bin/qseecomd",
            "/vendor/bin/hw/android.hardware.security.keymint-service-qti",
            "/mnt/system_ext/bin/hwservicemanager",
            "/vendor/bin/hw/android.hardware.gatekeeper@1.0-service-qti",
            "/vendor/bin/hw/android.hardware.boot@1.2-service",
        ]
    );
    assert_eq!(cfg.services[1].libraries, ["/odm/lib64", "/system/lib64"]);
    assert_eq!(cfg.services[0].ready_property, None, "the touch programs are ready as soon as they run");
    assert_eq!(cfg.services[2].ready_property.as_deref(), Some("vendor.sys.listeners.registered"));
    assert_eq!(
        cfg.services[3].ready_service.as_deref(),
        Some("android.hardware.security.keymint.IKeyMintDevice/default")
    );
    assert_eq!(
        cfg.services[4].ready_property.as_deref(),
        Some("hwservicemanager.ready"),
        "the directory for the hardware services says so itself"
    );
    assert_eq!(
        cfg.services[5].ready_hardware_service.as_deref(),
        Some("android.hardware.gatekeeper@1.0::IGatekeeper/default")
    );
    assert_eq!(cfg.services[6].ready_hardware_service.as_deref(), Some("android.hardware.boot@1.2::IBootControl/default"));
    assert_eq!(cfg.backup.efs, ["modemst1", "modemst2", "fsg", "fsc", "persist"]);
    assert!(cfg.backup.data_skipped.contains(&"vendor/tzstorage".to_string()));
}

#[test]
fn the_phone_is_read_from_a_configuration_this_build_cannot_read_whole() {
    assert_eq!(codename(include_str!("../../../../devices/ishtar.toml")).unwrap(), "ishtar");
    let later = "[device]\ncodename = \"x\"\nab = \"no longer a switch\"\n[display]\nwidth = \"wide\"\n[aurora]\nnew = 1\n";
    assert!(DeviceConfig::parse(later).is_err());
    assert_eq!(codename(later).unwrap(), "x");
    assert!(codename("[display]\nwidth = 1440\n").is_err(), "a configuration that names no phone");
}

#[test]
fn what_a_backup_takes_in_is_named_plainly() {
    let base = "[device]\ncodename = \"x\"\n";
    assert!(DeviceConfig::parse(&format!("{base}[backup]\nefs = [\"by-name/fsg\"]\n")).is_err());
    assert!(DeviceConfig::parse(&format!("{base}[backup]\ndata_skipped = [\"/data/vendor\"]\n")).is_err());
    assert!(DeviceConfig::parse(&format!("{base}[backup]\ndata_skipped = [\"vendor/../system\"]\n")).is_err());
    assert!(DeviceConfig::parse(&format!("{base}[backup]\ndata_skipped = [\"vendor_de/*/fpdata\"]\n")).is_ok());
}

#[test]
fn the_phones_own_logs_are_folders_inside_the_data_partition() {
    let cfg = DeviceConfig::parse(include_str!("../../../../devices/ishtar.toml")).unwrap();
    assert!(cfg.wipe.logs.contains(&"/data/vendor/diag".to_string()));
    assert!(DeviceConfig::parse("[device]\ncodename = \"x\"\n").unwrap().wipe.logs.is_empty(), "a phone may have none");
    for bad in ["/data", "/data/", "/metadata/logs", "data/log", "/data/../system"] {
        let text = format!("[device]\ncodename = \"x\"\n[wipe]\nlogs = [\"{bad}\"]\n");
        assert!(DeviceConfig::parse(&text).is_err(), "{bad} is refused");
    }
}

#[test]
fn ishtar_brings_its_wlan_up_from_its_own_partitions() {
    let cfg = DeviceConfig::parse(include_str!("../../../../devices/ishtar.toml")).unwrap();
    let wlan = cfg.wlan.expect("the 13 Ultra's WLAN is described");
    assert_eq!(wlan.modules, ["qca_cld3_kiwi_v2"]);
    assert_eq!(wlan.writes, [Write { path: "/sys/kernel/cnss/fs_ready".into(), value: "1".into() }]);
    assert_eq!(wlan.firmware[0].partition, None);
    assert_eq!(wlan.firmware[1].partition.as_deref(), Some("persist"));
    assert_eq!(wlan.firmware[1].path, "kiwi_v2/wlan_mac.bin");
    assert_eq!(wlan.interface, "wlan0");
    assert_eq!(wlan.service, "android.hardware.wifi.supplicant.ISupplicant/default");
    assert_eq!(wlan.probe, "http://connect.rom.miui.com/generate_204");
    assert_eq!(cfg.update.expect("its update server is known").server, "https://update.miui.com/updates/miotaV3.php");
}

#[test]
fn nezha_brings_its_wlan_up_from_its_own_partitions() {
    let cfg = DeviceConfig::parse(include_str!("../../../../devices/nezha.toml")).unwrap();
    assert!(cfg.modules.sources.iter().any(|s| s == "system_dlkm"), "the kernel's own modules are a source");
    let wlan = cfg.wlan.expect("the 17 Ultra's WLAN is described");
    assert_eq!(wlan.modules, ["rfkill", "qca_cld3_peach_v2"]);
    assert_eq!(wlan.writes, [Write { path: "/sys/kernel/cnss/fs_ready".into(), value: "1".into() }]);
    assert!(wlan.firmware.is_empty(), "the vendor's own links reach its settings and persist's MAC address");
    assert_eq!(wlan.libraries, ["/system/lib64", "/vendor/lib64"], "the recovery's own libraries come first");
    assert_eq!(wlan.probe, "http://connect.rom.miui.com/generate_204", "as the stock system asks it");
    assert_eq!(cfg.update.expect("its update server is known").server, "https://update.miui.com/updates/miotaV3.php");
}

#[test]
fn a_wlan_is_probed_over_plain_http() {
    let base = "[device]\ncodename = \"x\"\n[wlan]\nmodules = []\ninterface = \"wlan0\"\nsupplicant = \"/s\"\nservice = \"a/b\"\n";
    assert!(DeviceConfig::parse(&format!("{base}probe = \"http://connect.rom.miui.com/generate_204\"\n")).is_ok());
    assert!(DeviceConfig::parse(&format!("{base}probe = \"https://connect.rom.miui.com/generate_204\"\n")).is_err(), "a login cannot answer in its stead");
    assert!(DeviceConfig::parse(base).is_err(), "a WLAN says how it is probed");
}

#[test]
fn firmware_is_named_from_the_loaders_folders_and_found_plainly() {
    let base = "[device]\ncodename = \"x\"\n[wlan]\nmodules = []\ninterface = \"wlan0\"\nsupplicant = \"/s\"\nservice = \"a/b\"\nprobe = \"http://p/\"\n";
    let with = |file: &str| DeviceConfig::parse(&format!("{base}firmware = [{file}]\n"));
    assert!(with("{ name = \"wlan/a.ini\", path = \"/vendor/etc/a.ini\" }").is_ok());
    assert!(with("{ name = \"wlan/mac.bin\", partition = \"persist\", path = \"kiwi/mac.bin\" }").is_ok());
    assert!(with("{ name = \"/etc/firmware/a.ini\", path = \"/vendor/etc/a.ini\" }").is_err());
    assert!(with("{ name = \"../a.ini\", path = \"/vendor/etc/a.ini\" }").is_err());
    assert!(with("{ name = \"a.ini\", path = \"vendor/etc/a.ini\" }").is_err());
    assert!(with("{ name = \"mac.bin\", partition = \"persist\", path = \"/kiwi/mac.bin\" }").is_err());
    assert!(with("{ name = \"mac.bin\", partition = \"by-name/persist\", path = \"mac.bin\" }").is_err());
}

#[test]
fn the_partition_a_stock_image_comes_from_is_named_plainly() {
    let hash = "0".repeat(64);
    let base = format!(
        "[device]\ncodename = \"x\"\n[stock]\nversion = \"OS1.0.10.0.TKHCNXM\"\npackage = \"https://a/miui_OS1.0.10.0.TKHCNXM.zip\"\nrecovery = \"{hash}\"\n"
    );
    let with = |extra: &str| DeviceConfig::parse(&format!("{base}{extra}"));
    let plain = with("").unwrap().stock.expect("the stock build is named");
    assert_eq!(plain.partition, "recovery", "a phone with a recovery partition of its own");
    assert_eq!(with("partition = \"boot\"\n").unwrap().stock.unwrap().partition, "boot", "a phone that keeps its recovery in the boot image");
    for bad in ["by-name/boot", "", "boot a"] {
        assert!(with(&format!("partition = \"{bad}\"\n")).is_err(), "{bad:?} is refused");
    }
}

#[test]
fn a_partition_mounted_as_the_phone_mounts_it_is_named_plainly() {
    let with = |name: &str| DeviceConfig::parse(&format!("[device]\ncodename = \"x\"\n[partitions]\nmounted = [\"{name}\"]\n"));
    assert_eq!(with("persist").unwrap().partitions.mounted, ["persist"]);
    assert!(with("by-name/persist").is_err());
    assert!(with("").is_err());
    assert!(DeviceConfig::parse("[device]\ncodename = \"x\"\n").unwrap().partitions.mounted.is_empty(), "a phone may need none");
}

#[test]
fn parses_alioth() {
    let cfg = DeviceConfig::parse(include_str!("../../../../devices/alioth.toml")).unwrap();
    assert_eq!(cfg.device.codename, "alioth");
    assert_eq!(cfg.device.model, "Redmi K40");
    assert_eq!((cfg.display.width, cfg.display.height, cfg.display.density), (1080, 2400, 440));
    assert_eq!(cfg.modules.sources, ["vendor"], "every module is on the vendor partition");
    assert!(cfg.modules.load.is_empty(), "its touchscreen is built into the kernel");
    assert_eq!(cfg.firmware, Some(FirmwarePartition { partition: "modem".into() }));
    let stock = cfg.stock.expect("alioth names the package its boot image is taken from");
    assert_eq!((stock.version.as_str(), stock.partition.as_str()), ("OS1.0.10.0.TKHCNXM", "boot"));
    assert_eq!(stock.recovery, "745ef81b4f68fe7f10473cbc3bd84d3ac622110723ec6d0a2faec3d5dafcdd99");
    let paths: Vec<&str> = cfg.services.iter().map(|s| s.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "/vendor/bin/qseecomd",
            "/mnt/system/system/bin/servicemanager",
            "/mnt/system/system/bin/hwservicemanager",
            "/vendor/bin/hw/android.hardware.keymaster@4.0-service-qti",
            "/mnt/system/system/bin/keystore2",
            "/vendor/bin/hw/android.hardware.gatekeeper@1.0-service-qti",
            "/vendor/bin/hw/android.hardware.boot@1.1-service",
        ]
    );
    assert_eq!(cfg.services[0].groups, ["root"], "its own init runs qseecomd as root alone");
    let keymaster = &cfg.services[3];
    assert_eq!(keymaster.ready_hardware_service.as_deref(), Some("android.hardware.keymaster@4.0::IKeymasterDevice/default"));
    assert!(
        keymaster.groups.contains(&"drmrpc".to_string()),
        "its own init runs the key service in drmrpc, which is the group /dev/qseecom is handed to"
    );
    let gatekeeper = &cfg.services[5];
    assert_eq!(gatekeeper.ready_hardware_service.as_deref(), Some("android.hardware.gatekeeper@1.0::IGatekeeper/default"));
    let boot = &cfg.services[6];
    assert_eq!(
        boot.ready_hardware_service.as_deref(),
        Some("android.hardware.boot@1.1::IBootControl/default"),
        "its vendor partition ships no 1.2 service, and the 1.1 one is what has the slots"
    );
    assert_eq!(boot.groups, ["root"], "its own init runs the boot control as root alone");
    assert!(
        cfg.services[4].ready_service.is_some(),
        "the credential service is written after the key store, which a phone without it still has"
    );
    let wlan = cfg.wlan.expect("its WLAN is described");
    assert_eq!(wlan.modules, ["qca_cld3_qca6390"]);
    assert_eq!(
        wlan.firmware,
        [
            Firmware {
                name: "wlan/qca_cld/qca6390/WCNSS_qcom_cfg.ini".into(),
                path: "/vendor/etc/wifi/qca6390/WCNSS_qcom_cfg.ini".into(),
                partition: None,
            },
            Firmware {
                name: "wlan/qca_cld/qca6390/wlan_mac.bin".into(),
                path: "/mnt/vendor/persist/wlan/wlan_mac.bin".into(),
                partition: None,
            },
        ],
        "the driver asks for both by name, and its own vendor tree answers with links a recovery cannot follow"
    );
    assert_eq!(cfg.backup.efs, ["modemst1", "modemst2", "fsg", "fsc", "persist"]);
    assert!(cfg.backup.data_skipped.contains(&"vendor/fpc".to_string()), "its fingerprint is FPC's");
}

#[test]
fn services_use_the_partitions_they_run_from() {
    let cfg = DeviceConfig::parse(include_str!("../../../../devices/ishtar.toml")).unwrap();
    let [report, sensor, listeners, ..] = &cfg.services[..] else { panic!("ishtar keeps several programs running") };
    assert!(report.uses(Path::new("/odm")));
    assert!(sensor.uses(Path::new("/odm")));
    assert!(!report.uses(Path::new("/odm_dlkm")), "a longer name is another partition");
    assert!(!sensor.uses(Path::new("/mnt/system")), "its /system/lib64 is the recovery's own");
    assert!(!report.uses(Path::new("/vendor")));
    assert!(listeners.uses(Path::new("/vendor")));
}

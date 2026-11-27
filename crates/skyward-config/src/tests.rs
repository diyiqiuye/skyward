use super::*;

#[test]
fn the_commands_sit_beside_the_recovery_service_binary() {
    let folder = |path: &str| Path::new(path).parent().map(Path::to_path_buf);
    for command in ramdisk::COMMANDS {
        assert_eq!(folder(command), folder(ramdisk::RECOVERY), "{command}");
    }
}

#[test]
fn parses_minimal() {
    let cfg = DeviceConfig::parse("[device]\ncodename = \"nezha\"\n").unwrap();
    assert_eq!(cfg.device.codename, "nezha");
    assert!(cfg.device.ab);
    assert_eq!(cfg.device.os_version_prop, "ro.build.display.id");
    assert_eq!(cfg.modules.sources, default_module_sources());
    assert!(cfg.modules.load.is_empty());
    assert_eq!((cfg.firmware, cfg.wlan, cfg.update), (None, None, None), "a phone may have none of them");
    assert_eq!(cfg.logd, None, "a recovery's own logd usually has all it needs");
}

#[test]
fn nezha_gives_its_logd_the_libraries_its_recovery_leaves_out() {
    let cfg = DeviceConfig::parse(include_str!("../../../devices/nezha.toml")).unwrap();
    let logd = cfg.logd.expect("the 17 Ultra's recovery leaves out what its logd loads");
    assert_eq!(logd.libraries, ["/mnt/system/system/lib64/libsysutils.so", "/mnt/system/system/lib64/libcap.so"]);
    let ishtar = DeviceConfig::parse(include_str!("../../../devices/ishtar.toml")).unwrap();
    assert_eq!(ishtar.logd, None, "the 13 Ultra's recovery carries both");
}

#[test]
fn ishtar_clicks_with_a_waveform_its_driver_keeps() {
    let cfg = DeviceConfig::parse(include_str!("../../../devices/ishtar.toml")).unwrap();
    let haptics = cfg.haptics.expect("the 13 Ultra has a motor");
    assert_eq!((haptics.click, haptics.resonance.as_str()), (Some(0), "/sys/class/qcom-haptics/lra_frequency_hz"));
    assert_eq!(haptics.strength, Strength { lightest: 0.5, strongest: 1.0 });
}

#[test]
fn a_click_is_kept_or_made_and_its_strengths_rise() {
    let with = |body: &str| DeviceConfig::parse(&format!("[device]\ncodename = \"x\"\n[haptics]\n{body}\n"));
    let (strength, resonance) = ("strength = { lightest = 0.25, strongest = 1.0 }", "resonance = \"/sys/class/qcom-haptics/lra_frequency_hz\"");
    assert!(with(&format!("{resonance}\n{strength}")).is_ok(), "a click made");
    assert!(with(&format!("click = 0\n{resonance}\n{strength}")).is_ok(), "a click kept");
    assert!(with(&format!("click = 0\n{strength}")).is_err(), "no motor's resonance to make the ticks at");
    assert!(with(&format!("resonance = \"sys/f\"\n{strength}")).is_err(), "a file named from nowhere");
    assert!(with(resonance).is_err(), "a click without its strengths");
    for (lightest, strongest) in [(0.0, 0.5), (0.5, 0.5), (0.5, 0.25), (0.25, 1.5)] {
        let s = format!("{resonance}\nstrength = {{ lightest = {lightest:?}, strongest = {strongest:?} }}");
        assert!(with(&s).is_err(), "{lightest} to {strongest} is refused");
    }
}

#[test]
fn a_library_for_logd_is_an_absolute_path_to_a_file() {
    let with = |library: &str| DeviceConfig::parse(&format!("[device]\ncodename = \"x\"\n[logd]\nlibraries = [\"{library}\"]\n"));
    assert!(with("/mnt/system/system/lib64/libcap.so").is_ok());
    for bad in ["lib64/libcap.so", "/", "/mnt/system/../lib64/libcap.so", ""] {
        assert!(with(bad).is_err(), "{bad} is refused");
    }
}

#[test]
fn files_handed_over_name_their_accounts_a_mode_and_absolute_paths() {
    let with = |body: &str| DeviceConfig::parse(&format!("[device]\ncodename = \"x\"\n[[owners]]\n{body}\n"));
    let ok = with("user = \"system\"\ngroup = \"system\"\nmode = 0o664\npaths = [\"/dev/xiaomi-touch\"]").unwrap();
    assert_eq!(ok.owners, [Owners { user: "system".into(), group: "system".into(), mode: 0o664, paths: vec!["/dev/xiaomi-touch".into()] }]);
    assert!(with("user = \"system\"\ngroup = \"system\"\nmode = 0o664\npaths = []").is_err(), "no file");
    assert!(with("user = \"sys tem\"\ngroup = \"system\"\nmode = 0o664\npaths = [\"/dev/a\"]").is_err());
    assert!(with("user = \"system\"\ngroup = \"system\"\nmode = 0o17777\npaths = [\"/dev/a\"]").is_err(), "not a file's mode");
    for bad in ["dev/a", "/", "/sys/../dev/a"] {
        assert!(with(&format!("user = \"system\"\ngroup = \"system\"\nmode = 0o664\npaths = [\"{bad}\"]")).is_err(), "{bad} is refused");
    }
    assert!(DeviceConfig::parse("[device]\ncodename = \"x\"\n").unwrap().owners.is_empty(), "a phone may need none");
}

#[test]
fn parses_nezha() {
    let cfg = DeviceConfig::parse(include_str!("../../../devices/nezha.toml")).unwrap();
    assert_eq!(cfg.device.codename, "nezha");
    assert_eq!(cfg.device.os_version_prop, "ro.mi.os.version.incremental");
    let soc = cfg.device.soc.expect("nezha names its SoC");
    assert_eq!(soc.hans, "第五代骁龙®8至尊版移动平台");
    assert_eq!(soc.en, "Snapdragon® 8 Elite Gen 5 Mobile Platform");
    assert_eq!(cfg.display.width, 1200);
    assert_eq!(cfg.display.diagonal, Some(6.9));
    assert_eq!(cfg.modules.load, ["synaptics_tcm2"]);
    assert_eq!(
        cfg.touch.writes,
        [Write { path: "/sys/class/touch/touch_dev/enable_touch_raw".into(), value: "0".into() }],
        "its controller reports positions itself"
    );
    assert_eq!(cfg.firmware, Some(FirmwarePartition { partition: "modem".into() }));
    assert_eq!(cfg.partitions.mounted, ["persist"], "the secure world keeps its own files there");
    let paths: Vec<&str> = cfg.services.iter().map(|s| s.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "/vendor/bin/sh",
            "/vendor/bin/qseecomd",
            "/vendor/bin/hlosminkdaemon",
            "/vendor/bin/hw/android.hardware.security.onekeymint-service-qti",
            "/vendor/bin/hw/android.hardware.gatekeeper-rust-service-qti",
            "/odm/bin/hw/android.hardware.secure_element-service-goodix",
            "/odm/bin/hw/android.hardware.weaver-service",
            "/vendor/bin/hw/android.hardware.boot-service.qti",
        ]
    );
    for s in &cfg.services {
        assert_eq!(s.libraries[..2], ["/system/lib64", "/vendor/lib64"], "{} loads the recovery's own libraries first", s.path);
        assert_eq!(s.ready_hardware_service, None, "{} is of the newer kind", s.path);
    }
    let [post_boot, listeners, opener, keymint, gatekeeper, element, weaver, boot] = &cfg.services[..] else { panic!("eight programs") };
    assert!(post_boot.oneshot, "the system's own settings for after start are made once");
    assert_eq!(post_boot.args, ["/vendor/bin/init.kernel.post_boot.sh"]);
    assert!(cfg.services[1..].iter().all(|s| !s.oneshot && s.args.is_empty()));
    assert_eq!(listeners.ready_property.as_deref(), Some("vendor.sys.listeners.registered"));
    assert_eq!((opener.ready_property.as_ref(), opener.ready_service.as_ref()), (None, None));
    assert_eq!(keymint.ready_service.as_deref(), Some("android.hardware.security.keymint.IKeyMintDevice/default"));
    assert!(keymint.registers.contains(&"android.hardware.security.secureclock.ISecureClock/default".to_string()));
    assert!(keymint.registers.contains(&"android.hardware.security.sharedsecret.ISharedSecret/default".to_string()));
    assert_eq!(gatekeeper.ready_service.as_deref(), Some("android.hardware.gatekeeper.IGatekeeper/default"));
    assert_eq!(element.ready_service.as_deref(), Some("android.hardware.secure_element.ISecureElement/eSE2"));
    assert_eq!(element.description.as_deref(), Some("/odm/etc/secure_element-service-goodixsp.xml"));
    assert_eq!(weaver.ready_service.as_deref(), Some("android.hardware.weaver.IWeaver/default"));
    assert_eq!(boot.ready_service.as_deref(), Some("android.hardware.boot.IBootControl/default"));
    let haptics = cfg.haptics.expect("the 17 Ultra has a motor");
    assert_eq!((haptics.click, haptics.resonance.as_str()), (None, "/sys/class/qcom-haptics/lra_frequency_hz"), "its click is made");
    assert_eq!(haptics.strength, Strength { lightest: 0.25, strongest: 0.75 }, "past three quarters the motor rattles");
    assert_eq!(cfg.backup.efs, ["modemst1", "modemst2", "fsg", "fsc", "persist"]);
    assert!(cfg.backup.data_skipped.contains(&"vendor/goodix".to_string()));
    assert!(cfg.wipe.logs.contains(&"/data/vendor/bsplog".to_string()));
}

#[test]
fn the_secure_element_of_each_build_runs_on_that_build_alone() {
    let cfg = DeviceConfig::parse(include_str!("../../../devices/nezha.toml")).unwrap();
    let runs = |props: &[(&str, &str)]| -> Vec<String> {
        let props: HashMap<String, String> = props.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        cfg.services.iter().filter(|s| s.runs_on(&props)).map(|s| s.path.clone()).collect()
    };
    let leica = runs(&[("ro.special_edition", "true")]);
    assert_eq!(leica.len(), 8);
    let plain = runs(&[]);
    assert_eq!(plain.len(), 6);
    assert!(!plain.iter().any(|p| p.starts_with("/odm/")), "the other builds' weaver cannot run in a recovery");
    assert_eq!(runs(&[("ro.special_edition", "false")]), plain);
}

#[test]
fn a_program_runs_only_where_the_property_has_its_value() {
    let base = "[device]\ncodename = \"x\"\n[[service]]\npath = \"/odm/bin/a\"\n";
    let only = DeviceConfig::parse(&format!("{base}when = {{ property = \"ro.sku\", value = \"p\" }}\n")).unwrap();
    let always = DeviceConfig::parse(base).unwrap();
    let props = |v: Option<&str>| -> HashMap<String, String> { v.map(|v| ("ro.sku".to_string(), v.to_string())).into_iter().collect() };
    for (value, runs) in [(Some("p"), true), (Some("q"), false), (None, false)] {
        assert_eq!(only.services[0].runs_on(&props(value)), runs, "with ro.sku {value:?}");
        assert!(always.services[0].runs_on(&props(value)));
    }
}

#[test]
fn a_program_run_once_waits_for_nothing_and_registers_nothing() {
    let base = "[device]\ncodename = \"x\"\n[[service]]\npath = \"/vendor/bin/sh\"\nargs = [\"/vendor/bin/a.sh\"]\noneshot = true\n";
    let once = DeviceConfig::parse(base).unwrap();
    assert!(once.services[0].oneshot);
    assert_eq!(once.services[0].args, ["/vendor/bin/a.sh"]);
    assert!(DeviceConfig::parse(&format!("{base}ready_property = \"vendor.a\"\n")).is_err());
    assert!(DeviceConfig::parse(&format!("{base}ready_service = \"a.IA/default\"\n")).is_err());
    assert!(DeviceConfig::parse(&format!("{base}registers = [\"a.IB/default\"]\n")).is_err());
    assert!(DeviceConfig::parse(&format!("{base}description = \"/vendor/etc/a.xml\"\n")).is_err());
    let kept = DeviceConfig::parse("[device]\ncodename = \"x\"\n[[service]]\npath = \"/vendor/bin/a\"\n").unwrap();
    assert!(!kept.services[0].oneshot && kept.services[0].args.is_empty(), "a program is kept running unless it says otherwise");
}

#[test]
fn a_program_runs_as_its_user_in_its_groups_keeping_named_capabilities() {
    let base = "[device]\ncodename = \"x\"\n[[service]]\npath = \"/odm/bin/a\"\n";
    let root = DeviceConfig::parse(base).unwrap();
    let s = &root.services[0];
    assert_eq!((s.user.as_ref(), s.groups.len(), s.capabilities.len()), (None, 0, 0), "root, as init runs one that names no user");
    let named = DeviceConfig::parse(&format!("{base}user = \"system\"\ngroups = [\"system\", \"drmrpc\"]\ncapabilities = [\"WAKE_ALARM\"]\n")).unwrap();
    let s = &named.services[0];
    assert_eq!((s.user.as_deref(), &s.groups[..], &s.capabilities[..]), (Some("system"), &["system".to_string(), "drmrpc".to_string()][..], &["WAKE_ALARM".to_string()][..]));
    assert!(DeviceConfig::parse(&format!("{base}user = \"system\"\n")).is_err(), "a user without its group");
    assert!(DeviceConfig::parse(&format!("{base}user = \"system\"\ngroups = [\"system\"]\ncapabilities = [\"WAKE\"]\n")).is_err(), "no such capability");
    assert!(DeviceConfig::parse(&format!("{base}user = \"sys tem\"\ngroups = [\"system\"]\n")).is_err());
    assert_eq!(
        [capability("CHOWN"), capability("SETPCAP"), capability("WAKE_ALARM"), capability("BLOCK_SUSPEND"), capability("CHECKPOINT_RESTORE"), capability("CAP_CHOWN")],
        [Some(0), Some(8), Some(35), Some(36), Some(40), None],
        "numbered as the kernel numbers them, named as init names them"
    );
}

#[test]
fn each_phones_programs_run_as_their_init_scripts_run_them() {
    let nezha = DeviceConfig::parse(include_str!("../../../devices/nezha.toml")).unwrap();
    let ishtar = DeviceConfig::parse(include_str!("../../../devices/ishtar.toml")).unwrap();
    for s in nezha.services.iter().chain(&ishtar.services) {
        assert!(s.user.is_some(), "{} names the user its init script does", s.path);
    }
    let find = |cfg: &DeviceConfig, path: &str| cfg.services.iter().find(|s| s.path == path).cloned().expect("listed");
    let element = find(&nezha, "/odm/bin/hw/android.hardware.secure_element-service-goodix");
    assert_eq!((element.user.as_deref(), &element.groups[..]), (Some("system"), &["system".to_string()][..]), "its logs in the data partition are the system's");
    let weaver = find(&nezha, "/odm/bin/hw/android.hardware.weaver-service");
    assert_eq!(weaver.capabilities, ["WAKE_ALARM"]);
    let listeners = find(&nezha, "/vendor/bin/qseecomd");
    assert_eq!((listeners.user.as_deref(), &listeners.groups[..]), (Some("root"), &["root".to_string(), "drmrpc".to_string()][..]));
    let opener = find(&nezha, "/vendor/bin/hlosminkdaemon");
    assert_eq!(opener.capabilities, ["WAKE_ALARM", "BLOCK_SUSPEND"]);
    let touch = find(&ishtar, "/odm/bin/touch_report");
    assert_eq!((touch.user.as_deref(), &touch.capabilities[..]), (Some("system"), &["SYS_NICE".to_string()][..]));
}

#[test]
fn each_phone_names_the_stock_build_its_recovery_is_made_on() {
    for (text, version) in [
        (include_str!("../../../devices/nezha.toml"), "OS4.0.4.0.XPACNXM"),
        (include_str!("../../../devices/ishtar.toml"), "OS3.0.308.0.WMACNXM"),
        (include_str!("../../../devices/pudding.toml"), "OS4.0.4.0.XPCCNXM"),
        (include_str!("../../../devices/pandora.toml"), "OS4.0.4.0.XBLCNXM"),
        (include_str!("../../../devices/popsicle.toml"), "OS4.0.4.0.XPBCNXM"),
        (include_str!("../../../devices/myron.toml"), "OS4.0.4.0.XPMCNXM"),
        (include_str!("../../../devices/songyuan.toml"), "OS4.0.4.0.XGNCNXM"),
        (include_str!("../../../devices/athens.toml"), "OS4.0.4.0.XPICNXM"),
        (include_str!("../../../devices/fuxi.toml"), "OS3.0.309.0.WMCCNXM"),
        (include_str!("../../../devices/nuwa.toml"), "OS3.0.312.0.WMBCNXM"),
        (include_str!("../../../devices/houji.toml"), "OS3.0.307.0.WNCCNXM"),
        (include_str!("../../../devices/shennong.toml"), "OS3.0.309.0.WNBCNXM"),
        (include_str!("../../../devices/aurora.toml"), "OS3.0.308.0.WNACNXM"),
        (include_str!("../../../devices/dada.toml"), "OS3.0.307.0.WOCCNXM"),
        (include_str!("../../../devices/haotian.toml"), "OS3.0.309.0.WOBCNXM"),
        (include_str!("../../../devices/xuanyuan.toml"), "OS3.0.314.0.WOACNXM"),
        (include_str!("../../../devices/byron.toml"), "OS3.0.310.0.WAFCNXM"),
        (include_str!("../../../devices/zorn.toml"), "OS3.0.309.0.WOKCNXM"),
        (include_str!("../../../devices/miro.toml"), "OS3.0.305.0.WOMCNXM"),
        (include_str!("../../../devices/annibale.toml"), "OS3.0.307.0.WPKCNXM"),
        (include_str!("../../../devices/warsaw.toml"), "OS3.0.309.0.WHPCNXM"),
        (include_str!("../../../devices/alioth.toml"), "OS1.0.10.0.TKHCNXM"),
    ] {
        let stock = DeviceConfig::parse(text).unwrap().stock.expect("a stock build to build on");
        assert_eq!(stock.version, version);
        assert!(stock.package.ends_with(".zip") && stock.package.contains(&format!("/{version}/")), "{}", stock.package);
    }
}

#[test]
fn a_stock_build_is_a_full_package_over_https_and_a_sha_256() {
    let ok = "[device]\ncodename = \"x\"\n[stock]\nversion = \"OS1.0.1.0.XXXCNXM\"\npackage = \"https://d.example/OS1.0.1.0.XXXCNXM/x-ota_full-OS1.0.1.0.XXXCNXM-user-16.0-0123456789.zip\"\n";
    let hash = "faa425b10231a0cfe2d74ff34f344f032ee2d891ea5e9420bf4e67cd86d622a7";
    assert!(DeviceConfig::parse(&format!("{ok}recovery = \"{hash}\"\n")).is_ok());
    assert!(DeviceConfig::parse(&format!("{ok}recovery = \"{}\"\n", hash.to_uppercase())).is_err(), "in lowercase");
    assert!(DeviceConfig::parse(&format!("{ok}recovery = \"{}\"\n", &hash[1..])).is_err(), "too short");
    let plain = ok.replace("https://", "http://");
    assert!(DeviceConfig::parse(&format!("{plain}recovery = \"{hash}\"\n")).is_err(), "over plain HTTP");
    let other = ok.replace("version = \"OS1.0.1.0", "version = \"OS1.0.2.0");
    assert!(DeviceConfig::parse(&format!("{other}recovery = \"{hash}\"\n")).is_err(), "the package of another build");
    assert_eq!(DeviceConfig::parse("[device]\ncodename = \"x\"\n").unwrap().stock, None);
}

#[test]
fn the_firmware_partition_is_named_plainly() {
    let base = "[device]\ncodename = \"x\"\n[firmware]\n";
    assert!(DeviceConfig::parse(&format!("{base}partition = \"modem\"\n")).is_ok());
    assert!(DeviceConfig::parse(&format!("{base}partition = \"by-name/modem\"\n")).is_err());
    assert!(DeviceConfig::parse(&format!("{base}partition = \"\"\n")).is_err());
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
fn parses_ishtar() {
    let cfg = DeviceConfig::parse(include_str!("../../../devices/ishtar.toml")).unwrap();
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
    assert_eq!(codename(include_str!("../../../devices/ishtar.toml")).unwrap(), "ishtar");
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
    let cfg = DeviceConfig::parse(include_str!("../../../devices/ishtar.toml")).unwrap();
    assert!(cfg.wipe.logs.contains(&"/data/vendor/diag".to_string()));
    assert!(DeviceConfig::parse("[device]\ncodename = \"x\"\n").unwrap().wipe.logs.is_empty(), "a phone may have none");
    for bad in ["/data", "/data/", "/metadata/logs", "data/log", "/data/../system"] {
        let text = format!("[device]\ncodename = \"x\"\n[wipe]\nlogs = [\"{bad}\"]\n");
        assert!(DeviceConfig::parse(&text).is_err(), "{bad} is refused");
    }
}

#[test]
fn ishtar_brings_its_wlan_up_from_its_own_partitions() {
    let cfg = DeviceConfig::parse(include_str!("../../../devices/ishtar.toml")).unwrap();
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
    let cfg = DeviceConfig::parse(include_str!("../../../devices/nezha.toml")).unwrap();
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
fn parses_alioth() {
    let cfg = DeviceConfig::parse(include_str!("../../../devices/alioth.toml")).unwrap();
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
        ]
    );
    assert_eq!(cfg.services[3].ready_hardware_service.as_deref(), Some("android.hardware.keymaster@4.0::IKeymasterDevice/default"));
    let wlan = cfg.wlan.expect("its WLAN is described");
    assert_eq!(wlan.modules, ["qca_cld3_qca6390"]);
    assert!(wlan.firmware.is_empty(), "its vendor partition's own links are left to say where they are");
    assert_eq!(cfg.backup.efs, ["modemst1", "modemst2", "fsg", "fsc", "persist"]);
    assert!(cfg.backup.data_skipped.contains(&"vendor/fpc".to_string()), "its fingerprint is FPC's");
}

#[test]
fn services_use_the_partitions_they_run_from() {        let cfg = DeviceConfig::parse(include_str!("../../../devices/ishtar.toml")).unwrap();
    let [report, sensor, listeners, ..] = &cfg.services[..] else { panic!("ishtar keeps several programs running") };
    assert!(report.uses(Path::new("/odm")));
    assert!(sensor.uses(Path::new("/odm")));
    assert!(!report.uses(Path::new("/odm_dlkm")), "a longer name is another partition");
    assert!(!sensor.uses(Path::new("/mnt/system")), "its /system/lib64 is the recovery's own");
    assert!(!report.uses(Path::new("/vendor")));
    assert!(listeners.uses(Path::new("/vendor")));
}

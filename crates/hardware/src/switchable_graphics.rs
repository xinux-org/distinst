use dbus::blocking::Connection;
use std::{fs, io, path::Path, time::Duration};

const POWER: &str = "etc/modprobe.d/system76-power.conf";
const PRIME_DISCRETE: &str = "etc/prime-discrete";

static MODPROBE_HYBRID: &[u8] = br#"# Automatically generated by distinst
options nvidia NVreg_DynamicPowerManagement=0x02
blacklist i2c_nvidia_gpu
alias i2c_nvidia_gpu off
"#;

static MODPROBE_INTEGRATED: &[u8] = br#"# Automatically generated by distinst
blacklist i2c_nvidia_gpu
blacklist nouveau
blacklist nvidia
blacklist nvidia-drm
blacklist nvidia-modeset
alias i2c_nvidia_gpu off
alias nouveau off
alias nvidia off
alias nvidia-drm off
alias nvidia-modeset off
"#;

/// Configure graphics mode if switchable graphics is supported.
pub fn configure_graphics(mount_dir: &Path) -> io::Result<bool> {
    if !has_switchable_graphics() {
        return Ok(false);
    }

    let _ = fs::create_dir_all(mount_dir.join("etc/modprobe.d/"));

    match default_graphics().as_str() {
        "integrated" => {
            info!("disabling external NVIDIA graphics by default");
            fs::write(mount_dir.join(POWER), MODPROBE_INTEGRATED)?;

            info!("configuring gpu-manager for integrated graphics mode");
            fs::write(mount_dir.join(PRIME_DISCRETE), "off")?;
        }
        "hybrid" => {
            info!("settings module options for hybrid graphics mode");
            fs::write(mount_dir.join(POWER), MODPROBE_HYBRID)?;

            info!("configuring gpu-manager for hybrid graphics mode");
            fs::write(mount_dir.join(PRIME_DISCRETE), "on-demand")?;
        }
        _ => (),
    }

    Ok(true)
}

fn default_graphics() -> String {
    if let Ok(conn) = Connection::new_system() {
        let proxy = conn.with_proxy(
            "com.system76.PowerDaemon",
            "/com/system76/PowerDaemon",
            Duration::from_millis(1000),
        );
        let (gfx,): (String,) = proxy
            .method_call("com.system76.PowerDaemon", "GetDefaultGraphics", ())
            .unwrap_or_else(|_| ("nvidia".to_string(),));

        gfx
    } else {
        "nvidia".to_string()
    }
}

fn has_switchable_graphics() -> bool {
    if let Ok(conn) = Connection::new_system() {
        let proxy = conn.with_proxy(
            "com.system76.PowerDaemon",
            "/com/system76/PowerDaemon",
            Duration::from_millis(1000),
        );
        let (switchable,): (bool,) =
            proxy.method_call("com.system76.PowerDaemon", "GetSwitchable", ()).unwrap_or_default();

        // Limit configuring graphics to System76 models
        switchable && (system_vendor() == "System76")
    } else {
        false
    }
}

fn system_vendor() -> String {
    fs::read_to_string("/sys/class/dmi/id/sys_vendor").unwrap_or_default().trim().into()
}

/// Read battery status from Linux sysfs.
/// Returns true if the system is on battery power (AC not connected and battery present).
#[cfg(target_os = "linux")]
pub fn on_battery() -> bool {
    use std::fs;

    let power_supply_dir = std::path::Path::new("/sys/class/power_supply");
    let entries = match fs::read_dir(power_supply_dir) {
        Ok(e) => e,
        Err(_) => return false,
    };

    let mut has_ac = false;
    let mut has_battery = false;
    let mut ac_online = false;
    let mut battery_discharging = false;

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let path = power_supply_dir.join(&name);

        // Read type file to classify
        let typ = fs::read_to_string(path.join("type")).unwrap_or_default();
        let typ = typ.trim();

        if typ == "Mains" {
            has_ac = true;
            if let Ok(online) = fs::read_to_string(path.join("online")) {
                if online.trim() == "1" {
                    ac_online = true;
                }
            }
        } else if typ == "Battery" {
            has_battery = true;
            if let Ok(status) = fs::read_to_string(path.join("status")) {
                let status = status.trim();
                if status == "Discharging" || status == "Not charging" {
                    battery_discharging = true;
                }
            }
        }
    }

    if has_ac {
        // If AC is present and online, not on battery.
        if ac_online {
            return false;
        }
        // AC present but not online -> on battery.
        return true;
    }

    // No AC supply; if there's a battery and it's discharging, we're on battery.
    if has_battery && battery_discharging {
        return true;
    }

    false
}

#[cfg(not(target_os = "linux"))]
pub fn on_battery() -> bool {
    false
}
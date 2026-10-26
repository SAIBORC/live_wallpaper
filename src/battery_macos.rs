/// Read battery status on macOS using `pmset -g batt`.
/// Returns true if the system is on battery power (AC not connected).
#[cfg(target_os = "macos")]
pub fn on_battery() -> bool {
    let output = std::process::Command::new("pmset")
        .args(["-g", "batt"])
        .output();

    match output {
        Ok(out) => {
            let text = String::from_utf8_lossy(&out.stdout);
            // pmset output looks like:
            //   Now drawing from 'Battery Power'
            //   -InternalBattery-0 100%; charging; 0:00 remaining
            // or
            //   Now drawing from 'AC Power'
            text.contains("Battery Power") && !text.contains("AC Power")
        }
        Err(_) => false,
    }
}

#[cfg(not(target_os = "macos"))]
pub fn on_battery() -> bool {
    false
}
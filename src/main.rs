
use std::collections::HashMap;
use std::process::Command;
use std::str;
use std::sync::mpsc::{channel, Receiver};
use std::thread;
use std::time::Duration;

use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "App Volume Controller",
        options,
        Box::new(|_cc| Box::<MyApp>::default()),
    )
}

struct MyApp {
    apps: HashMap<u32, HashMap<String, String>>, // pid -> {prop -> val}
    per_app_volumes: HashMap<u32, f32>,          // pid -> volume in percent
    vol: f32,                                    // main vol
    last_update: std::time::Instant,
    update_rx: Receiver<HashMap<u32, HashMap<String, String>>>,
}

impl Default for MyApp {
    fn default() -> Self {
        let vol = match get_system_volume() {
            Some(v) => v,
            None => 0.0,
        };

        let (tx, rx) = channel();

        // Spawn a background thread that polls `pactl` every second and sends results.
        thread::spawn(move || loop {
            let apps = parse_sink_inputs();
            // best-effort send; if receiver was dropped, stop the thread
            if tx.send(apps).is_err() {
                break;
            }
            thread::sleep(Duration::from_secs(1));
        });

        Self {
            apps: HashMap::new(),
            per_app_volumes: HashMap::new(),
            vol,
            last_update: std::time::Instant::now(),
            update_rx: rx,
        }
    }
}


impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Drain any background updates and apply the latest state.
        for apps in self.update_rx.try_iter() {
            self.apps = apps;
            // Update per-app volumes from latest apps snapshot
            self.per_app_volumes.clear();
            for (pid, data) in &self.apps {
                if let Some(vol_str) = data.get("Volume") {
                    if let Some(first_percent) = vol_str.split('/').nth(1) {
                        if let Some(percent_str) = first_percent.trim().strip_suffix('%') {
                            if let Ok(percent) = percent_str.trim().parse::<f32>() {
                                self.per_app_volumes.insert(*pid, percent);
                            }
                        }
                    }
                }
            }

            // Also refresh system volume when we have new data
            if let Some(sys_v) = get_system_volume() {
                self.vol = sys_v;
            }
            self.last_update = std::time::Instant::now();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("🎧 System Volume Controller");

        
            ui.group(|ui| {
                ui.label("🔊 System Volume:");
                let slider = ui.add(egui::Slider::new(&mut self.vol, 0.0..=100.0).text("%"));
                if slider.changed() {
                    set_main_volume(self.vol);
                }
            });

            ui.separator();

            // App Sliders
            ui.label("🎶 Application Volumes:");
            // Iterate in a stable, sorted order to avoid widgets jumping positions
            let mut pids: Vec<u32> = self.apps.keys().cloned().collect();
            pids.sort();
            for pid in pids {
                let props = &self.apps[&pid];
                let unknown = "Unknown".to_string();
                // Prefer a more descriptive title when present (tab/page title or media name)
                let primary = props
                    .get("media.name")
                    .or_else(|| props.get("application.name"))
                    .or_else(|| props.get("application.process.binary"))
                    .unwrap_or(&unknown);
                ui.group(|ui| {
                    ui.label(format!("{} (pid: {})", primary, pid));
                    // Show secondary info when available and different from primary
                    if let Some(app_name) = props.get("application.name") {
                        if app_name != primary {
                            ui.label(format!("App: {}", app_name));
                        }
                    }
                    if let Some(media_title) = props.get("media.name") {
                        if media_title != primary {
                            ui.label(format!("Title: {}", media_title));
                        }
                    }

                    if let Some(vol) = self.per_app_volumes.get_mut(&pid) {
                        let slider = ui.add(egui::Slider::new(vol, 0.0..=100.0).text("%"));
                        if slider.changed() {
                            set_app_volume(pid, *vol);
                        }
                    } else {
                        ui.label("No volume data.");
                    }
                });
                ui.separator();
            }
        });
    }
}

fn set_main_volume(vol: f32) {
    let _ = Command::new("wpctl")
        .args(["set-volume", "@DEFAULT_AUDIO_SINK@", &format!("{:.2}%", vol)])
        .output();
}

fn set_app_volume(index: u32, vol: f32) {
    //println!("the u32 {} and the vol {} ",index,vol);

   let id_str=index.to_string();

   let _ = Command::new("pactl")
        .args(&["set-sink-input-volume", &id_str, &format!("{}%",vol)])
        .output();
    }




fn get_system_volume() -> Option<f32> {
    let output = Command::new("wpctl")
        .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
        .output()
        .expect("failed to get volume");

    if output.status.success() {
        let stdout = str::from_utf8(&output.stdout).unwrap_or("invalid UTF-8");
        let parts: Vec<&str> = stdout.trim().split_whitespace().collect();

        if let Some(volume_str) = parts.last() {
            if let Ok(volume) = volume_str.parse::<f32>() {
           
                return Some(volume * 100.0); // as percentage
            }
        }

        eprintln!("Failed to parse volume from output: {}", stdout);
    } else {
        eprintln!(
            "Error: {}",
            str::from_utf8(&output.stderr).unwrap_or("unknown error")
        );
    }

    None
}

fn parse_sink_inputs() -> HashMap<u32, HashMap<String, String>> {
    let output = Command::new("pactl")
        .args(&["list", "sink-inputs"])
        .output()
        .expect("Failed to execute pactl");

    let stdout = str::from_utf8(&output.stdout).expect("Invalid UTF-8 output");

    let mut result: HashMap<u32, HashMap<String, String>> = HashMap::new();
    let mut current_id: Option<u32> = None;

    for line in stdout.lines() {
        let trimmed = line.trim_start();

        if let Some(id_str) = trimmed.strip_prefix("Sink Input #") {
            if let Ok(id) = id_str.trim().parse::<u32>() {
                current_id = Some(id);
                result.insert(id, HashMap::new());
            }
        }

        if let Some((key, value)) = trimmed.split_once(" = ") {
            if let Some(id) = current_id {
                result
                    .get_mut(&id)
                    .unwrap()
                    .insert(key.to_string(), value.trim_matches('"').to_string());
            }
        }

        if trimmed.starts_with("Volume:") {
            if let Some(id) = current_id {
                result
                    .get_mut(&id)
                    .unwrap()
                    .insert("Volume".to_string(), trimmed["Volume:".len()..].trim().to_string());
            }
        }
    }

    result
}

impl MyApp {
    fn refresh_apps(&mut self) {
        self.apps = parse_sink_inputs();
        self.per_app_volumes.clear();
        for (pid, data) in &self.apps {
            if let Some(vol_str) = data.get("Volume") {
                if let Some(first_percent) = vol_str.split('/').nth(1) {
                    if let Some(percent_str) = first_percent.trim().strip_suffix('%') {
                        if let Ok(percent) = percent_str.trim().parse::<f32>() {
                            self.per_app_volumes.insert(*pid, percent);
                        }
                    }
                }
            }
        }

        // Also refresh system vol
        let out = Command::new("wpctl")
            .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
            .output()
            .unwrap();
        if out.status.success() {
            let stdout = str::from_utf8(&out.stdout).unwrap_or("");
            let trimmed = stdout.trim();
            if let Some(val) = trimmed.split_whitespace().nth(1) {
                if let Ok(val_f) = val.parse::<f32>() {
                    self.vol = val_f * 100.0;
                }
            }
        }
    }
}

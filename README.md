# App Volume Controller (Rust + eframe)

## Overview

This Rust application provides a graphical user interface (GUI) for managing the **system volume** and **per-application audio volumes** on Linux systems that use PipeWire and PulseAudio compatibility (`wpctl` and `pactl`).

The app is built using [eframe](https://docs.rs/eframe/latest/eframe/) (based on `egui`) for a lightweight and responsive native GUI.

**Note:** I developed this tool for my personal use to conveniently control audio volumes directly from a simple GUI.

## Features

- **System Volume Control:**  
  Shows a slider to adjust the global audio output volume using `wpctl set-volume`.

- **Per-Application Volume Control:**  
  Lists all current audio sink inputs (audio streams) with their application names and allows adjusting each stream's volume individually using `pactl set-sink-input-volume`.

- **Automatic Refresh:**  
  Updates system and per-app volume information every second to reflect real-time changes.

## Requirements

- Linux with **PipeWire** and **PulseAudio compatibility**  
- Commands: `wpctl` and `pactl` available in system PATH  
- Rust toolchain with `eframe` crate  

## How It Works

- The app periodically runs `wpctl get-volume @DEFAULT_AUDIO_SINK@` to fetch the current system volume.
- It runs `pactl list sink-inputs` to enumerate all audio streams, parsing application names and volumes.
- The GUI displays a main volume slider and sliders for each active audio stream.
- Adjusting sliders updates volumes in real time by running `wpctl set-volume` or `pactl set-sink-input-volume`.

## Usage

Build and run with Cargo:

``` bash
cargo run
```


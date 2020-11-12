use std::fs::{File, Metadata};
use std::io::Write;
use std::path::Path;
use std::{env, fs, process};

fn main() {
    let all_arguments: Vec<String> = env::args().collect();
    if all_arguments.len() < 2 {
        println!("You must provide absolute path to Godot project.");
        process::exit(1);
    }
    let godot_project = all_arguments[1].trim_end_matches('/').to_string();

    if !Path::new(&godot_project).is_dir() {
        println!("{} isn't proper directory.", all_arguments[1]);
        process::exit(1);
    }
    if !Path::new(&(godot_project.to_string() + "/2d")).exists()
        || !Path::new(&(godot_project.to_string() + "/3d")).exists()
        || !Path::new(&(godot_project.to_string() + "/networking")).exists()
    {
        println!(
            "{} isn't proper Godot demo project repository.",
            all_arguments[1]
        );
        process::exit(1);
    }

    let mut good_directories: Vec<String> = Vec::new();
    let mut folders_to_check: Vec<String> = Vec::new();
    let mut next_folder: String;
    let mut current_folder: String;

    folders_to_check.push(godot_project.clone());

    while !folders_to_check.is_empty() {
        current_folder = folders_to_check.pop().unwrap();

        let read_dir = match fs::read_dir(&current_folder) {
            Ok(t) => t,
            _ => continue,
        };
        for entry in read_dir {
            let entry_data = match entry {
                Ok(t) => t,
                Err(_) => continue, //Permissions denied
            };
            let metadata: Metadata = match entry_data.metadata() {
                Ok(t) => t,
                Err(_) => continue, //Permissions denied
            };
            if metadata.is_dir() {
                let folder_name: String = match entry_data.file_name().into_string() {
                    Ok(t) => t,
                    Err(_) => continue, // Permission Denied
                };
                if folder_name.starts_with('.') {
                    continue;
                }
                if folder_name == "mono" || folder_name == "plugins" {
                    continue;
                }

                next_folder = format!("{}/{}", current_folder, folder_name);
                folders_to_check.push(next_folder);
            } else if metadata.is_file() {
                let file_name: String = entry_data.file_name().to_string_lossy().to_string();
                if file_name == "project.godot" {
                    good_directories.push(current_folder.clone());
                }
            }
        }
    }

    let common_text: &str = "name: 🐧 Linux Builds
on: [push, pull_request]


jobs:
  test-projects:
    runs-on: \"ubuntu-20.04\"
    name: Test demo projects

    steps:
      - uses: actions/checkout@v2

      - name: Change sources.list
        run: |
          sudo rm -f /etc/apt/sources.list.d/*
          sudo cp -f sources.list /etc/apt/sources.list
          sudo apt-get update
";
    let default_stage = "
      - name: Download Godot
        run: |
          sudo apt-get install -y build-essential pkg-config libx11-dev libxcursor-dev \
            libxinerama-dev libgl1-mesa-dev libglu-dev libasound2-dev libpulse-dev libudev-dev libxi-dev libxrandr-dev yasm \
            wget2 unzip -y
          wget2 https://downloads.tuxfamily.org/godotengine/3.2.3/Godot_v3.2.3-stable_x11.64.zip
          unzip Godot_v3.2.3-stable_x11.64.zip
          mv Godot_v3.2.3-stable_x11.64 godot
";
    let sanitizer_stage = "
      - name: Compile Godot
        run: |
          sudo apt-get install -y build-essential pkg-config libx11-dev libxcursor-dev \
            libxinerama-dev libgl1-mesa-dev libglu-dev libasound2-dev libpulse-dev libudev-dev libxi-dev libxrandr-dev yasm \
            git
          git clone https://github.com/godotengine/godot.git
          cd godot
          scons tools=yes target=debug use_asan=yes use_ubsan=yes -j2
          cd ..
          mv bin/godot.x11.tools.64s godot
";

    let text_to_change : &str = "
      - name: PROJECT_NAME
        run: |
          echo \"\" > sanitizers_log.txt
          DRI_PRIME=0 timeout 10s xvfb-run ./godot --audio-driver Dummy -e    --path PROJECT_NAME 2>&1 | tee -a sanitizers_log.txt || true
          DRI_PRIME=0             xvfb-run ./godot --audio-driver Dummy -e -q --path PROJECT_NAME 2>&1 | tee -a sanitizers_log.txt || true
          DRI_PRIME=0 timeout 10s xvfb-run ./godot --audio-driver Dummy       --path PROJECT_NAME 2>&1 | tee -a sanitizers_log.txt || true
          ./check_ci_log.py sanitizers_log.txt
    ";

    let excluded_items = [
        "audio/mic_record",        // Leaking Memory even with default Godot binary
        "loading/background_load", // Contains some images and loads in more than 10 seconds
        "misc/2.5d",               // Leaking Memory even with default Godot binary
        "3d/material_testers",     // Contains some images and loads in more than 10 seconds
        "3d/ik", // Contains some images and loads in more than 10 seconds or just fails without any reason
        "3d/platformer", // Contains some images and loads in more than 10 seconds
        "2d/physics_platformer", // Strange crash, needs to be checked
        "2d/navigation", // Strange crash, needs to be checked
    ];

    let mut file_godot_default =
        File::create("ci_data_default.txt").expect("Failed to create file");
    let mut file_godot_sanitizers =
        File::create("ci_data_sanitizers.txt").expect("Failed to create file");

    write!(file_godot_default, "{}", common_text).expect("Failed save data to file");
    write!(file_godot_sanitizers, "{}", common_text).expect("Failed save data to file");

    write!(file_godot_default, "{}", default_stage).expect("Failed save data to file");
    write!(file_godot_sanitizers, "{}", sanitizer_stage).expect("Failed save data to file");

    for folder in good_directories {
        let project_name = folder[godot_project.len() + 1..].to_string();
        let mut new_text = text_to_change.replace("PROJECT_NAME", project_name.as_str());
        if excluded_items.iter().any(|e| *e == project_name) {
            new_text = new_text.replace('\n', "\n#");
        }

        write!(file_godot_default, "{}", new_text).expect("Failed save data to file");
        write!(file_godot_sanitizers, "{}", new_text).expect("Failed save data to file");

        //print!("{}", new_text);
    }
}

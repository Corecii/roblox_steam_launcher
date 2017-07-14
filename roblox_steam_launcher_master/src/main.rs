extern crate roblox_steam_launcher_shared;
extern crate hyper;
extern crate notify;

use roblox_steam_launcher_shared::*;
use std::path::PathBuf;
use hyper::client::Client;
use std::io::Read;
use notify::{RecommendedWatcher, Watcher, RecursiveMode};
use std::sync::mpsc::{channel, RecvTimeoutError};
use std::ffi::OsStr;
use std::time::{Duration, Instant};

#[derive(Debug)]
enum ApplyError {
    NoExecutable,
    NoProgramDir,
    CannotRename(std::io::Error),
    CannotCopy(std::io::Error),
    CannotRemove(std::io::Error),
}

#[derive(Debug)]
enum ApplyResult {
    Applied,
    AlreadyApplied,
    Error(ApplyError)
}

fn check_if_newest_version(current_version: String) -> hyper::error::Result<bool> {
    let client = Client::new();
    let mut res = client.get("http://setup.roblox.com/version").send()?;
    let mut msg = String::new();
    res.read_to_string(&mut msg)?;
    Ok(msg == current_version)
}

fn apply_launcher(version_path: &PathBuf) -> ApplyResult {
    let mut new_application_path = version_path.clone();
    new_application_path.push("RobloxPlayerLauncher_original.exe");
    if new_application_path.is_file() {
        return ApplyResult::AlreadyApplied;  // We've already applied the launcher here!
    }
    let mut new_substitute_path = version_path.clone();
    new_substitute_path.push("RobloxPlayerLauncher.exe");
    if !new_substitute_path.is_file() {
        return ApplyResult::Error(ApplyError::NoExecutable);  // We can't apply to a non-existent executable.
    }
    let mut substitute_base_path = match get_program_directory(&mut std::env::args()) {
        Some(path) => path,
        None => return ApplyResult::Error(ApplyError::NoProgramDir),
    };
    substitute_base_path.push("roblox_steam_launcher_substitute.exe");
    if let Err(err) = std::fs::rename(&new_substitute_path, new_application_path) {
        return ApplyResult::Error(ApplyError::CannotRename(err));
    }
    if let Err(err) = std::fs::copy(&substitute_base_path, new_substitute_path) {
        return ApplyResult::Error(ApplyError::CannotCopy(err));
    }
    return ApplyResult::Applied;
}

fn unapply_launcher(version_path: &PathBuf) -> ApplyResult {
    let mut new_application_path = version_path.clone();
    new_application_path.push("RobloxPlayerLauncher_original.exe");
    if !new_application_path.is_file() {
        return ApplyResult::AlreadyApplied;
    }
    let mut new_substitute_path = version_path.clone();
    new_substitute_path.push("RobloxPlayerLauncher.exe");
    if new_substitute_path.exists() {
        if let Err(err) = std::fs::remove_file(&new_substitute_path) {
            return ApplyResult::Error(ApplyError::CannotRemove(err));
        }
    }
    if let Err(err) = std::fs::rename(&new_application_path, new_substitute_path) {
        return ApplyResult::Error(ApplyError::CannotRename(err));
    }
    return ApplyResult::Applied;
}

fn watch_for_new_exe(version_path: &PathBuf, config_debug: bool) {
    let (tx, rx) = channel();
    let mut watcher: RecommendedWatcher = Watcher::new(tx, Duration::from_secs(0)).expect("Could not create file watcher");
    watcher.watch(version_path, RecursiveMode::Recursive).expect("Could not watch directory");
    if config_debug {
        println!("Watching for changes...");
    }
    let start = Instant::now();
    let timeout_duration = Duration::from_secs(120);
    loop {
        match rx.recv_timeout(Duration::from_secs(10)) {
            Ok(event) => {
                if config_debug {
                    println!("{:?}", event);
                }
                if let notify::DebouncedEvent::Create(new_path) = event {
                    if new_path.file_name() == Some(OsStr::new("RobloxPlayerLauncher.exe")) {
                        //fix up this new exe
                        if config_debug {
                            println!("Applying launcher...");
                        }
                        let mut version_path = new_path.clone();
                        if config_debug {
                            println!("New launcher at: {:?}", version_path);
                        }
                        version_path.pop();
                        let result = apply_launcher(&version_path);
                        if config_debug {
                            println!("Result: {:?}", result);
                        }
                        break;
                    }
                }
            },
            Err(RecvTimeoutError::Timeout) => {
                if start.elapsed() >= timeout_duration && config_debug {
                    println!("Timeout reached. Exiting.");
                }
            },
            Err(RecvTimeoutError::Disconnected) => {
                if config_debug {
                    println!("watch error: disconnected");
                }
                break;
            },
        }
    }
    if config_debug {
        println!("Done watching for changes.");
    }
}

enum UIErr {
    Simple(&'static str),
    String(&'static str, String),
    ConfigRead(&'static str, ConfigReadError),
    ConfigWrite(&'static str, ConfigWriteError),
    Apply(&'static str, ApplyError),
}

fn errorable_main() -> Result<bool, UIErr> {
    let program_directory = match get_program_directory(&mut std::env::args()) {
        Some(path) => path,
        None => return Err(UIErr::Simple("Cannot get program directory.")),
    };
    let mut config_path = program_directory.clone();
    config_path.push(get_config_file_name());  // down to config file
    let mut config = match Config::from_path(&config_path) {
        Ok(config) => config,
        Err(err) => return Err(UIErr::ConfigRead("Could not read config file", err)),
    };
    let config_debug = config.debug;
    if config_debug {
        println!("Read config file");
    }
    let config_arguments = config.arguments;
    config.arguments = vec![];
    match config.write_to_path(&config_path) {
        Ok(_) => (),
        Err(err) => return Err(UIErr::ConfigWrite("Could not write config file", err)),
    };
    if config_debug {
        println!("Cleared arguments and wrote config file");
    }
    let current_version_directory = match get_newest_roblox_player_directory_path(&program_directory) {
        Some(v) => v,
        None => return Err(UIErr::Simple("Error getting Roblox newest directory")),
    };
    if config_debug {
        println!("Got current version directory: {:?}", current_version_directory);
    }
    let current_version = match current_version_directory.file_name() {
        Some(v) => v,
        None => return Err(UIErr::Simple("Error getting newest version name from directory"))
    }.to_string_lossy().into_owned();
    if config_debug {
        println!("Got current version: {:?}", current_version);
    }
    match check_if_newest_version(current_version) {
        Ok(false) => {
            if config_debug {
                println!("Roblox is not the newest version. Updating...");
            }
            // Remove existing modifications
            if let ApplyResult::Error(err) = unapply_launcher(&current_version_directory) {
                return Err(UIErr::Apply("Error unapplying launcher for update", err));
            }
            if config_debug {
                println!("Unapplied existing launcher.");
            }
            // Run roblox once to update, and wait for it to close.
            // In the future, we should only run it once, but replace the new exe as it is created.
            let mut exe_path = current_version_directory.clone();
            exe_path.push("RobloxPlayerLauncher.exe");
            match launch_game(&exe_path, &config_arguments) {
                Ok(_) => {
                    if config_debug {
                        println!("Began update process.");
                    }
                    watch_for_new_exe(&program_directory, config_debug);
                },
                Err(err) => return Err(UIErr::String("Could not run the Roblox updater", format!("{:?}", err))),
            }
        },
        Ok(true) | Err(_) => {
            let newest_version_directory = current_version_directory;
            match apply_launcher(&newest_version_directory) {
                ApplyResult::AlreadyApplied | ApplyResult::Applied => (),
                ApplyResult::Error(err) => return Err(UIErr::Apply("Error applying launcher", err)),
            }
            if config_arguments.len() == 0 {
                if config_debug {
                    println!("Arguments length was 0, exiting.");
                }
                return Ok(config_debug);  // We weren't supposed to run the roblox launcher anyway
            }
            let mut game_directory = newest_version_directory;
            game_directory.push("RobloxPlayerLauncher_original.exe");
            if let Err(err) = launch_game(&game_directory, &config_arguments) {
                return Err(UIErr::String("Could not run Roblox", format!("{:?}", err)));
            }
        },
    };
    Ok(config_debug)
}

fn main() {
    match errorable_main() {
        Ok(is_debug) => {
            if is_debug {
                println!("Press enter to exit.");
                std::io::stdin().read_line(&mut String::new()).expect("Failed to read line");
                return;
            }
        },
        Err(err) => {
            println!("Error launching Roblox!");
            match err {
                UIErr::Simple(reason) => println!("{}", reason),
                UIErr::String(reason, _) => println!("{}", reason),
                UIErr::ConfigRead(reason, _) => println!("{}", reason),
                UIErr::ConfigWrite(reason, _) => println!("{}", reason),
                UIErr::Apply(reason, _) => println!("{}", reason),
            }
            println!();
            println!("Press enter to exit.");
            std::io::stdin().read_line(&mut String::new()).expect("Failed to read line");
            return;
        }
    }
}

extern crate roblox_steam_launcher_shared;

use roblox_steam_launcher_shared::*;
use std::path::PathBuf;

#[derive(Copy, Clone)]
enum ApplyResult {
    Applied,
    AlreadyApplied,
    CannotApply,
    Error
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
        return ApplyResult::CannotApply;  // We can't apply to a non-existent executable.
    }
    let mut substitute_base_path = match get_program_directory(&mut std::env::args()) {
        Some(path) => path,
        None => panic!("Cannot get program directory."),
    };
    substitute_base_path.push("roblox_steam_launcher_substitute.exe");
    if std::fs::rename(&new_substitute_path, new_application_path).is_err() {
        return ApplyResult::Error;
    }
    if std::fs::copy(&substitute_base_path, new_substitute_path).is_err() {
        return ApplyResult::Error;
    }
    return ApplyResult::Applied;
}

fn main() {
    let program_directory = match get_program_directory(&mut std::env::args()) {
        Some(path) => path,
        None => panic!("Cannot get program directory."),
    };
    let mut config_path = program_directory.clone();
    config_path.push(get_config_file_name());  // down to config file
    let mut config = match Config::from_path(&config_path) {
        Ok(config) => config,
        Err(err) => panic!("{}", err),
    };
    let newest_version_directory = get_newest_roblox_player_directory_path(&program_directory).expect("Error getting newest directory.");
    match apply_launcher(&newest_version_directory) {
        ApplyResult::AlreadyApplied | ApplyResult::Applied => (),
        ApplyResult::CannotApply => panic!("Could not apply launcher: RobloxPlayerLauncher.exe does not exist."),
        ApplyResult::Error => panic!("Error applying launcher."),
    }
    if config.arguments.len() == 0 {
        return;  // We weren't supposed to run the roblox launcher anyway
    }
    let mut game_directory = newest_version_directory;
    game_directory.push("RobloxPlayerLauncher_original.exe");
    launch_game(&game_directory, &config.arguments).expect("Error launching ROBLOX");
    config.arguments = vec![];
    match config.write_to_path(&config_path) {
        Ok(_) => (),
        Err(err) => panic!("{}", err),
    };

}

#[macro_use]
extern crate serde_derive;

extern crate serde;
extern crate serde_json;

use std::fs::OpenOptions;
use std::env;
use std::io::prelude::*;
use std::process::Command;
use std::path::PathBuf;
use std::error::Error;
use std::fmt;


#[derive(Clone,Debug,PartialEq,Serialize, Deserialize)]
pub struct Config {
    pub steam_gameid: String,
    pub debug: bool,
    pub arguments: Vec<String>,
}

#[derive(Clone)]
pub enum ConfigReadError {
    Malformed(String),
    NotReadable(String),
    NotOpenable(String),
}

impl std::fmt::Display for ConfigReadError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &ConfigReadError::Malformed(ref err) => write!(formatter, "Malformed config file: {}", err),
            &ConfigReadError::NotReadable(ref err) => write!(formatter, "Cannot read config file: {}", err),
            &ConfigReadError::NotOpenable(ref err) => write!(formatter, "Cannot open config file for reading: {}", err),
        }
    }
}

#[derive(Clone)]
pub enum ConfigWriteError {
    NotSerializable(String),
    NotWriteable(String),
    NotOpenable(String),
}

impl std::fmt::Display for ConfigWriteError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &ConfigWriteError::NotSerializable(ref err) => write!(formatter, "Cannot serialize config struct: {}", err),
            &ConfigWriteError::NotWriteable(ref err) => write!(formatter, "Cannot write config file: {}", err),
            &ConfigWriteError::NotOpenable(ref err) => write!(formatter, "Cannot open config file for writing: {}", err),
        }
    }
}

impl Config {
    pub fn new() -> Config {
        Config {
            steam_gameid: String::new(),
            debug: false,
            arguments: vec![],
        }
    }
    pub fn from_path(path: &PathBuf) -> Result<Config, ConfigReadError> {
        return match OpenOptions::new().read(true).write(false).open(path) {
            Ok(mut read_file) => {
                let mut contents = String::new();
                match read_file.read_to_string(&mut contents) {
                    Ok(_) => {
                        match serde_json::from_str(&contents) {
                            Ok(config) => Ok(config),
                            Err(err) => Err(ConfigReadError::Malformed(String::from(err.description())))
                        }
                    },
                    Err(err) => Err(ConfigReadError::NotReadable(String::from(err.description())))
                }
            },
            Err(err) => Err(ConfigReadError::NotOpenable(String::from(err.description())))
        }
    }
    pub fn write_to_path(&self, path: &PathBuf) -> Result<(), ConfigWriteError> {
        match OpenOptions::new().read(false).write(true).create(true).truncate(true).open(path) {
            Ok(mut write_file) => {
                let config_as_str = match serde_json::to_string(self) {
                    Ok(as_str) => as_str,
                    Err(err) => return Err(ConfigWriteError::NotSerializable(String::from(err.description())))
                };
                match write_file.write_all(config_as_str.as_bytes()) {
                    Ok(_) => return Ok(()),
                    Err(err) => return Err(ConfigWriteError::NotWriteable(String::from(err.description()))),
                };
            },
            Err(err) => return Err(ConfigWriteError::NotOpenable(String::from(err.description()))),
        }
    }
}

pub fn get_program_directory(args: &mut env::Args) -> Option<PathBuf> {
    match args.nth(0) {
        Some(arg) => {
            let mut base = PathBuf::from(arg);
            base.pop();
            Some(base)
        },
        None => None,
    }
}

pub fn get_intended_arguments(args: &mut env::Args) -> Vec<String> {
    args.collect::<Vec<_>>().split_off(1)
}

pub fn get_config_file_name() -> &'static str {
    "roblox_steam_launcher_config.json"
}

pub fn launch_steam(game_id: String) -> std::io::Result<std::process::Child> {
    Command::new("explorer.exe")
        .arg(format!("steam://rungameid/{}", game_id))
        .spawn()
}

pub fn launch_game(game_path: &PathBuf, args: &Vec<String>) -> std::io::Result<std::process::Child> {
    let mut working_path = game_path.clone();
    working_path.pop();
    Command::new(game_path)
        .args(args)
        .current_dir(working_path)
        .spawn()
}

pub fn get_newest_roblox_player_directory_path(versions_path: &PathBuf) -> Option<PathBuf> {
    let mut newest_dir = None;
    for current_dir_opt in versions_path.read_dir().expect("Failed to iterate over directory") {
        if let Ok(current_dir) = current_dir_opt {
            if current_dir.path().is_dir() {
                let mut player_path = current_dir.path().clone();
                player_path.push("RobloxPlayerLauncher.exe");
                if player_path.exists() {
                    if let Ok(current_metadata) = current_dir.metadata() {
                        if let Ok(current_created) = current_metadata.created() {
                            newest_dir = match newest_dir {
                                None => Some((current_dir, current_created)),
                                Some((newest_dir, newest_created)) => {
                                    if current_created > newest_created {
                                        Some((current_dir, current_created))
                                    } else {
                                        None
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    match newest_dir {
        None => None,
        Some((newest_dir, newest_created)) => Some(newest_dir.path()),
    }
}

pub fn get_newest_roblox_player_executable_path(versions_path: &PathBuf) -> Option<PathBuf> {
    let folder = get_newest_roblox_player_directory_path(versions_path);
    match folder {
        None => None,
        Some(path) => {
            let mut new_path = path.clone();
            new_path.push("RobloxPlayerLauncher_original.exe");
            if new_path.is_file() {
                Some(new_path)
            } else {
                None
            }
        }
    }
}

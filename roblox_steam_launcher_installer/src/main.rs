#[macro_use]
extern crate lazy_static;
extern crate roblox_steam_launcher_shared;
extern crate steam_vdf;
extern crate winreg;
extern crate regex;

use roblox_steam_launcher_shared::*;

use std::process::Command;
use std::path::{Path, PathBuf};
use std::io;
use std::io::{Read, Write};
use std::fs::OpenOptions;
use std::ffi::OsString;

use winreg::RegKey;
use winreg::enums::*;

use regex::Regex;

fn convert_to_hex_string(input: String) -> String {
    let mut out = String::new();
    for c in input.as_bytes() {
        out += &format!("{:X}", c);
    }
    out
}

#[derive(Debug)]
enum GameIdError {
    CommandError(String),
    Utf8Error(String)
}

fn get_gameid_raw(target: String, name: String) -> Result<String, GameIdError> {
    let hex_input = convert_to_hex_string(target + &name);
    match Command::new("roblox_steam_launcher_gameid.exe").arg(hex_input).output() {
        Ok(out) => match String::from_utf8(out.stdout) {
            Ok(s) => Ok(String::from(s.trim())),
            Err(err) => Err(GameIdError::Utf8Error(format!("{:?}", err))),
        },
        Err(err) => Err(GameIdError::CommandError(format!("{:?}", err))),
    }
}

fn get_gameid(target: &PathBuf, name: String) -> Result<String, GameIdError> {
    get_gameid_raw(String::from("\"") + &target.as_os_str().to_string_lossy() + "\"", name)
}

struct SteamUser {
    user_id: String,
    user_name: String,
    userdata_dir: PathBuf,
}

fn get_steam_users(steam_userdata: &PathBuf) -> io::Result<Vec<SteamUser>> {
    let mut users = vec![];
    for userdata_dir in steam_userdata.read_dir()? {
        let mut path = userdata_dir?.path();
        let name_path = path.clone();
        let user_id = match name_path.file_name() {
            Some(name) => name.to_string_lossy().into_owned(),
            None => continue,  // This should never happen.
        };
        path.push("config");
        let userdata_dir = path.clone();
        path.push("localconfig.vdf");
        let mut file = OpenOptions::new().read(true).write(false).open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        lazy_static! {
            static ref REGEX_GET_NAME: Regex = Regex::new("\"PersonaName\"\\s*\"(.+?)\"[\r\n]+").unwrap();
            static ref REGEX_REPLACE_QUOTE: Regex = Regex::new("\"").unwrap();
        }
        let captures = REGEX_GET_NAME.captures(&contents).unwrap();
        let user_name_raw = match captures.get(1) {
            Some(mat) => mat.as_str(),
            None => "Unknown"
        };
        let user_name = REGEX_REPLACE_QUOTE.replace_all(&user_name_raw, "\"").into_owned();
        users.push(SteamUser {
            user_id: user_id,
            user_name: String::from(user_name),
            userdata_dir: userdata_dir,
        });
    }
    Ok(users)
}

fn read_shortcuts(user_userdata_dir: &PathBuf) -> io::Result<Option<steam_vdf::ValveData>> {
    let mut shortcuts_path = user_userdata_dir.clone();
    shortcuts_path.push("shortcuts.vdf");
    if !shortcuts_path.exists() {
        return Ok(Some(steam_vdf::ValveData::List(OsString::from("shortcuts"), vec![])));
    }

    let mut file = OpenOptions::new().read(true).write(false).open(shortcuts_path)?;
    steam_vdf::read_data(&mut file)
}

fn write_shortcuts(user_userdata_dir: &PathBuf, shortcuts: &steam_vdf::ValveData) -> io::Result<()> {
    let mut shortcuts_path = user_userdata_dir.clone();
    shortcuts_path.push("shortcuts.vdf");
    let mut file = OpenOptions::new().read(false).write(true).truncate(true).create(true).open(shortcuts_path)?;
    steam_vdf::write_data(&mut file, shortcuts)?;
    file.write_all(&[0x08])?;
    Ok(())
}

fn remove_roblox_launcher(shortcuts_list: &mut steam_vdf::ValveData, roblox_launcher_path: &PathBuf) -> usize {
    let absolute_path = match roblox_launcher_path.canonicalize() {
        Ok(new_path) => new_path,
        Err(err) => panic!("Could not get absolute path: {}", err),
    };
    let mut absolute_path_value = OsString::from("\"");
    absolute_path_value.push(absolute_path.as_os_str());
    absolute_path_value.push("\"");
    let mut count = 0;
    match shortcuts_list {
        &mut steam_vdf::ValveData::List(ref name, ref mut shortcuts_vec) => {
            shortcuts_vec.retain(|ref mut shortcut| {
                if let &mut &steam_vdf::ValveData::List(ref shortcut_name, ref shortcut_vec) = shortcut {
                    for property in shortcut_vec {
                        if let &steam_vdf::ValveData::String(ref property_name, ref property_value) = property {
                            if property_name == "exe" && absolute_path_value == *property_value {
                                count += 1;
                                return false;
                            }
                        }
                    }
                }
                true
            });
            for (index, shortcut) in shortcuts_vec.iter_mut().enumerate() {
                if let &mut steam_vdf::ValveData::List(ref mut shortcut_name, ref mut shortcut_vec) = shortcut {
                    shortcut_name.clear();
                    shortcut_name.push(&index.to_string());
                }
            }
        },
        _ => panic!("shortcuts should be a ValveData::List!"),
    }
    count
}

fn add_roblox_launcher(shortcuts_list: &mut steam_vdf::ValveData, roblox_launcher_path: &PathBuf, roblox_launcher_name: String) -> (PathBuf, String) {
    let absolute_path = match roblox_launcher_path.canonicalize() {
        Ok(new_path) => new_path,
        Err(err) => panic!("Could not get absolute path: {}", err),
    };
    let mut absolute_start = absolute_path.clone();
    absolute_start.pop();
    let new_position: u64;
    let mut add_entry_to: &mut Vec<steam_vdf::ValveData>;

    match shortcuts_list {
        &mut steam_vdf::ValveData::List(ref name, ref mut shortcuts_vec) => {
            let mut new_position = 0;
            let len = shortcuts_vec.len();
            if len > 0 {
                let last_shortcut = &mut shortcuts_vec[len - 1];
                // Get last item. It should be a list, but it could be anything.
                match last_shortcut {
                    // It's a list.
                    &mut steam_vdf::ValveData::List(ref name, ref mut contents) => {
                        let position: u64 = match name.to_string_lossy().parse() {
                            Ok(i) => i,
                            Err(_) => panic!("shortcut item name should be a positive integer!"),
                        };
                        new_position = position + 1;
                    },
                    // It's not a list. Whoops.
                    _ => panic!("shortcut item should be a ValveData::List!"),
                }
            }
            let new_entry_name = new_position.to_string();
            let mut new_entry_data = vec![];
            new_entry_data.push(steam_vdf::ValveData::String(OsString::from("AppName"), OsString::from(&roblox_launcher_name)));
            let mut absolute_path_value = OsString::from("\"");
            absolute_path_value.push(absolute_path.as_os_str());
            absolute_path_value.push("\"");
            new_entry_data.push(steam_vdf::ValveData::String(OsString::from("exe"), absolute_path_value));
            let mut absolute_start_value = OsString::from("\"");
            absolute_start_value.push(absolute_start.as_os_str());
            absolute_start_value.push("\"");
            new_entry_data.push(steam_vdf::ValveData::String(OsString::from("StartDir"),  absolute_start_value));
            new_entry_data.push(steam_vdf::ValveData::String(OsString::from("icon"), OsString::from("")));
            new_entry_data.push(steam_vdf::ValveData::String(OsString::from("ShortcutPath"), OsString::from("")));
            new_entry_data.push(steam_vdf::ValveData::String(OsString::from("LaunchOptions"), OsString::from("")));
            new_entry_data.push(steam_vdf::ValveData::Bytes4(OsString::from("IsHidden"), [0x01, 0x00, 0x00, 0x00]));
            new_entry_data.push(steam_vdf::ValveData::Bytes4(OsString::from("AllowDesktopConfig"), [0x01, 0x00, 0x00, 0x00]));
            new_entry_data.push(steam_vdf::ValveData::Bytes4(OsString::from("OpenVR"), [0x00, 0x00, 0x00, 0x00]));
            new_entry_data.push(steam_vdf::ValveData::Bytes4(OsString::from("LastPlayTime"), [0x00, 0x00, 0x00, 0x00]));
            new_entry_data.push(steam_vdf::ValveData::List(  OsString::from("tags"), vec![]));
            let new_entry = steam_vdf::ValveData::List(OsString::from(new_entry_name), new_entry_data);
            shortcuts_vec.push(new_entry);
        },
        _ => panic!("shortcuts should be a ValveData::List!"),
    }
    (absolute_path, roblox_launcher_name)
}

fn get_steam_directory() -> PathBuf {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let steam_key = hkcu.open_subkey_with_flags("Software\\Valve\\Steam", KEY_READ).expect("Cannot open steam registry key!");
    let steam_location: String = steam_key.get_value("SteamPath").expect("Cannot read SteamPath key!");
    return Path::new(&steam_location).to_path_buf();
}

/// Returns (VersionsDirectory, CurrentPlayerLauncherPath)
fn get_roblox_directories() -> (PathBuf, PathBuf) {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let roblox_key = hkcu.open_subkey_with_flags("Software\\RobloxReg", KEY_READ).expect("Cannot open roblox registry key!");
    let roblox_location: String = roblox_key.get_value("").expect("Cannot read (Default) key!");
    let current_path = Path::new(&roblox_location).to_path_buf();
    let mut base_path = current_path.clone();
    base_path.pop();
    base_path.pop();
    return (base_path, current_path);
}

fn main() {
    let (roblox_versions_path, roblox_current_path) = get_roblox_directories();
    let steam_path = get_steam_directory();

    println!("Roblox Steam Launcher installer and uninstaller");

    println!();

    println!("Roblox directory: {}", roblox_versions_path.as_os_str().to_string_lossy());
    println!("Steam directory: {}", steam_path.as_os_str().to_string_lossy());

    println!();
    println!("What would you like to do?\n[0] Install\n[1] Uninstall");

    let action_choice_num: u8;
    loop {
        println!("Please enter your choice:");
        let mut action_choice_str = String::new();
        std::io::stdin().read_line(&mut action_choice_str).expect("Failed to read line");
        action_choice_str = String::from(action_choice_str.trim());

        action_choice_num = match action_choice_str.parse() {
            Ok(num) => {
                if num < 2 {
                    num
                } else {
                    println!("Please enter a number on the list.");
                    continue;
                }
            },
            Err(_) => {
                println!("Please enter a positive number.");
                continue;
            }
        };
        break;
    };

    match action_choice_num {
        0 => install(roblox_versions_path, roblox_current_path, steam_path),
        1 => uninstall(roblox_versions_path, roblox_current_path, steam_path),
        _ => unreachable!(),
    }
}

fn uninstall(roblox_versions_path: PathBuf, roblox_current_path: PathBuf, steam_path: PathBuf) {
    let mut config_path = roblox_versions_path.clone();
    config_path.push(get_config_file_name());

    let mut config = match Config::from_path(&config_path) {
        Ok(config) => config,
        Err(err) => panic!("{}", err),
    };

    let mut master_launcher_path = roblox_versions_path.clone();
    master_launcher_path.push("roblox_steam_launcher_master.exe");

    println!("Removing launcher from Steam...");

    let mut users_dir = steam_path.clone();
    users_dir.push("userdata");
    match get_steam_users(&users_dir) {
        Ok(users) => {
            println!("Remove Roblox Launcher non-Steam game for which Steam user?");
            for (index, user) in users.iter().enumerate() {
                println!("[{}] {}", index, user.user_name);
            }
            println!("[{}] Skip", users.len());

            println!();

            let steam_user_choice_num: usize;
            loop {
                println!("Please enter your choice:");
                let mut steam_user_choice_str = String::new();
                std::io::stdin().read_line(&mut steam_user_choice_str).expect("Failed to read line");
                steam_user_choice_str = String::from(steam_user_choice_str.trim());

                steam_user_choice_num = match steam_user_choice_str.parse() {
                    Ok(num) => {
                        if num <= users.len() {
                            num
                        } else {
                            println!("Please enter a number on the list.");
                            continue;
                        }
                    },
                    Err(_) => {
                        println!("Please enter a positive number.");
                        continue;
                    }
                };
                break;
            };

            if steam_user_choice_num == users.len() {
                println!("Skipping step...");
            } else {
                println!("Will remove non-Steam game for {}", users[steam_user_choice_num].user_name);

                let mut shortcuts = read_shortcuts(&users[steam_user_choice_num].userdata_dir).unwrap().unwrap();
                let removed_shortcuts = remove_roblox_launcher(&mut shortcuts, &master_launcher_path);
                write_shortcuts(&users[steam_user_choice_num].userdata_dir, &shortcuts).unwrap();
                println!("Removed {} non-Steam games from Steam.", removed_shortcuts);
                println!();
                println!("Please restart Steam.");
            }
        },
        Err(err) => {
            println!("Could not get Steam users! Skipping step...")
        }
    };

    let files = ["roblox_steam_launcher_master.exe", "roblox_steam_launcher_substitute.exe", &get_config_file_name()];

    println!("Deleting files from Roblox\\Versions directory...");

    for file_name in files.iter() {
        let mut base_path = roblox_versions_path.clone();
        base_path.push(file_name);
        if let Err(err) = std::fs::remove_file(&base_path) {
            println!("Error deleting file: {}", err);
            println!("Press enter to exit.");
            std::io::stdin().read_line(&mut String::new()).expect("Failed to read line");
            return;
        }
    }

    println!("Files deleted.");

    println!("Reverting changes to current Roblox player version...");

    let normal_exe_path = roblox_current_path;
    let mut original_exe_path = normal_exe_path.clone();
    original_exe_path.pop();
    original_exe_path.push("RobloxPlayerLauncher_original.exe");

    if original_exe_path.exists() {

        if let Err(err) = std::fs::remove_file(&normal_exe_path) {
            println!("Error deleting file: {}", err);
            println!("Press enter to exit.");
            std::io::stdin().read_line(&mut String::new()).expect("Failed to read line");
            return;
        }

        if let Err(err) = std::fs::rename(&original_exe_path, normal_exe_path) {
            println!("Error renaming file: {}", err);
            println!("Press enter to exit.");
            std::io::stdin().read_line(&mut String::new()).expect("Failed to read line");
            return;
        }

        println!("Changes reverted.");
    } else {
        println!("Changes appear to already be reverted.");
    }

    println!();
    println!("Done. Roblox Steam Launcher should be uninstalled.");

    println!("Press enter to exit.");
    std::io::stdin().read_line(&mut String::new()).expect("Failed to read line");
}

fn install(roblox_versions_path: PathBuf, roblox_current_path: PathBuf, steam_path: PathBuf) {
    let mut config_path = roblox_versions_path.clone();
    config_path.push(get_config_file_name());

    let mut users_dir = steam_path.clone();
    users_dir.push("userdata");
    let users = get_steam_users(&users_dir).expect("Could not get Steam users.");

    println!("Add Roblox Launcher non-Steam game for which Steam user?");
    for (index, user) in users.iter().enumerate() {
        println!("[{}] {}", index, user.user_name);
    }

    let skip_steam_enabled = config_path.exists();
    if skip_steam_enabled {
        println!("[{}] Skip", users.len());
    }

    println!();

    let steam_user_choice_num: usize;
    loop {
        println!("Please enter your choice:");
        let mut steam_user_choice_str = String::new();
        std::io::stdin().read_line(&mut steam_user_choice_str).expect("Failed to read line");
        steam_user_choice_str = String::from(steam_user_choice_str.trim());

        steam_user_choice_num = match steam_user_choice_str.parse() {
            Ok(num) => {
                if num < users.len() {
                    num
                } else if skip_steam_enabled {
                    num
                } else {
                    println!("Please enter a number on the list.");
                    continue;
                }
            },
            Err(_) => {
                println!("Please enter a positive number.");
                continue;
            }
        };
        break;
    };

    if steam_user_choice_num == users.len() {
        println!("Will NOT add non-Steam game to any account.");
    } else {
        println!("Will add non-Steam game for {}", users[steam_user_choice_num].user_name);
    }

    println!("Installing files in Roblox\\Versions directory...");

    let mut master_launcher_path = roblox_versions_path.clone();
    master_launcher_path.push("roblox_steam_launcher_master.exe");

    let program_dir = match get_program_directory(&mut std::env::args()) {
        Some(path) => path,
        None => panic!("Cannot get program directory."),
    };

    if master_launcher_path.exists() {
        println!("Roblox Steam Launcher appears to be installed already. Skipping step.");
    } else {


        let copy_files = ["roblox_steam_launcher_master.exe", "roblox_steam_launcher_substitute.exe"];

        println!("Copying files to Roblox\\Versions directory...");

        for file_name in copy_files.iter() {
            let mut installer_path = program_dir.clone();
            installer_path.push(file_name);
            let mut base_path = roblox_versions_path.clone();
            base_path.push(file_name);
            if std::fs::copy(&installer_path, &base_path).is_err() {
                println!("Error copying file!");
                println!("Press enter to exit.");
                std::io::stdin().read_line(&mut String::new()).expect("Failed to read line");
                return;
            }
        }

        println!("Copied files.");
    }

    let mut steam_gameid = String::new();

    if steam_user_choice_num < users.len() {
        println!("Adding launcher to Steam as a non-Steam game...");
        let mut shortcuts = read_shortcuts(&users[steam_user_choice_num].userdata_dir).unwrap().unwrap();
        let (abs_launcher_path, launcher_steam_name) = add_roblox_launcher(&mut shortcuts, &master_launcher_path, String::from("Roblox"));
        steam_gameid = get_gameid(&abs_launcher_path, launcher_steam_name).unwrap();
        write_shortcuts(&users[steam_user_choice_num].userdata_dir, &shortcuts).unwrap();
        println!("Added launcher to Steam. Game id: {}", steam_gameid);
    }

    if !config_path.exists() {
        println!("Generating configuration file...");

        let mut config = Config::new();
        config.steam_gameid = steam_gameid;

        match config.write_to_path(&config_path) {
            Ok(_) => (),
            Err(err) => {
                println!("Error creating config file!");
                println!("Press enter to exit.");
                std::io::stdin().read_line(&mut String::new()).expect("Failed to read line");
                return;
            }
        };

        println!("Generated and saved config file.");
    }

    println!("Running roblox_steam_launcher_master.");

    match Command::new(master_launcher_path).output() {
        Ok(out) => println!("Ran roblox_steam_launcher_master successfully."),
        Err(err) => println!("Error running roblox_steam_launcher_master: {}", err),
    };

    println!("Done.");

    println!();

    if steam_user_choice_num < users.len() {
        println!("Please restart Steam.");
    }

    println!();

    println!("Press enter to exit.");
    std::io::stdin().read_line(&mut String::new()).expect("Failed to read line");
    return;
}

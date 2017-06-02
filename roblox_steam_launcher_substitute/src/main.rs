extern crate roblox_steam_launcher_shared;
extern crate hyper;

use roblox_steam_launcher_shared::*;
use hyper::client::Client;
use std::io::Read;

fn check_if_newest_version(current_version: String) -> hyper::error::Result<bool> {
    let client = Client::new();
    let mut res = client.get("http://setup.roblox.com/version").send()?;
    let mut msg = String::new();
    res.read_to_string(&mut msg)?;
    Ok(msg == current_version)
}

fn main() {
    let program_dir = match get_program_directory(&mut std::env::args()) {
        Some(path) => path,
        None => panic!("Cannot get program directory."),
    };
    let current_version = program_dir.file_name().expect("Could not get file name!").to_string_lossy().into_owned();
    let mut config_path = program_dir.clone();
    config_path.pop();  // up from a `version-###` folder to `versions`
    config_path.push(get_config_file_name());  // down to config file
    let mut config = match Config::from_path(&config_path) {
        Ok(config) => config,
        Err(err) => panic!("{}", err),
    };
    config.arguments = get_intended_arguments(&mut std::env::args());
    match config.write_to_path(&config_path) {
        Ok(_) => (),
        Err(err) => panic!("{}", err),
    };
    // if Roblox needs to update then it needs to update outside if Steam.
    // if Roblox tries to update inside of Steam, it crashes!
    match check_if_newest_version(current_version) {
        Ok(false) => {
            // Run roblox once to update, and wait for it to close. Then we run it through Steam.
            let mut exe_path = program_dir.clone();
            exe_path.push("RobloxPlayerLauncher_original.exe");
            let mut child_process = launch_game(&exe_path, &vec![]).expect("Could not launch Roblox");
            if let Err(err) = child_process.wait() {
                println!("Error waiting for Roblox: {:?}", err);
            }
        },
        Ok(true) | Err(_) => (),  // if we are on the newest version OR if we couldn't check, launch roblox.
    };
    // arguments are written to config file. Launch the master exe so we can start the game.
    match launch_steam(config.steam_gameid) {
        Ok(_) => (),
        Err(err) => panic!("Failed to launch steam: {}", err)
    };
}

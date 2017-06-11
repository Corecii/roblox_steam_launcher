extern crate roblox_steam_launcher_shared;

use roblox_steam_launcher_shared::*;

fn main() {
    let program_dir = match get_program_directory(&mut std::env::args()) {
        Some(path) => path,
        None => panic!("Cannot get program directory."),
    };
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
    // arguments are written to config file. Launch the master exe so we can start the game.
    match launch_steam(config.steam_gameid) {
        Ok(_) => (),
        Err(err) => panic!("Failed to launch steam: {}", err)
    };
}

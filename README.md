# Roblox Steam Launcher

This allows one to have the Steam overlay and Steam Input in Roblox. It functions
by replacing `RobloxPlayerLauncher.exe` with a custom launcher that launches the
real `RobloxPlayerLauncher` through Steam.

## Installation and Uninstallation

1. Grab the [Latest Release](https://github.com/Corecii/roblox_steam_launcher/releases)
2. Unzip
3. Run `roblox_steam_launcher_installer.exe`.

## Building

Building requires a Windows computer with an installation of [Rust](https://www.rust-lang.org/en-US/install.html), [Python 2.7](https://www.python.org/downloads/release/python-2713/),
and [PyInstaller](http://www.pyinstaller.org/).  
The two build scripts (`build.bat` and `build_release.bat`) will run `cargo build` and `pyinstaller -F -y` in the proper places and copy the executables
to `build` or `build_release`.

## Issues

* Python is used for calculating the game id of a game because I could not figure out how
to set up the crc32 calculation properly in Rust, and I did not want to spend time
writing my own. It was fastest to bundle a python executable.
* When updating Roblox, it cannot be running underneath Steam or it will crash. `roblox_steam_launcher` will
check if Roblox needs to be updated, and run `RobloxPlayerLauncher` with no arguments if
it does. This is untested, but it *should* update Roblox, then open up the Games page.
After updating, `roblox_steam_launcher` runs `RobloxPlayerLauncher` through Steam.
* The Roblox website will suggest that one downloads Roblox, as if it is not already installed.
Roblox still runs fine. This may be an issue with the launcher taking too long, but it has
not been investigated.
* Errors are not handled well, and any executable included will likely error out and
exit without prompting the user if there is a problem.

## Screenshots

![Steam Notification](http://i.imgur.com/E1eb1eh.png "Steam Notification")
![Steam Overlay](http://i.imgur.com/JURyXFe.png "Steam Overlay")
![Steam Controller Support](http://i.imgur.com/dOHbnDm.png "Steam Controller Support")
![Steam Keyboard Support](http://i.imgur.com/J9IDFMe.jpg "Steam Keyboard Support")

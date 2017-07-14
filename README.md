# Roblox Steam Launcher

This allows one to have the Steam overlay and Steam Input in Roblox. It functions
by replacing `RobloxPlayerLauncher.exe` with a custom launcher that launches the
real `RobloxPlayerLauncher` through Steam.

## [Newest Release](https://github.com/Corecii/roblox_steam_launcher/releases)

## Installation and Uninstallation

1. Grab the [Latest Release](https://github.com/Corecii/roblox_steam_launcher/releases)
2. Unzip
3. Run `roblox_steam_launcher_installer.exe`.
4. If you have installed Roblox Steam Launcher previously, `Uninstall` first, then run the installer again and choose `Install`

## Building

Building requires a Windows computer with an
installation of [Rust](https://www.rust-lang.org/en-US/install.html),
[Python 2.7](https://www.python.org/downloads/release/python-2713/), and [PyInstaller](http://www.pyinstaller.org/).  
The two build scripts (`build.bat` and `build_release.bat`) will run `cargo build` and `pyinstaller -F -y`
in the proper places and copy the executables to `build` or `build_release`.

## Issues

* Python is used for calculating the game id of a game because I could not figure out how
to set up the crc32 calculation properly in Rust, and I did not want to spend time
writing my own. It was fastest to bundle a python executable.
* The Roblox website will suggest that one downloads Roblox, as if it is not already installed.
Roblox still runs fine. This is probably an issue with the launcher taking too long, and it
sometimes happens even without the launcher installed.
* Errors in one half of the launching process are not handled well, and won't give any indication
if things break.

## Screenshots

![Steam Notification](http://i.imgur.com/E1eb1eh.png "Steam Notification")
![Steam Overlay](http://i.imgur.com/JURyXFe.png "Steam Overlay")
![Steam Controller Support](http://i.imgur.com/dOHbnDm.png "Steam Controller Support")
![Steam Keyboard Support](http://i.imgur.com/J9IDFMe.jpg "Steam Keyboard Support")

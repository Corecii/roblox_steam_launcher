cd roblox_steam_launcher_installer
cargo build --release
cd ..
cd roblox_steam_launcher_master
cargo build --release
cd ..
cd roblox_steam_launcher_substitute
cargo build --release
cd ..
cd python_gameid_crc
pyinstaller -F -y gameid.py
cd ..

mkdir build_release
copy /y roblox_steam_launcher_installer\target\release\roblox_steam_launcher_installer.exe build_release\roblox_steam_launcher_installer.exe
copy /y roblox_steam_launcher_master\target\release\roblox_steam_launcher_master.exe build_release\roblox_steam_launcher_master.exe
copy /y roblox_steam_launcher_substitute\target\release\roblox_steam_launcher_substitute.exe build_release\roblox_steam_launcher_substitute.exe
copy /y python_gameid_crc\dist\gameid.exe build_release\roblox_steam_launcher_gameid.exe

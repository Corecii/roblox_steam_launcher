cd roblox_steam_launcher_installer
cargo build
cd ..
cd roblox_steam_launcher_master
cargo build
cd ..
cd roblox_steam_launcher_substitute
cargo build
cd ..
cd python_gameid_crc
pyinstaller -F -y gameid.py
cd ..

mkdir build
copy /y roblox_steam_launcher_installer\target\debug\roblox_steam_launcher_installer.exe build\roblox_steam_launcher_installer.exe
copy /y roblox_steam_launcher_master\target\debug\roblox_steam_launcher_master.exe build\roblox_steam_launcher_master.exe
copy /y roblox_steam_launcher_substitute\target\debug\roblox_steam_launcher_substitute.exe build\roblox_steam_launcher_substitute.exe
copy /y python_gameid_crc\dist\gameid.exe build\roblox_steam_launcher_gameid.exe

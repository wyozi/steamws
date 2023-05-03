# steamws

Set of binaries for working with Steam Workshop and some game formats.
The goal is to provide composable binaries with few dependencies that are
easy to run in e.g. CI environments.

Included are

- `workshop` for downloading and inspecting workshop items
    - Utilizes Steamworks, so Steam must be running for this one.
- `gma` for working with Garry's Mod Addon files
- `mdl` for working with Source Engine model (.mdl) files
- `vtf` for working with Valve Texture Format (.vtf) files
- `bsp` for working with Source BSP map (.bsp) files

```bash
# print metadata about workshop
workshop info 1512211167

# download workshop file to a file
workshop get 1512211167 > file.gma

# download + list file entries inside .gma
workshop get 1512211167 | gma list -

# download + unpack files inside .gma to folder "out" in working dir
workshop get 1512211167 | gma unpack - out

# unpack files matching given filter from .gma in working dir
# UNIMPLEMENTED: gma unpack myaddon.gma out "**.mdl"

# download + print contents of all files matching pattern
workshop get 1512211167 | gma cat - "**.lua"

# Get item, unpack to folder, update a file, repackage as gma, update to workshop
workshop get 2137434632 | gma unpack - out && echo `date` > out/date.txt && gma pack out | workshop update 2137434632 -

# Fetch+Unpack workshop item, copy given .mdl and its dependencies (materials+textures) to another folder
workshop get 1512211167 | gma unpack - tiger && mdl cp tiger/models/kaesar/hobbs/hobbs.mdl my-content

# Extract entity lump from a file
bsp extract-entity-lump bowling.bsp bowling_final.bsp
```

## Quickstart

Grab a binary for your OS from the latest release:
https://github.com/wyozi/steamws/releases/latest/

If you're in CI environment, you can use this URL for the binary:
https://github.com/wyozi/steamws/releases/latest/download/gma_ubuntu-latest

For example:
```
curl -L https://github.com/wyozi/steamws/releases/latest/download/bsp_ubuntu-latest > bsp
chmod +x bsp

# Search for cubemap textures in map Pakfile
./bsp ls-pak mymap.bsp | grep -E "c\-?[0-9]+_\-?[0-9]+_\-?[0-9]+\.vtf"
```

## Compiling

Compile with [Steamworks SDK](https://partner.steamgames.com/downloads/steamworks_sdk.zip), e.g. `STEAM_SDK_LOCATION=~/Downloads/steam_sdk cargo build`

You can test locally with `cargo run --bin workshop -- get 1512211167 | cargo run --bin gma -- list`

For general use you'll probably want to install the binaries and place them on PATH. For that you can use `STEAM_SDK_LOCATION=~/Downloads/steam_sdk cargo install --path .`
If you get a fun error (like `dyld: Library not loaded: @loader_path/libsteam_api.dylib` on OS X), try moving the dynamic library file (dylib,so,dll) from `steam_sdk/redistributable_bin/<os>/` to where the binaries were installed to.

## Note about app_id

If you run any of the workshop commands without a game running in the background, Steam
will not be able to figure out the app_id and the binary will crash.

You can fix that by creating `steam_appid.txt` in the working directory or `SteamAppId` environment variable with the app id.
The `workshop` binary provides `-a <id>` option to temporarily create the file with given id,
and it'll also try its best to clean up the file before the command finishes.

## Credits

- Entity lump extraction is inspired by https://github.com/meepen/entremover_bsp
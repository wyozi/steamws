# steamws

Set of binaries for working with Steam Workshop. Utilizes Steamworks, so Steam must be running for this to work.

Included are

- `workshop` for downloading and inspecting workshop items
- `gma` for working with Garry's Mod Addon files

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

# example usecase:
# very rough anti-backdoor script grepping for "http" in all files
workshop get 426998109 | gma cat - | grep http

# Get item, unpack to folder, update a file, repackage as gma, update to workshop
workshop get 2137434632 | gma unpack - out && echo `date` > out/date.txt && gma pack out | workshop update 2137434632 -
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

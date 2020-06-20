# steamws

Set of binaries for working with Steam Workshop. Utilizes Steamworks, so Steam must be running for this to work.

```bash
# print metadata about workshop
# UNIMPLEMENTED: workshop info 1512211167

# download workshop file and cat to stdout
workshop get 1512211167 > file.gma

# download + list file entries inside .gma
workshop get 1512211167 | gma list

# download + unpack files inside .gma to folder "out" in working dir
# UNIMPLEMENTED: workshop get 1512211167 | gma unpack - out

# unpack files matching given filter from .gma in working dir
gma unpack myaddon.gma out **.mdl

# download + print contents of all files matching pattern
# UNIMPLEMENTED: workshop get 1512211167 | gma cat **.lua 
```

## Compiling 

Compile with [Steamworks SDK](https://partner.steamgames.com/?goto=%2Fdownloads%2Fsteamworks_sdk.zip), e.g. `STEAM_SDK_LOCATION=~/Downloads/steam_sdk cargo build`

You can test locally with `cargo run --bin workshop -- get 1512211167 | cargo run --bin gma -- list`
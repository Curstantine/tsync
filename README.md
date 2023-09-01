# tsync

CLI utility to sync tracks from your local music library to your mobile device, through ADB.

## Current issues

- ADB commands are done by using rust`s`Commands`API, instead of having a direct connection to the`adbd`.

- Doesn't not check for changes done to a file when syncing. Even if a change is done, the file will be skipped over if it exists in the target device.

# tsync

CLI utility to sync tracks from your local music library to your mobile device, through ADB.

## Features

- Syncing music to your mobile device through ADB with ease. (Faster and safer than MTP)
- Transcode tracks on the fly with custom quality settings.
- Select which tracks to sync based on file type, folder names and etc.
- Retains the source folder structure. e.g. `~/Music/Library` -> `/data/sdcard/Music/Library`
  - `~/Music/Library/Porter Robinson/SMILE! :D/01 Knock Yourself Out XD.flac` -> `/data/sdcard/Music/Porter Robinson/SMILE! :D/01 Knock Yourself Out XD.flac`

## Usage

Examples are based on library being in `~/Music/Library`, and mobile library being in `/data/sdcard/Music/Library`.

1. Syncing the entire library (no transcode)
   ```sh
   tsync sync ~/Music/Library /sdcard/Music/Library
   ```

2. Syncing the entire library with opus@128K transcode
   ```sh
   tsync sync -c opus -b 128 ~/Music/Library /sdcard/Music/Library
   ```

3. Syncing a select sync list.
   >[!NOTE]
   > A sync list is a plain-text file with with valid directories relative to the sync source separated by new lines.

   ```sh
   tsync sync --sync-list ./synclist.txt ~/Music/Library /sdcard/Music/Library
   ```

4. Syncing a select sync list with a no transcode flac filter
   ```sh
   tsync sync --sync-list ./synclist.txt --sync-codecs flac ~/Music/Library /sdcard/Music/Library
   ```

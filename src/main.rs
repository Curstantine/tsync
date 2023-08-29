use std::{
    fs, io,
    path::{Path, PathBuf},
    process::Command,
};

use indicatif::{ProgressBar, ProgressStyle};

const SOURCE_DIR: &str = "/data/storage/Music/Library/";
const TEMP_DIR: &str = "./tmp";
const TARGET_DIR: &str = "/sdcard/Music/Library/";

fn main() -> io::Result<()> {
    let is_adb_running = Command::new("adb")
        .arg("devices")
        .output()
        .map(|output| output.status.success())?;

    if !is_adb_running {
        return Err(io::Error::new(io::ErrorKind::Other, "ADB is not running"));
    }

    match fs::create_dir(TEMP_DIR) {
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => fs::remove_dir_all(TEMP_DIR)?,
        Err(e) => return Err(e),
        Ok(_) => {}
    }

    let files = read_dir_recursively(SOURCE_DIR)?
        .into_iter()
        .filter(|f| f.is_file() && f.extension().unwrap_or_default() == "flac")
        .collect::<Vec<_>>();

    println!("Found {} files", files.len());

    let indicator = ProgressBar::new(files.len() as u64);
    indicator.set_style(
        ProgressStyle::with_template(
            "{msg}\n[{elapsed_precise}] [{wide_bar:.cyan/blue}] [{pos}/{len}]",
        )
        .unwrap()
        .progress_chars("#>-"),
    );

    for file in files.into_iter() {
        let sub_path = file.strip_prefix(SOURCE_DIR).unwrap();
        let rel_path_display = sub_path.to_str().unwrap_or_default();

        let mut temp_path = Path::new(TEMP_DIR).join(sub_path);
        temp_path.set_extension("opus");

        indicator.set_message(format!("Transcoding {rel_path_display}"));
        fs::create_dir_all(temp_path.parent().unwrap())?;
        transcode_file(&file, &temp_path)?;

        indicator.set_message(format!("Pushing {rel_path_display}"));
        let target_path = Path::new(TARGET_DIR).join(sub_path.with_extension("opus"));
        push_to_adb_device(&temp_path, &target_path)?;

        fs::remove_file(temp_path)?;
        indicator.inc(1);
    }

    indicator.finish();

    // editor-fold
    // let chunks = files.chunks(2);
    // let mut handles = Vec::<JoinHandle<Result<(), Box<io::Error>>>>::new();
    //
    // for chunk in chunks {
    //     let chunk = chunk.to_vec();
    //     let handle: JoinHandle<Result<(), Box<io::Error>>> = thread::spawn(move || {
    //         for file in chunk.iter() {
    //             let sub_path = file.strip_prefix("/data/storage/Music/Library").unwrap();
    //             let mut target_path = Path::new("./tmp").join(sub_path);
    //             target_path.set_extension("opus");
    //             fs::create_dir_all(target_path.parent().unwrap())?;
    //             transcode_file(file, &target_path)?;
    //             println!("Transcoded {} to {}", file.display(), target_path.display());
    //         }
    //         Ok(())
    //     });
    //     handles.push(handle);
    // }
    //
    // for handle in handles {
    //     let results = handle.join().unwrap();
    //     if let Err(e) = results {
    //         println!("Error: {}", e);
    //     }
    // }}
    // editor-fold-end

    Ok(())
}

fn read_dir_recursively<P: AsRef<Path>>(path: P) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let mut sub_files = read_dir_recursively(path)?;
            files.append(&mut sub_files);
        } else {
            files.push(path);
        }
    }

    Ok(files)
}

fn transcode_file<P: AsRef<Path>>(source: P, target: P) -> io::Result<()> {
    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-i")
        .arg(source.as_ref().to_str().unwrap())
        .arg("-c:a")
        .arg("libopus")
        .arg("-b:a")
        .arg("128K")
        .arg(target.as_ref().to_str().unwrap());

    let output = cmd.output()?;
    if !output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "ffmpeg exited with code {}",
                output.status.code().unwrap_or(-1)
            ),
        ));
    }

    Ok(())
}

fn push_to_adb_device<P: AsRef<Path>>(source: P, target: P) -> io::Result<()> {
    let mut cmd = Command::new("adb");
    cmd.arg("push")
        .arg(source.as_ref().to_str().unwrap())
        .arg(target.as_ref().to_str().unwrap());

    let output = cmd.output()?;
    if !output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "adb exited with code {}",
                output.status.code().unwrap_or(-1)
            ),
        ));
    }

    Ok(())
}

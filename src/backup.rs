use std::{io::Error, process::{Command, ExitStatus}};

use chrono::Local;
use tracing::info;

pub fn start_backup(dir: &String, file_name: &String) -> Result<ExitStatus, Error> {
    let curr_date = Local::now().format("%m.%d.%Y").to_string();
    Command::new("zip")
        .args(["-vr", format!("./{} {}.zip", file_name, curr_date).as_str(), dir])
        .spawn()
        .expect("Should be able to spawn zip process")    
        .wait()
        .expect("Failed to zip server folder")
    ;
    info!("Finished zipping server folder into archive {} {}.zip", file_name, curr_date);
    Command::new("rclone")
        .args(["copy", "--update", format!("{} {}.zip", file_name, curr_date).as_str(), "gdrive:Minecraft-Servers/"])
        .spawn()
        .expect("Should be able to spawn rclone process")
        .wait()
}
use std::{path::Path, fs::read_to_string};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub restart_duration: u64,
    pub restart_warning_msgs: Vec<RestartWarning>,
    pub server_start_file: String,
    pub server_folder: String,
    pub backup_file_name: String,
    pub java_args: Args,
    pub nogui: bool
}

impl Config {
    pub fn read<P>(path: P) -> Self 
        where P: AsRef<Path> + std::fmt::Display
        {
        let config_data = read_to_string(path).expect("Failed To Read Config File. Make Sure You Have `config/server_bounce_config.json` in your root directory");

        let config_data = serde_json::from_str::<Config>(config_data.as_str()).expect("Failed To Parse Data From Config File. Possibly Invalid Syntax");

        config_data
    }
}

#[derive(Serialize, Deserialize)]
pub struct RestartWarning {
    pub msg: String,
    pub time: u64
}

pub type Args = Vec<String>;

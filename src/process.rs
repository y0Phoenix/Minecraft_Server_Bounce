use std::{io::{BufWriter, Write}, process::{ChildStdin, Child, Command, Stdio}, time::Duration, thread, fs::File, env};

use crate::config::Args;

pub struct Process {
    pub process: Child,
    pub stdin: BufWriter<ChildStdin>
}

impl Process {
    pub fn new(jar_file: String, args: Args, nogui: bool) -> Self {
        let valid_file = match File::open("server/".to_string() + &jar_file) {
            Ok(_) => true,
            Err(_) => false
        };

        if !valid_file {
            eprintln!("Error Accessing {} Check Config For `jar_file_name`", jar_file.clone());
        }

        let production = match env::var("PRODUCTION") {
            Ok(str) => str.as_str().parse::<bool>().expect("Error: Invalid Environment Variable PRODUCTION"),
            Err(_) => false
        };

        let mut process = Command::new("java")
            .current_dir(if production {"server"} else {"debug server"})
            .args(args)
            .arg("-jar")
            .arg(jar_file)
            .arg(if nogui {"nogui"} else {""})
            .stdin(Stdio::piped())
            .spawn()
            .expect("Failed to start java process")
        ;

        let stdin = BufWriter::new(process.stdin.take().expect("Failed To Aquire STD Input for Child Process"));

        Self {
            process,
            stdin
        }
    }
    pub fn kill(mut self) {
        self.stdin.write_all("/stop\n".as_bytes()).expect("Internal Error: Error While Writing To Std Input");
        self.stdin.flush().unwrap();
        match self.process.try_wait() {
            Err(io_err) => {
                eprintln!("Internal Error: Error Stoping Child Process {}: Will Try Again In 30 Seconds", io_err);
                thread::sleep(Duration::from_secs(30));
                self.stdin.write_all("/stop\n".as_bytes()).expect("Internal Error: Error While Writing To Std Input");
                self.stdin.flush().unwrap();
                self.process.try_wait().expect("Internal Error: Error Retrying To Stop Child Process");
            },
            _ => {}
        }
    }

    pub fn stdin_write(&mut self, input: String) {
        // write the msg to the sdtin buffer
        self.stdin.write_all(format!("/say {}\n", input).as_bytes()).expect("Error Writing To STD Input Buffer");
        // flush the buffer in order to ensure the bytes get pushed to the stdin
        self.stdin.flush().expect("Error Flushing STD Input Buffer");
    }
}
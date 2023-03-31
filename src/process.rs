use std::{io::{BufWriter, Write}, process::{ChildStdin, Child, Command, Stdio}, time::Duration, thread, fs::File};

pub struct Process {
    pub process: Child,
    pub stdin: BufWriter<ChildStdin>
}

impl Process {
    pub fn new(jar_file: String) -> Self {
        let valid_file = match File::open(jar_file.clone()) {
            Ok(_) => true,
            Err(_) => false
        };

        if !valid_file {
            eprintln!("Error Accessing {} Check Config For `jar_file_name`", jar_file.clone());
        }

        let mut process = Command::new("java")
            .current_dir("fabric minecraft server")
            .arg("-Xmx2G")
            .arg("-jar")
            .arg(jar_file)
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
}
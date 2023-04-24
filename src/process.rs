use std::{io::{BufWriter, Write}, process::{ChildStdin, Child, Command, Stdio}, time::Duration, thread::{self, JoinHandle}, fs::File, env, sync::{Arc, Mutex, mpsc::{Sender, self}}};

use crate::config::Args;

pub struct Process {
    killed: Arc<Mutex<bool>>,
    stop_checker_thread: JoinHandle<()>,
    send_kill_tx: Sender<bool>,
    pub stdin: Arc<Mutex<BufWriter<ChildStdin>>>
}

impl Process {
    pub fn new(jar_file: String, server_folder: String, args: Args, nogui: bool) -> Self {
        let valid_file = File::open(server_folder.clone() + &jar_file).is_ok(); 

        if !valid_file {
            eprintln!("Error Accessing {} Check Config For `jar_file_name`", jar_file);
        }

        let production = match env::var("PRODUCTION") {
            Ok(str) => str.as_str().parse::<bool>().expect("Error: Invalid Environment Variable PRODUCTION"),
            Err(_) => false
        };

        println!("Attempting To Start Jar File {} in {}", jar_file, server_folder);

        let killed = Arc::new(Mutex::new(false));
        let killed_clone = Arc::clone(&killed);

        let (send_kill_tx, send_kill_rx) = mpsc::channel::<bool>();

        let mut process = spawn_process(&jar_file, &server_folder, &args, &nogui, &production);

        let stdin = Arc::new(Mutex::new(BufWriter::new(process.stdin.take().expect("Failed To Aquire STD Input for Child Process"))));

        let stdin_clone = Arc::clone(&stdin);

        let stop_checker_thread = thread::Builder::new()
            .name("stop_checker".to_string())
            .spawn(move || {
                let killed = killed_clone;
                loop {
                    if let Ok(bool) = send_kill_rx.recv_timeout(Duration::from_secs(1)) {
                        let mut stdin_clone = stdin_clone.lock().unwrap();
                        stdin_clone.write_all("/stop".as_bytes()).unwrap();
                        stdin_clone.flush().unwrap();
                        if let Err(io_err) = process.try_wait() {
                            eprintln!("Internal Error: Error Stoping Child Process {}: Will Try Again In 30 Seconds", io_err);
                            thread::sleep(Duration::from_secs(30));
                            stdin_clone.write_all("/stop\n".as_bytes()).expect("Internal Error: Error While Writing To Std Input");
                            stdin_clone.flush().unwrap();
                            process.try_wait().expect("Internal Error: Error Retrying To Stop Child Process");
                        }
                        if bool {
                            break;
                        }
                        else {
                            process = spawn_process(&jar_file, &server_folder, &args, &nogui, &production);
                            let stdin = BufWriter::new(process.stdin.take().unwrap());
                            *stdin_clone = stdin;
                            continue;
                        }
                    }    
                    if let Ok(Some(_)) = process.try_wait() {
                        println!("Minecraft Server Unexpectedly Stopped Attemping To Restart It");
                        *killed.lock().unwrap() = true;
                        process = spawn_process(&jar_file, &server_folder, &args, &nogui, &production);
                        let stdin = BufWriter::new(process.stdin.take().unwrap());
                        *stdin_clone.lock().unwrap() = stdin;
                    }
                }
            })
        .unwrap();

        Self {
            killed,
            stdin,
            stop_checker_thread,
            send_kill_tx,
        }
    }
    pub fn kill(self) {
        let _ = self.send_kill_tx.send(true);
        self.stop_checker_thread.join().unwrap();
    }

    pub fn stdin_write(&mut self, input: String) {
        let mut stdin = self.stdin.lock().unwrap();
        // write the msg to the sdtin buffer
        stdin.write_all(format!("/say {}\n", input).as_bytes()).expect("Error Writing To STD Input Buffer");
        // flush the buffer in order to ensure the bytes get pushed to the stdin
        stdin.flush().expect("Error Flushing STD Input Buffer");
    }
    pub fn restart(&mut self) {
       let _ = self.send_kill_tx.send(false); 
    }
    pub fn is_stopped(&self) -> bool {
        *self.killed.lock().unwrap()
    }
}

fn spawn_process(jar_file: &String, server_folder: &str, args: &Args, nogui: &bool, production: &bool) -> Child {
    Command::new("java")
        .current_dir(if *production {server_folder} else {"debug server"})
        .args(args)
        .arg("-jar")
        .arg(jar_file)
        .arg(if *nogui {"nogui"} else {""})
        .stdin(Stdio::piped())
        .spawn()
        .expect("Failed to start java process")
}

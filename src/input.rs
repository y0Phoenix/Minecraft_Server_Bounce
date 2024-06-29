use std::{collections::HashSet, str::SplitWhitespace, sync::{mpsc::{self, Receiver}, Arc, Mutex}, thread::{self, JoinHandle}};

pub struct Input {
    check_input_thread: JoinHandle<()>,
    input_rx: Receiver<String>,
    killed: Arc<Mutex<bool>>
}

impl Input {
    pub fn new() -> Self {
        let (input_tx, input_rx) = mpsc::channel();

        let killed = Arc::new(Mutex::new(false));

        let killed_clone1 = Arc::clone(&killed);

        let check_input_thread = thread::Builder::new()
            .name("checkinput".to_string())
            .spawn(move|| {
                let killed = killed_clone1;
                loop {
                    if *killed.lock().unwrap() {
                        println!("[thread:checkinput]: Closing Thread");
                        break;
                    }
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input).unwrap();
                    input = input.trim().to_string();
                    if input_tx.send(input.clone()).is_err() {
                        break;
                    }
                    if input == *"stop"{
                        println!("Closing [thread:checkinput]");
                        drop(input_tx);
                        break;
                    }
                }
            })
            .unwrap()
            ;
        
        Self { 
            check_input_thread,
            input_rx,
            killed
         }
    }

    pub fn new_input(&mut self) -> Option<String> {
        match self.input_rx.recv() {
            Ok(input) => {
                return Some(input);
            },
            Err(_) => {
                return None;
            }
        }
    }

    pub fn kill(self) {
        *self.killed.lock().unwrap() = true;
        self.check_input_thread.join().unwrap();
        println!("Threads Closed");
    }

    pub fn parse_input(input: String) -> InputCode {
        let mut parts = input.split_whitespace();

        let mut flags = HashSet::<InputFlag>::new();

        let mut command = InputCommand::default();

        let default_twice_command_err = InputCode::InvalidMsg("Error: You Can't Use A Command Twice usage: say \"Restarting in 50 minutes...\"".to_string());
        
        while let Some(str) = parts.next() {
            match str {
                "-t" => {
                    if let Some(time) = parts.next() {
                        match time.parse::<u64>() {
                            Ok(time) => {
                                flags.insert(InputFlag::Time(time));
                                continue;
                            },
                            Err(_) => return InputCode::InvalidMsg("Error: Time Must A Valid u64 Number usage: restart -m \"Restarting\" -t 3000".to_string())
                        }
                    }
                    return InputCode::InvalidMsg("Error: You Need To Specify A Time After The `-t` Flag usage: restart -m \"Restarting\" -t 3000".to_string());
                },
                "-m" => {
                    if let Some(message) = Input::parse_msg(&mut parts) {
                        flags.insert(InputFlag::Msg(message));
                        continue;
                    }
                    return InputCode::InvalidMsg("Error: Invalid Message Format usage: restart -m \"Restarting in 50 minutes\"".to_string());
                },
                "say" => {
                    if command != InputCommand::default() {
                        return default_twice_command_err;
                    }
                    if let Some(msg) = Input::parse_msg(&mut parts) {
                        return InputCode::SendMsg(msg);
                    }
                    return InputCode::InvalidMsg("Error: Invalid Message Format After `say` usage: say \"Restarting In 50 Minutes\"".to_string());
                },
                "stop" => {
                    if command != InputCommand::default() {
                        return default_twice_command_err
                    }
                    return InputCode::Exit;
                },
                "restart" => {
                    if command != InputCommand::default() {
                        return default_twice_command_err
                    }
                    command = InputCommand::Restart;
                },
                "backup" => {
                    if command != InputCommand::default() {
                        return default_twice_command_err
                    }
                    return InputCode::Backup;
                },
                "cmd" => {
                    if command != InputCommand::default() {
                        return default_twice_command_err;
                    }
                    if let Some(cmd) = Input::parse_msg(&mut parts) {
                        return InputCode::Cmd(cmd);
                    }
                    return InputCode::InvalidMsg("Error: Invalid Message Format After `cmd` usage: cmd \"/op <user to op>\"".to_string());
                }
                _ => {}
            }
        }

        if command == InputCommand::Restart {
            let mut time = 0;
            let mut message = String::new();
            for flag in flags.into_iter() {
                match flag {
                    InputFlag::Msg(msg) => {
                        if message.is_empty() {
                            message = msg;
                            continue;
                        }
                        return InputCode::InvalidMsg("Error: Too Many `-m` Flags usage: restart -m \"Restarting in 50 minutes...\" -t 3000".to_string());
                    },
                    InputFlag::Time(t) => {
                        if time == 0 && t > 0{
                            time = t;
                            continue;
                        }
                        return InputCode::InvalidMsg("Error: Either Time Is 0 Or Too Many `-t` Flags usage: restart -m \"Restarting in 50 minutes\" -t 3000".to_string());
                    }
                }
            }
            if time > 0 && !message.is_empty() {
                return InputCode::RestartWithMsgTime(message, time);
            }
            else if time > 0 {
                return  InputCode::RestartWithTime(time);
            }
            else if !message.is_empty() {
                return InputCode::RestartWithMsg(message);
            }
            return InputCode::Restart;
        }
        InputCode::Invalid
    }   

    fn parse_msg(parts: &mut SplitWhitespace) -> Option<String> {
        let mut start = false;

        let mut msg = String::new();

        loop {
            match parts.next() {
                Some(part) => {
                    // siingle word message
                    if part.starts_with('"') && part.ends_with('"'){
                        let tmp_msg = <&str>::clone(&part).replace('"', "");
                        msg.push_str(format!("{} ", tmp_msg).as_str());
                        return Some(msg);
                    }
                    // start message
                    else if part.starts_with('"') {
                        start = true;
                        let tmp_msg = <&str>::clone(&part).replace('"', "");
                        msg.push_str(format!("{} ", tmp_msg).as_str()); 
                    }
                    // end message
                    else if start && part.ends_with('"') {
                        let tmp_msg = <&str>::clone(&part).replace('"', "");
                        msg.push_str(format!("{} ", tmp_msg).as_str());
                        return Some(msg);
                    }
                    // add to message
                    else if start {
                        msg.push_str(format!("{} ", part).as_str());
                    }
                    // invalid message
                    else {
                        return None;
                    }
                },
                None => return None
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum InputCode {
    SendMsg(String),
    RestartWithMsg(String),
    RestartWithTime(u64),
    RestartWithMsgTime(String, u64),
    Restart,
    Exit,
    Invalid,
    InvalidMsg(String),
    Backup,
    Cmd(String)
}

#[derive(PartialEq, Eq, Hash)]
pub enum InputFlag {
    Time(u64),
    Msg(String)
}

impl InputFlag {
    
}

#[derive(Default, PartialEq, Eq)]
pub enum InputCommand {
    Restart,
    #[default]
    NoCommand
}

use std::{time::{Instant, Duration}, thread};

use config::Config;
use input::Input;
use process::Process;
use rusty_time::Timer;

mod config;
mod process;
mod input;

#[derive(Debug, Default, PartialEq, Eq)]
pub enum AppState {
    RestartWithTime(u64),
    Exit,
    #[default]
    Normal
}

fn main() {
    dotenv::dotenv().ok();

    let config_data = Config::read("config/server_bounce_config.json");
    
    let mut instant = Instant::now();

    let mut input = Input::new();

    let mut app_state = AppState::default();
    
    // main loop for starting a new process and new timers
    'main: loop {
        // start the child process and grab the stdin and child process 
        let mut child = Process::new(
            config_data.jar_file_name.clone(), 
            config_data.server_folder.clone(), 
            config_data.java_args.clone(), 
            config_data.nogui
        );

        // create an iterator over the config warning msgs
        let mut warning_msgs = config_data.restart_warning_msgs.iter().enumerate();

        // create two timers one for the reset duration and the other for the warning messages
        println!("creating new warning timer for {} millis", config_data.restart_warning_msgs.get(0).expect("No Warning Msg Configs Found").time * 1000);
        let mut warning_timer = Timer::from_millis(config_data.restart_warning_msgs.get(0).expect("No Warning Msg Configs Found").time * 1000);
        println!("creating new restart timer for {} millis", config_data.restart_duration * 1000);
        let mut reset_timer = Timer::from_millis(config_data.restart_duration * 1000);

        // inner loop for checking timers
        'timer: loop {
            if app_state != AppState::default() {
                break 'timer;
            }
            // grab the current delta
            let delta = instant.elapsed();
            instant = Instant::now();

            // update the timer with the delta
            reset_timer.update(delta);
            warning_timer.update(delta);

            // check for user input 
            if let Some(new_input) = input.new_input() {
                match Input::parse_input(new_input) {
                    input::InputCode::SendMsg(msg) => child.stdin_write(msg),
                    input::InputCode::RestartWithMsg(msg) => {
                        child.stdin_write(msg);
                        thread::sleep(Duration::from_secs(5));
                        break 'timer;
                    },
                    input::InputCode::RestartWithTime(time) => {
                        app_state = AppState::RestartWithTime(time);
                        child.stdin_write(format!("Manual Restart In {}", time));
                        break 'timer;
                    },
                    input::InputCode::RestartWithMsgTime(msg, time) => {
                        app_state = AppState::RestartWithTime(time);
                        child.stdin_write(msg);
                        break 'timer;
                    },
                    input::InputCode::Restart => {
                        child.stdin_write("Manual Restart In 10 Seconds...".to_string());
                        thread::sleep(Duration::from_secs(10));
                        break 'timer;
                    },
                    input::InputCode::Exit => {
                        child.stdin_write("Manual Server Shutdown In 10 Seconds...".to_string());
                        thread::sleep(Duration::from_secs(10));
                        break 'main;
                    },
                    input::InputCode::Invalid => println!("Error: Invalid Command Input usage: restart -m \"Restarting In 10 Minutes...\" -t 600"),
                    input::InputCode::InvalidMsg(msg) => println!("{}", msg)
                }
            }

            // check if we are ready to send a warning message
            if warning_timer.ready {
                // grab the next warning message from the iterator
                if let Some(current_msg) = warning_msgs.next() {
                    let (i, current_msg) = current_msg;
                    println!("sending /say {}", current_msg.msg);
                    
                    // write the timed msg to the child stdin
                    child.stdin_write(current_msg.msg.to_string());

                    // set the new duration to the next time instead of the current one
                    if let Some(new_durration) = config_data.restart_warning_msgs.get(i + 1) {
                        println!("new timer duration {}", new_durration.time);
                        warning_timer.duration = Duration::from_secs(new_durration.time);
                    }
                    else {
                        println!("end of new timers");
                    }
                    // reset the timer after the new duration is set
                    warning_timer.reset();
                }
            }
            // check if the reset timer is ready
            if reset_timer.ready {
                println!("restart timer ready");
                break 'timer;
            }
            // sleep the current thread. We don't need to check as fast as we can. The implemenation can afford a slow check
            thread::sleep(Duration::from_secs(1));
        }
        // when we enter a manual restart with a timer
        if let AppState::RestartWithTime(time) = app_state {
            println!("creating new restart timer for {}", time * 1000);
            let mut custom_timer = Timer::from_millis(time * 1000);
            'customrestart: loop {
                let delta = instant.elapsed();
                custom_timer.update(delta);
                instant = Instant::now();

                if custom_timer.ready {
                    child.stdin_write("Manual Restart In 10 Seconds".to_string());
                    thread::sleep(Duration::from_secs(10));
                    break 'customrestart;
                }
                // sleep the current thread. We don't need to check as fast as we can. The implemenation can afford a slow check
                thread::sleep(Duration::from_secs(1));
            }
        }
        // stop the current child process
        child.kill();
        if app_state == AppState::Exit {
            break 'main;
        }
    }
    println!("Exiting App");
    input.kill();
}

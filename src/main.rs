use std::{time::{Instant, Duration}, thread};
use backup::start_backup;
use config::Config;
use input::Input;
use process::Process;
use rusty_time::Timer;
use tracing::{error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

mod config;
mod process;
mod input;
mod backup;

#[derive(Debug, Default, PartialEq, Eq)]
pub enum AppState {
    RestartWithTime(u64),
    Exit,
    #[default]
    Normal
}

fn main() {
    dotenv::dotenv().ok();

    // a builder for `FmtSubscriber`.
    let subscriber = FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::TRACE)
        // completes the builder.
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    let config_data = Config::read("config/server_bounce_config.json");
    
    let mut instant = Instant::now();

    let mut input = Input::new();

    let mut app_state = AppState::default();
    
    // start the child process and grab the stdin and child process 
    let mut child = Process::new(
        config_data.server_start_file.clone(), 
        config_data.server_folder.clone(), 
        config_data.java_args.clone(), 
        config_data.nogui
        );
    // main loop for starting a new process and new timers
    'main: loop {
        // create an iterator over the config warning msgs
        let mut warning_msgs = config_data.restart_warning_msgs.iter().enumerate();

        // create two timers one for the reset duration and the other for the warning messages
        info!("creating new warning timer for {} minutes", config_data.restart_warning_msgs.get(0).expect("No Warning Msg Configs Found").time / 60);
        let mut warning_timer = Timer::from_millis(config_data.restart_warning_msgs.get(0).expect("No Warning Msg Configs Found").time * 1000);
        info!("creating new restart timer for {} minutes", config_data.restart_duration / 60);
        let mut reset_timer = Timer::from_millis(config_data.restart_duration * 1000);

        // inner loop for checking timers
        'timer: loop {
            if app_state != AppState::default() || child.is_stopped() {
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
                    input::InputCode::SendMsg(msg) => child.say(msg),
                    input::InputCode::RestartWithMsg(msg) => {
                        child.say(msg);
                        thread::sleep(Duration::from_secs(5));
                        break 'timer;
                    },
                    input::InputCode::RestartWithTime(time) => {
                        app_state = AppState::RestartWithTime(time);
                        child.say(format!("Manual Restart In {}", time));
                        break 'timer;
                    },
                    input::InputCode::RestartWithMsgTime(msg, time) => {
                        app_state = AppState::RestartWithTime(time);
                        child.say(msg);
                        break 'timer;
                    },
                    input::InputCode::Restart => {
                        child.say("Manual restart in 30 seconds...".to_string());
                        thread::sleep(Duration::from_secs(30));
                        break 'timer;
                    },
                    input::InputCode::Exit => {
                        child.say("Manual server shutdown in 30 seconds...".to_string());
                        thread::sleep(Duration::from_secs(30));
                        break 'main;
                    },
                    input::InputCode::Invalid => warn!("Error: Invalid Command Input usage: restart -m \"Restarting In 10 Minutes...\" -t 600"),
                    input::InputCode::InvalidMsg(msg) => warn!("{}", msg),
                    input::InputCode::Backup => {
                        child.say("Manual server backup in 1 minute. Server will shutdown and may take ahwile to restart.".to_string());
                        thread::sleep(Duration::from_secs(5));
                        match start_backup(&config_data.server_folder, &config_data.backup_file_name) {
                            Ok(s) => {
                                if s.success() {
                                    info!("Backup created and uploaded to Google Drive {}", s.to_string());
                                }
                            },
                            Err(err) => error!("Failed to create and upload backup to Google Drive: {}", err),
                        }
                        break 'timer;
                    },
                    input::InputCode::Cmd(cmd) => child.cmd(cmd),
                }
            }

            // check if we are ready to send a warning message
            if warning_timer.ready {
                // grab the next warning message from the iterator
                if let Some(current_msg) = warning_msgs.next() {
                    let (i, current_msg) = current_msg;
                    info!("sending /say {}", current_msg.msg);
                    
                    // write the timed msg to the child stdin
                    child.say(current_msg.msg.to_string());

                    // set the new duration to the next time instead of the current one
                    if let Some(new_durration) = config_data.restart_warning_msgs.get(i + 1) {
                        info!("new timer duration {} minutes", new_durration.time / 60);
                        warning_timer.duration = Duration::from_secs(new_durration.time);
                    }
                    else {
                        info!("end of new timers");
                    }
                    // reset the timer after the new duration is set
                    warning_timer.reset();
                }
            }
            // check if the reset timer is ready
            if reset_timer.ready {
                info!("restart timer ready");
                break 'timer;
            }
            // sleep the current thread. We don't need to check as fast as we can. The implemenation can afford a slow check
            thread::sleep(Duration::from_secs(1));
        }
        // when we enter a manual restart with a timer
        if let AppState::RestartWithTime(time) = app_state {
            info!("creating new restart timer for {} minutes", time / 60);
            let mut custom_timer = Timer::from_millis(time * 1000);
            'customrestart: loop {
                let delta = instant.elapsed();
                custom_timer.update(delta);
                instant = Instant::now();

                if custom_timer.ready {
                    break 'customrestart;
                }
                // sleep the current thread. We don't need to check as fast as we can. The implemenation can afford a slow check
                thread::sleep(Duration::from_secs(1));
            }
        }
        // stop the current child process
        child.restart();
        if app_state == AppState::Exit {
            break 'main;
        }
        app_state = AppState::default();
    }
    info!("Exiting App");
    child.kill();
    input.kill();
    thread::sleep(Duration::from_secs_f32(3.5));
}

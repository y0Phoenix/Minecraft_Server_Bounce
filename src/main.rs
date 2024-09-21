use backup::start_backup;
use chrono::Local;
use config::Config;
use input::Input;
use lettre::{message::header::ContentType, transport::smtp::authentication::Credentials, Message, SmtpTransport, Transport};
use process::Process;
use rusty_time::Timer;
use std::{
    fs::remove_file, thread, time::{Duration, Instant}
};
use tracing::{error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

mod backup;
mod config;
mod input;
mod process;

#[derive(Debug, Default, PartialEq, Eq)]
pub enum AppState {
    RestartWithTime(u64),
    Exit,
    Backup,
    #[default]
    Normal,
}

fn main() {
    dotenv::dotenv().ok();
    let (gmail, pass) = (dotenv::var("EMAILER_EMAIL").expect("EMAILER_EMAIL envar should exist"), dotenv::var("EMAILER_PASS").expect("EMAILER_PASS envar should exist"));

    // a builder for `FmtSubscriber`.
    let subscriber = FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::TRACE)
        // completes the builder.
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let config_data = Config::read("config/server_bounce_config.json");
    let (hour, minute) = config_data.backup_time.split_at(3);
    let (mut hour, minute) = (hour.to_string(), minute.to_string());
    hour.remove(2);

    // let status = start_backup(&config_data.server_folder, &config_data.backup_file_name).unwrap();
    // if status.code().unwrap() == 1 {
    //     println!("yay");
    // }

    let mut instant = Instant::now();

    let mut input = Input::new();

    let mut app_state = AppState::default();

    let mut child: Option<Process>;

    // main loop for starting a new process and new timers
    'main: loop {
        // start the child process and grab the stdin and child process
        child = Some(Process::new(
            config_data.server_start_file.clone(),
            config_data.server_folder.clone(),
            config_data.java_args.clone(),
            config_data.nogui,
        ));
        if let Some(child) = &mut child {
            app_state = AppState::default();
            'restart: loop {
                // create an iterator over the config warning msgs
                let mut warning_msgs = config_data.restart_warning_msgs.iter().enumerate();

                info!("Restart loop started with app_state {:?}", app_state);

                // create two timers one for the reset duration and the other for the warning messages
                info!(
                    "creating new warning timer for {} minutes",
                    config_data
                        .restart_warning_msgs
                        .get(0)
                        .expect("No Warning Msg Configs Found")
                        .time
                        / 60
                );
                let mut warning_timer = Timer::new(Duration::from_millis(
                    config_data
                        .restart_warning_msgs
                        .get(0)
                        .expect("No Warning Msg Configs Found")
                        .time
                        * 1000,
                ));
                info!(
                    "creating new restart timer for {} minutes",
                    config_data.restart_duration / 60
                );
                let mut reset_timer =
                    Timer::new(Duration::from_millis(config_data.restart_duration * 1000));

                // inner loop for checking timers
                'timer: loop {
                    if app_state != AppState::default() || child.is_stopped() {
                        break 'timer;
                    }
                    // grab the current delta
                    let delta = instant.elapsed();
                    instant = Instant::now();

                    let curr_hour = Local::now().format("%H").to_string();
                    let curr_min = Local::now().format("%M").to_string().parse::<u8>().expect("Should be a valid u8");

                    // println!("{} {}", curr_hour, hour);
                    if curr_hour == hour {
                        let min = match minute.parse::<u8>() {
                            Ok(min) => min,
                            Err(_) => panic!("backup_time config data is invalid. Use 24 hour time format."),
                        };
                        if min.abs_diff(curr_min) == 1 {
                            child.say("Automatic server backup in 1 minute. Server will shutdown and may take ahwile to restart.".to_string());
                            thread::sleep(Duration::from_secs(60));
                            app_state = AppState::Backup;
                            break 'restart;
                        }
                    }

                    // update the timer with the delta
                    reset_timer.tick(delta);
                    warning_timer.tick(delta);

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
                                thread::sleep(Duration::from_secs(60));
                                app_state = AppState::Backup;
                                break 'restart;
                            },
                            input::InputCode::Cmd(cmd) => child.cmd(cmd),
                        }
                    }

                    // check if we are ready to send a warning message
                    if warning_timer.finished() {
                        // grab the next warning message from the iterator
                        if let Some(current_msg) = warning_msgs.next() {
                            let (i, current_msg) = current_msg;
                            info!("sending /say {}", current_msg.msg);

                            // write the timed msg to the child stdin
                            child.say(current_msg.msg.to_string());

                            // set the new duration to the next time instead of the current one
                            if let Some(new_durration) = config_data.restart_warning_msgs.get(i + 1)
                            {
                                info!("new timer duration {} minutes", new_durration.time / 60);
                                warning_timer.set_duration(Duration::from_secs(new_durration.time));
                            } else {
                                info!("end of new timers");
                            }
                            // reset the timer after the new duration is set
                            warning_timer.reset();
                        }
                    }
                    // check if the reset timer is ready
                    if reset_timer.finished() {
                        info!("restart timer ready");
                        break 'timer;
                    }
                    // sleep the current thread. We don't need to check as fast as we can. The implemenation can afford a slow check
                    thread::sleep(Duration::from_secs(1));
                }
                // when we enter a manual restart with a timer
                if let AppState::RestartWithTime(time) = app_state {
                    info!("creating new restart timer for {} minutes", time / 60);
                    let mut custom_timer = Timer::new(Duration::from_millis(time * 1000));
                    'customrestart: loop {
                        let delta = instant.elapsed();
                        custom_timer.tick(delta);
                        instant = Instant::now();

                        if custom_timer.finished() {
                            break 'customrestart;
                        }
                        // sleep the current thread. We don't need to check as fast as we can. The implemenation can afford a slow check
                        thread::sleep(Duration::from_secs(1));
                    }
                }
                // stop the current child process
                if app_state == AppState::Exit {
                    break 'main;
                }
                child.restart();
                app_state = AppState::default();
            }
        }
        child.expect("Should be a child process").kill();
        if app_state == AppState::Backup {
            match start_backup(&config_data.server_folder, &config_data.backup_file_name) {
                Ok(s) => {
                    if s.success() {
                        let curr_date = Local::now().format("%m.%d.%Y").to_string();
                        match remove_file(format!(
                            "./{} {}.zip",
                            config_data.backup_file_name, curr_date
                        )) {
                            Ok(_) => info!("local server zip file deleted"),
                            Err(e) => error!("failed to remove local server zip file {}", e),
                        }
                        info!(
                            "Backup created and uploaded to Google Drive {}",
                            s.to_string()
                        );
                    }
                    else if s.code().unwrap() == 1 {
                        error!(
                            "Failed to create and upload backup to Google Drive"
                        );
                        let email = Message::builder()
                            .from("Aaron Graybill <aarongraybill3@gmail.com>".parse().unwrap())
                            .to("Aaron Graybill <aarongraybill3@gmail.com>".parse().unwrap())
                            .subject("Minecraft Server Backup Issue")
                            .header(ContentType::TEXT_PLAIN)
                            .body(String::from("Please fix thnx"))
                            .unwrap();
        
                        let creds = Credentials::new(gmail.clone(), pass.clone());
        
                        // Open a remote connection to gmail
                        let mailer = SmtpTransport::relay("smtp.gmail.com")
                            .unwrap()
                            .credentials(creds)
                            .build();
        
                        // Send the email
                        match mailer.send(&email) {
                            Ok(_) => println!("Email sent successfully!"),
                            Err(e) => panic!("Could not send email: {e:?}"),
                        }
                    }
                }
                Err(err) => {
                    error!(
                        "Failed to create and upload backup to Google Drive: {}",
                        err
                    );
                    let email = Message::builder()
                        .from("Aaron Graybill <aarongraybill3@gmail.com>".parse().unwrap())
                        .to("Aaron Graybill <aarongraybill3@gmail.com>".parse().unwrap())
                        .subject("Minecraft Server Backup Issue")
                        .header(ContentType::TEXT_PLAIN)
                        .body(String::from("Please fix thnx"))
                        .unwrap();

                    let creds = Credentials::new("aarongraybill3@gmail.com".to_owned(), "xzbitnriwxdnhycf".to_owned());

                    // Open a remote connection to gmail
                    let mailer = SmtpTransport::relay("smtp.gmail.com")
                        .unwrap()
                        .credentials(creds)
                        .build();

                    // Send the email
                    match mailer.send(&email) {
                        Ok(_) => println!("Email sent successfully!"),
                        Err(e) => panic!("Could not send email: {e:?}"),
                    }
                },
            }
        }
    }
    info!("Exiting App");
    child.expect("Should be a child").kill();
    input.kill();
    thread::sleep(Duration::from_secs_f32(3.5));
}

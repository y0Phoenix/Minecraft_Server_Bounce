use std::{time::{Instant, Duration}, thread};
use std::io::Write;

use config::Config;
use process::Process;
use rusty_time::Timer;

mod config;
mod process;

fn main() {
    let config_data = Config::read("config/server_bounce_config.json");
    
    let mut instant = Instant::now();
    
    // main loop for starting a new process and new timers
    'main: loop {
        // start the child process and grab the stdin and child process 
        let mut child = Process::new(config_data.jar_file_name.clone(), config_data.java_args.clone());

        // create an iterator over the config warning msgs
        let mut warning_msgs = config_data.restart_warning_msgs.iter().enumerate();

        // create two timers one for the reset duration and the other for the warning messages
        println!("creating new warning timer for {} millis", config_data.restart_warning_msgs.get(0).expect("No Warning Msg Configs Found").time * 1000);
        let mut warning_timer = Timer::from_millis(config_data.restart_warning_msgs.get(0).expect("No Warning Msg Configs Found").time * 1000);
        println!("creating new restart timer for {} millis", config_data.restart_duration * 1000);
        let mut reset_timer = Timer::from_millis(config_data.restart_duration * 1000);

        // inner loop for checking timers
        'timer: loop {
            // grab the current delta
            let delta = instant.elapsed();
            instant = Instant::now();

            // update the timer with the delta
            reset_timer.update(delta);
            warning_timer.update(delta);

            // check if we are ready to send a warning message
            if warning_timer.ready {
                // grab the next warning message from the iterator
                if let Some(current_msg) = warning_msgs.next() {
                    let (i, current_msg) = current_msg;
                    println!("sending /say {}", current_msg.msg);
                    // write the msg to the sdtin buffer
                    child.stdin.write_all(format!("/say {}\n", current_msg.msg).as_bytes()).expect("Error Writing To STD Input Buffer");
                    // flush the buffer in order to ensure the bytes get pushed to the stdin
                    child.stdin.flush().expect("Error Flushing STD Input Buffer");

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
                // stop the current child process
                child.kill();
                break 'timer;
            }
            // sleep the current thread. We don't need to check as fast as we can. The implemenation can afford a slow check
            thread::sleep(Duration::from_millis(1000));
        }
    }
}

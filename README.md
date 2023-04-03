# Minecraft Server Bounce

### Description
Basic Rust Program Thats will Start and Manage your minecraft server. It manages a configurable Automatic Restart.

It will also send warning messages to the server based off of user configuration

### Usage 
config/server_bounce_config.json
```json
"jar_file_name": "<path-to-your-jarfile-here>",
"server_folder": "server",
"java_args": [
    "-Xmx6G",
],
"nogui": true,
"restart_duration": 60,
"restart_warning_msgs": [
    {
        "msg": "<your-warning-message-here>",
        "time": 30
    }
] 
```

The `jar_file_name` is meant for the server jarfile that you would get from Mojang or Fabric if your doing a modded server

The `server_folder` is for the servers root directory. this is where you will have all of your minecraft server files

The `java_args` are the optional arguments to pass to the java process for optimaization or in the example above, allocating 6gb of memory and nogui so it will only
run in the terminal. However you can adjust these arguments however you like

The `nogui` is a minecraft server argument to determine wheter or not to display the gui for server management

The `restart_duration` is the time in `seconds` that need to be elapsed before the server will attempt to restart

THE `restart_warning_msgs` are there so you can send a message to the server whenever the `time` is elapsed

### Using Commands
* `restart` 
    `restart` will send a message to the server `Manual Restart In 10 Seconds...` then the server will save and restart
    `restart -m <msg>` will send your custom message to the server then the server will save and restart in 5 seconds
    `restart -m <msg> -t <time in seconds>` will send the your custom message to the server then the server will save and restart once your custom time in seconds
    has elapsed

* `stop` 
    `stop` will send a message to the server `Manual Server Shutdown In 10 Seconds...` then the server will save, shutdown and the program will exit

* `say` 
    `say <msg>` will send your custom message to the server

### How to get started with your server
Currently there is no binary with the files to download.

If your looking to use yourself, you will need to install the [rustup]("https://rustup.rs/") package to install the proper rust workspace.

Clone the repo to your machine then configure the config file to your needs.

After your configuration is complete just run the following command in your terminal
```bash
cargo run --release
```
this will compile and run the code in release mode

If all your configuration is correct you should see your server starting up in the same terminal

### Additional Info
if your trying to get the warning messages timed down to the second from the `restart_duration` try adding 2-4 extra seconds to the `restart_duration`. 
there are some inconsistencies with the timer.
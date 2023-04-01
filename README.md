# Minecraft Server Bounce

### Description
Basic Rust Program Thats will Start and Manage your minecraft server. It manages a configurable Automatic Restart.

It will also send warning messages to the server based off of user configuration

### Usage 
config/server_bounce_config.json
```json
"jar_file_name": "<path-to-your-jarfile-here>",
"java_args": [
    "-Xmx6G",
    "nogui"
]
"restart_duration": 60,
"restart_warning_msgs": [
    {
        "msg": "<your-warning-message-here>",
        "time": 30
    }
] 
```

The `jar_file_name` is meant for the server jarfile that you would get from Mojang or Fabric if your doing a modded server

The `java_args` are the optional arguments to pass to the java process for optimaization or in the example above, allocating 6gb of memory and nogui so it will only
run in the terminal. However you can adjust these arguments however you like

The `restart_duration` is the time in `seconds` that need to be elapsed before the server will attempt to restart

THE `restart_warning_msgs` are there so you can send a message to the server whenever the `time` is elapsed

### Additional Info
this program expects your server files to be located in a directory named `server`

if your trying to get the warning messages timed down to the second from the `restart_duration` try adding 2-4 extra seconds to the `restart_duration`. 
there are some inconsistencies with the timer.
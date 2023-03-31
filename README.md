# Minecraft Server Bounce

### Description
Basic Rust Program Thats will Start and Manage your minecraft server. It manages a configurable Automatic Restart.

It will also send warning messages to the server based off of user configuration

### Usage 
config/server_bounce_config.json
```json
"jar_file_name": "<path-to-your-jarfile-here>",
"restart_duration": 60,
"restart_warning_msgs": [
    {
        "msg": "<your-warning-message-here>",
        "time_left": 30
    }
] 
```

The `jar_file_name` is meant for the server jarfile that you would get from Mojang or Fabric if your doing a modded server

The `restart_duration` is the time in `seconds` that need to be elapsed before the server will attempt to restart

THE `restart_warning_msgs` are there so you can send a message to the server whenever the `time_left` is elapsed
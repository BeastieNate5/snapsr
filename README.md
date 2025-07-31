# Snapsr

- [About](#about)
- [Installation](#installation)
- [Getting Started](#getting-started)
- [Configuration](#configuration)

## About
**Snapsr** is a lightweight CLI tool that lets you take snapshots of your custom file setups and easily restore them later. Define which files to track in a simple config, save a snap, and Snapsr will archive the current state of those files. When you restore a snap, it restores everything tracked backto its original location, making it perfect for quickly switching between desktop ricing setuos, development environments, or configuration profiles

### Key Features:
- Save and load mutiple namped snaps
- Automatically restores files to their original paths
- Manage multiple setups (perfect for ricing, dev environments, or config testing)

Use Snapsr to version, switch, and experiment with different setups. Without overwriting your current work

## Installation
To build from source you will need [Rust and Cargo](https://www.rust-lang.org/tools/install)
If you do not have that on your system you can install Rust using the following command

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Once you have Rust and Cargo run the following command to install Snapsr

```bash
cargo install --git https://github.com/BeastieNate5/snapsr
```


## Getting Started
To get started you are going to want to create a Snap. A Snap is just snapshot of file(s) you are tracking with Snapsr in your configuration file. You can read about the format of the file [here](#configuration). The following command saves a snap labelled desktop_env

```bash
snapsr -s desktop_env
```

To see all your saved snaps you can run the following command

```bash
snapsr -l
```

If you wish to restore a Snap you made you run the following command

```bash
snapsr -r desktop_env
```

This will restore all files saved in the desktop_env Snap back to their original locations, just as they were when the snap was taken


To see all available commands use `-h`

## Configuration
The configuration file is located at `$HOME/.config/snapsr/config.toml`

```toml
[hooks]
pre_load = ""
post_load = "pkill waybar; hyprctl dispatch exec waybar"

[modules.hypr]
include = ["/home/0x2B/.config/hypr/*"]
description = "Hyprland configuration files"

template waybar.toml
```

The configuration is in toml format. First you add in your modules. Here we put a module called hypr.In the `include` variable you specifiy what files you want to be included in the module. Optionally you can add in a `description`

Snapsr also supports templates. Templating allows you to write your configuration in different files and include them into the main configuration file. You can create your templates at `$HOME/.config/snapsr/templates`. Here we have a template file called `waybar.toml` that we included in our configuration file. It contains the following

```toml
[modules.waybar]
include = ["/home/0x2B/.config/waybar/*"]
```

This then replaces the line `template waybar.toml` in the main configuration file

At the top of our config file we have a optional hooks section. In this section you can add the variables `pre_load` and `post_load`. These will be executed whenever you restore a Snap. `pre_load` gets executed before the files are restored, `post_load` gets executed after the files have been restored

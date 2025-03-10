# Dotulous
Dotulous is an easy to use dotfile manager, working off the concept of "profiles" that you can load and hotswap.

https://github.com/user-attachments/assets/9ed73e21-176d-4547-ab6b-3edd2958ecc8

Dotfile profiles can load/unload files, and run shell commands to reload your system for you. This lets you swap to another dotfile setup with a single command.

## Installation
To build from source, clone the repository and run `cargo install --path .` - Ensure `~/.cargo/bin` is included in your `$PATH`.

## Usage
> [!CAUTION]  
> Profiles can run arbitrary commands under your user, and can load/unload files from anywhere in the system. 
> 
> **ALWAYS** audit profiles you don't trust, especially ones you download from online even if you believe it to be from a trustful source.

Run `dotulous load {profile}` to load a profile onto your system. You can unload it by running `dotulous unload`.

To create a new profile, run `dotulous create {profile}` and modify the profile's directory inside `~/.dotulous`. For much more detailed information, see [the wiki](https://github.com/SamPertWasTaken/Dotulous/wiki/Creating-&-Modifying-Profiles).

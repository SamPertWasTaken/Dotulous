use std::{env, fs, io, path::{Path, PathBuf}, process::exit};

use clap::{Parser, Subcommand};
use profile::DotfileProfile;
use meta::Meta;

mod profile;
mod meta;
mod error;

/// Prints the given formatted string to stderror, prefixed with `"ERROR: "`, and exits with code -1.
/// Output is done using the [`eprintln`] macro.
macro_rules! error_and_exit {
    ($format: expr) => {
        eprint!("ERROR: ");
        eprintln!($format);
        exit(-1);
    };
    ($format: expr, $($arg:tt)*) => {
        eprint!("ERROR: ");
        eprintln!($format, format_args!($($arg)*));
        exit(-1);
    };
}

/// The command-line arguments that can be accepted. These are parsed with [`clap`].
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct CmdlineArgs {
    /// The [`Action`] to run.
    #[command(subcommand)]
    action: Action
}
/// An action for Dotulous to run.
#[derive(Subcommand, Debug)]
enum Action {
    /// Select & Load a new active dotfile configuration. 
    Load {
        /// The dotfile profile name to use.
        profile_name: String
    },

    /// Unloads the current active profile
    Unload {},

    /// Create a new dotfile configuration
    Create {
        /// The dotfile profile name to use.
        profile_name: String
    },

    /// Auto-Fills the files for a dotfile configuration, saving you time manually filling them out
    /// Will only work if the JSON array is already empty!
    AutoFill {
        /// The dotfile profile name to use.
        profile_name: String
    },

    /// Check the current "status" of your loaded dotfiles
    Status {}
}

fn main() {
    // Are we defo in Linux?
    // If your compiling this for some other platform and trust what your doing, comment out this
    // check at your own risk.
    if env::consts::OS != "linux" {
        println!("Dotulous is only supported on Linux.");
        exit(0);
    }

    let home_folder: String = match env::var("HOME") {
        Ok(r) => r,
        Err(e) => { error_and_exit!("Unable to find suitable home folder: {e}"); }
    };
    let home_path: &Path = Path::new(&home_folder);
    let manifest_path: String = format!("{home_folder}/.dotulous/");
    let manifest_location: &Path = Path::new(&manifest_path);
    if !manifest_location.exists() {
        if let Err(e) = fs::create_dir_all(manifest_location) {
            error_and_exit!("Unable to create dotulous folder: {e}");
        }
        let meta: Meta = Meta::new();
        if let Err(e) = meta.save_meta(manifest_location) {
            error_and_exit!("Failed to save meta: {e}");
        }
        println!("NOTE: Created dotulous folder at {manifest_path}");
        println!("NOTE: This is where your dotfile configurations will be!");
    }

    let args = CmdlineArgs::parse();
    match args.action {
        Action::Load { profile_name } => { action_load_profile(manifest_location, home_path, &profile_name) },
        Action::Unload { } => { action_unload_profile(manifest_location, home_path) },
        Action::Create { profile_name } => { action_create_profile(manifest_location, &profile_name); },
        Action::AutoFill { profile_name } => { action_fill_profile(manifest_location, &profile_name); },
        Action::Status { } => {
            let meta: Meta = match Meta::load_meta(manifest_location) {
                Ok(r) => r,
                Err(e) => { error_and_exit!("Could not load current meta: {e}"); },
            };
            let current_profile: Option<DotfileProfile> = meta.current_profile();
            if let Some(profile) = current_profile {
                println!("Currently loaded profile: {}", profile.name);
            } else {
                println!("No currently loaded profile.");
            }
            println!();
            println!("Detected profiles:");

            // Scan for all available profiles 
            let paths = match fs::read_dir(manifest_location) {
                Ok(r) => r,
                Err(e) => { error_and_exit!("Failed to read from directory \"{manifest_location:?}\": {e}"); }
            };
            for path in paths {
                let Ok(path) = path else {
                    continue;
                };
                if !path.path().is_dir() {
                    continue
                }

                let file_os_name = path.file_name();
                let Some(file_name) = file_os_name.to_str() else {
                    continue;
                };
                println!("  {file_name}");
            }
        },
    }
}


// Actions

/// User action that creates a new profile with `profile_name`, where `dotulous_path` is the user's `.dotulous` folder.
/// The folder for the profile is just the sanitized `profile_name`.
///
/// Can internally fail, however will not return a `Result` but rather simply exit since this is intended to only be
/// called by the CLI. Instead, look at [`DotfileProfile::new`] & [`DotfileProfile::save_manifest`].
fn action_create_profile(dotulous_path: &Path, profile_name: &str) {
    // Create the folder
    let folder_name = sanitize_filename::sanitize(profile_name);
    let folder_path: &Path = Path::new(&folder_name);
    let full_path: PathBuf = dotulous_path.join(folder_path);
    if full_path.exists() {
        error_and_exit!("Profile path \"{full_path:?}\" already exists!");
    }
    if let Err(e) = fs::create_dir_all(&full_path) {
        error_and_exit!("Unable to create folder \"{full_path:?}\": {e}");
    }

    // Create the manifest inside of it
    let manifest: DotfileProfile = DotfileProfile::new(profile_name, &full_path);
    if let Err(e) = manifest.save_manifest() {
        error_and_exit!("Failed to save profile manifest for \"{profile_name}\": {e}");
    }

    println!("Created new profile at: {}", full_path.to_str().unwrap());
}

/// User action for unloading the currently loaded profile from the system, where `dotulous_path`
/// is the user's `.dotulous` folder.
///
/// This function will also update the Meta file.
///
/// Can internally fail, however will not return a `Result` but rather simply exit since this is intended to only be
/// called by the CLI. Instead, look at [`Meta::current_profile`] & [`DotfileProfile::unload_profile_from_system`].
fn action_unload_profile(dotulous_path: &Path, home_path: &Path) {
    println!("Using home folder: {home_path:?}");

    let mut meta: Meta = match Meta::load_meta(dotulous_path) {
        Ok(r) => r,
        Err(e) => { error_and_exit!("Could not load current meta: {e}"); },
    };
    let Some(profile) = meta.current_profile() else {
        error_and_exit!("No currently loaded profile was found. Nothing to do.");
    };

    profile.unload_profile_from_system(home_path);

    meta.empty_current_profile();
    if let Err(e) = meta.save_meta(dotulous_path) {
        error_and_exit!("Failed to save meta: {e}");
    }
}

/// User action for loading a profile to the system, after finding the profile from `profile_name`, 
/// where `dotulous_path` is the user's `.dotulous` folder.
/// If the profile is not trusted, it will confirm with the user to trust it or not.
///
/// This function will also update the Meta file.
///
/// Can internally fail, however will not return a `Result` but rather simply exit since this is intended to only be
/// called by the CLI. Instead, look at [`DotfileProfile::load_profile_to_system`].
fn action_load_profile(dotulous_path: &Path, home_path: &Path, profile_name: &str) {
    println!("Using home folder: {home_path:?}");

    let mut meta: Meta = match Meta::load_meta(dotulous_path) {
        Ok(r) => r,
        Err(e) => { error_and_exit!("Could not load current meta: {e}"); },
    };
    if let Some(current_profile) = meta.current_profile() {
        current_profile.unload_profile_from_system(home_path);
        println!();
    }

    let profile: DotfileProfile = match DotfileProfile::find_profile(dotulous_path, profile_name) {
        Ok(r) => r,
        Err(e) => { error_and_exit!("Failed to load profile \"{profile_name}\": {e}"); },
    };

    if !meta.is_trusted(&profile.repo_path) {
        println!("WARNING: Profile has not been marked as trusted.");
        println!("Please verify the contents of the profile! Remember that profiles can run ANY ARBITRARY COMMANDS on your system, and can install ANY ARBITRARY FILES.");
        println!("You're essentially going to be running random code off of the internet, so be careful!");
        println!();
        println!("Do you trust this profile? (y/N)");
        let mut input: String = String::new();
        if let Err(e) = io::stdin().read_line(&mut input) {
            error_and_exit!("Failed to read from stdin: {e}");
        }
        if input.trim().to_lowercase() != "y" {
            println!("Quitting...");
            exit(-1);
        }

        meta.trust_profile(profile.repo_path.clone());
        println!("Trusting profile {}", profile.name);
    }
    profile.load_profile_to_system(home_path);

    meta.set_current_profile(&profile);
    if let Err(e) = meta.save_meta(dotulous_path) {
        error_and_exit!("Failed to save meta for \"{profile_name}\": {e}");
    }
}

/// User action for auto-filling a profile's `files` array to help them, finding the profile with
/// the given `profile_name`, and where `dotulous_path` is the user's `.dotulous` folder.
///
/// Can internally fail, however will not return a `Result` but rather simply exit since this is intended to only be
/// called by the CLI. Instead, look at [`DotfileProfile::fill_files`].
fn action_fill_profile(dotulous_path: &Path, profile_name: &str) {
    let mut profile: DotfileProfile = match DotfileProfile::find_profile(dotulous_path, profile_name) {
        Ok(r) => r,
        Err(e) => { error_and_exit!("Failed to load profile \"{profile_name}\": {e}"); },
    };
    if let Err(e) = profile.fill_files() {
        error_and_exit!("Failed to fill profile files for \"{profile_name}\": {e}");
    }
}

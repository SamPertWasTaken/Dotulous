use std::{env, fs, io, path::{Path, PathBuf}, process::exit};

use clap::{Parser, Subcommand};
use profile::DotfileProfile;
use meta::Meta;

mod profile;
mod meta;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct CmdlineArgs {
    #[command(subcommand)]
    action: Action
}
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
    if env::consts::OS != "linux" {
        println!("Dotulous only supports Linux!");
        exit(-1);
    }

    let home_folder: String = match env::var("HOME") {
        Ok(r) => r,
        Err(e) => panic!("Unable to find suitable home folder; {}", e)
    };
    let manifest_path: String = format!("{}/.dotulous/", home_folder);
    let manifest_location: &Path = Path::new(&manifest_path);
    if !manifest_location.exists() {
        fs::create_dir_all(manifest_location).expect("Unable to create dotulous folder.");
        let meta: Meta = Meta::new();
        meta.save_meta(manifest_location);
        println!("NOTE: Created dotulous folder at {manifest_path}");
        println!("NOTE: This is where your dotfile configurations will be!");
    }

    let args = CmdlineArgs::parse();
    match args.action {
        Action::Load { profile_name } => { action_load_profile(manifest_location, &profile_name) },
        Action::Unload { } => { action_unload_profile(manifest_location) },
        Action::Create { profile_name } => { action_create_profile(manifest_location, &profile_name); },
        Action::AutoFill { profile_name } => { action_fill_profile(manifest_location, &profile_name); },
        Action::Status { } => {
            let meta: Meta = Meta::load_meta(manifest_location);
            let current_profile: Option<String> = meta.current_profile_name();
            if current_profile.is_some() {
                println!("Currently loaded profile: {}", current_profile.unwrap());
            } else {
                println!("No currently loaded profile.");
            }
            println!();
            println!("Detected profiles:");

            // Scan for all available profiles 
            let paths = fs::read_dir(manifest_location).expect("Unable to read from directory.");
            for path in paths {
                if !path.as_ref().unwrap().path().is_dir() {
                    continue
                }
                println!("  {}", path.unwrap().path().file_name().unwrap().to_str().unwrap());
            }
        },
    }
}


// Actions
fn action_create_profile(manifest_location: &Path, profile_name: &str) {
    // Create the folder
    let folder_name = sanitize_filename::sanitize(profile_name);
    let folder_path: &Path = Path::new(&folder_name);
    let full_path: PathBuf = manifest_location.join(folder_path);
    if full_path.exists() {
        panic!("Profile path already exists.");
    }
    fs::create_dir_all(&full_path).expect("Unable to create profile folder.");

    // Create the manifest inside of it
    let manifest: DotfileProfile = DotfileProfile::new(profile_name, &full_path);
    manifest.save_manifest();

    println!("Created new profile at: {}", full_path.to_str().unwrap());
}

fn action_unload_profile(manifest_location: &Path) {
    let mut meta: Meta = Meta::load_meta(manifest_location);
    let current_profile: Option<String> = meta.current_profile_name();
    if current_profile.is_none() {
        panic!("No currently loaded profile.");
    }

    let manifest = DotfileProfile::find_profile(manifest_location, &current_profile.unwrap());
    manifest.unload_profile_from_system();

    meta.empty_current_profile();
    meta.save_meta(manifest_location);
}

fn action_load_profile(manifest_location: &Path, profile_name: &str) {
    let mut meta: Meta = Meta::load_meta(manifest_location);
    let current_profile: Option<String> = meta.current_profile_name();
    if current_profile.is_some() {
        let manifest = DotfileProfile::find_profile(manifest_location, &current_profile.unwrap());
        manifest.unload_profile_from_system();
        println!();
    }

    let profile: DotfileProfile = DotfileProfile::find_profile(manifest_location, profile_name);
    if !meta.is_trusted(&profile.repo_path) {
        println!("WARNING: Profile has not been marked as trusted.");
        println!("Please verify the contents of the profile! Remember that profiles can run ANY ARBITRARY COMMANDS on your system, and can install ANY ARBITRARY FILES.");
        println!("You're essentially going to be running random code off of the internet, so be careful!");
        println!();
        println!("Do you trust this profile? (y/N)");
        let mut input: String = String::new();
        io::stdin().read_line(&mut input).expect("Could not read input.");
        if input.trim().to_lowercase() != "y" {
            println!("Quitting...");
            exit(0);
        }

        meta.trust_profile(profile.repo_path.to_path_buf());
        println!("Trusting profile {}", profile.name);
    }
    profile.load_profile_to_system();

    meta.set_current_profile(&profile);
    meta.save_meta(manifest_location);
}

fn action_fill_profile(manifest_location: &Path, profile_name: &str) {
    let mut profile: DotfileProfile = DotfileProfile::find_profile(manifest_location, profile_name);
    profile.fill_files();
}

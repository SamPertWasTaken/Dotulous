use std::{collections::HashMap, env, fs, os::unix::fs::symlink, path::{Path, PathBuf}, process::Command};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct DotfileProfile {
    pub name: String,
    pub manifest_path: PathBuf,
    pub repo_path: PathBuf,
    files: HashMap<PathBuf, PathBuf>,
    pre_commands: Vec<String>,
    post_commands: Vec<String>,
    removal_commands: Vec<String>
}
impl DotfileProfile {
    pub fn new(name: &str, path: &Path) -> Self {
        Self {
            name: name.to_string(),
            manifest_path: path.join(Path::new("manifest.json")),
            repo_path: path.to_path_buf(),
            files: HashMap::new(),
            pre_commands: Vec::new(),
            post_commands: Vec::new(),
            removal_commands: Vec::new()
        }
    }
    pub fn find_profile(manifest_location: &Path, profile_name: &str) -> DotfileProfile {
        let folder_name = sanitize_filename::sanitize(profile_name);
        let folder_path: &Path = Path::new(&folder_name);
        let full_path: PathBuf = manifest_location.join(folder_path);
        if !full_path.exists() {
            panic!("Cannot find profile {profile_name}.");
        }

        // Load the manifest 
        DotfileProfile::from_manifest(&full_path)
    }
    pub fn from_manifest(profile_path: &Path) -> Self {
        let manifest_path: PathBuf = profile_path.join(Path::new("manifest.json"));
        if !manifest_path.exists() {
            panic!("Can't find manifest in profile.");
        }

        let contents: String = fs::read_to_string(&manifest_path).expect("Can't read manifest file.");
        let mut deserialized: DotfileProfile = serde_json::from_str(&contents).expect("Unable to deserialize manifest.");
        // Double-check the manifest/repo paths are correct, as these can be altered by the user 
        deserialized.manifest_path = manifest_path;
        deserialized.repo_path = profile_path.to_path_buf();

        deserialized
    }

    pub fn save_manifest(&self) {
        let serialized = serde_json::to_string_pretty(self).expect("Unable to serialize manifest to JSON.");
        fs::write(&self.manifest_path, serialized).expect("Unable to save manifest to repository folder.");
    }

    pub fn load_profile_to_system(&self) {
        println!("Loading profile: {}", self.name);
        let home_folder: String = match env::var("HOME") {
            Ok(r) => r,
            Err(e) => panic!("Unable to find suitable home folder; {}", e)
        };
        let home_path: &Path = Path::new(&home_folder);
        println!("Using home folder: {:?}", home_path);

        if !self.pre_commands.is_empty() {
            println!();
            println!("Running pre-commands.");
            for command in &self.pre_commands {
                println!("  {}", command);
                Command::new("sh")
                    .current_dir(home_path)
                    .arg("-c")
                    .arg(command)
                    .output()
                    .expect("Failed to execute command.");
            }
        }

        println!();
        for file in &self.files {
            let source: PathBuf = self.repo_path.join(file.0);
            let destination: PathBuf = home_path.join(file.1);
            println!("  {:?} => {:?}", source, destination);
            if destination.exists() {
                println!("  WARNING: Destination {destination:?} already exists! Skipping!");
                continue;
            }
            symlink(source, destination).expect("Unable to symlink paths.");
        }

        if !self.post_commands.is_empty() {
            println!();
            println!("Running post-commands.");
            for command in &self.post_commands {
                println!("  {}", command);
                Command::new("sh")
                    .current_dir(home_path)
                    .arg("-c")
                    .arg(command)
                    .output()
                    .expect("Failed to execute command.");
            }
        }

    }

    pub fn unload_profile_from_system(&self) {
        println!("Unloading profile: {}", self.name);
        let home_folder: String = match env::var("HOME") {
            Ok(r) => r,
            Err(e) => panic!("Unable to find suitable home folder; {}", e)
        };
        let home_path: &Path = Path::new(&home_folder);
        println!("Using home folder: {:?}", home_path);

        for file in &self.files {
            let destination: PathBuf = home_path.join(file.1);
            println!("  Removing {:?}", destination);
            if !destination.exists() {
                println!("  WARNING: Destination {destination:?} doesn't exist! Skipping!");
                continue;
            }

            if destination.is_dir() {
                if destination == Path::new("/") {
                    panic!("Tried to remove root!");
                }
                if destination == home_path {
                    panic!("Tried to remove home path!");
                }
                fs::remove_dir_all(destination).expect("Unable to delete destination.");
            } else {
                fs::remove_file(destination).expect("Unable to delete destination.");
            }
        }

        if !self.removal_commands.is_empty() {
            println!();
            println!("Running removal commands.");
            for command in &self.removal_commands {
                println!("  {}", command);
                Command::new("sh")
                    .current_dir(home_path)
                    .arg("-c")
                    .arg(command)
                    .output()
                    .expect("Failed to execute command.");
            }
        }
    }

    pub fn fill_files(&mut self) {
        if !self.files.is_empty() {
            panic!("Profile files array is not empty!");
        }

        println!("Filling files for profile: {}", self.name);
        let paths = fs::read_dir(&self.repo_path).expect("Unable to read from profile directory.");
        for path in paths {
            let path = path.unwrap().path().strip_prefix(&self.repo_path).expect("Failed").to_path_buf();
            println!("  {:?}", path);
            self.files.insert(path.clone(), path.clone());
        }
        println!();
        println!("Done! Make sure to go through them manually to make sure!");

        self.save_manifest();
    }
}

use std::{collections::HashMap, fs, io, os::unix::fs::symlink, path::{Path, PathBuf}, process::{Command, Output}};

use serde::{Deserialize, Serialize};

use crate::error::DotulousError;

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
    pub fn find_profile(manifest_location: &Path, profile_name: &str) -> Result<DotfileProfile, DotulousError> {
        let folder_name = sanitize_filename::sanitize(profile_name);
        let folder_path: &Path = Path::new(&folder_name);
        let full_path: PathBuf = manifest_location.join(folder_path);
        if !full_path.exists() {
            return Err(DotulousError::ProfileNotFound)
        }

        // Load the manifest 
        DotfileProfile::from_manifest(&full_path)
    }
    pub fn from_manifest(profile_path: &Path) -> Result<Self, DotulousError> {
        let manifest_path: PathBuf = profile_path.join(Path::new("manifest.json"));
        if !manifest_path.exists() {
            return Err(DotulousError::NoManifestInProfile)
        }

        let Ok(contents) = fs::read_to_string(&manifest_path) else { return Err(DotulousError::FailedReadManifest) };
        let Ok(mut deserialized) = serde_json::from_str::<DotfileProfile>(&contents) else { return Err(DotulousError::FailedDeserializeManifest) };
        // Double-check the manifest/repo paths are correct, as these can be altered by the user 
        deserialized.manifest_path = manifest_path;
        deserialized.repo_path = profile_path.to_path_buf();

        Ok(deserialized)
    }

    pub fn save_manifest(&self) -> Result<(), DotulousError> {
        let Ok(serialized) = serde_json::to_string_pretty(self) else { return Err(DotulousError::FailedSerializeManifest) };
        if fs::write(&self.manifest_path, serialized).is_err() { return Err(DotulousError::FailedSaveManifest) }
        Ok(())
    }

    pub fn fill_files(&mut self) -> Result<(), DotulousError> {
        if !self.files.is_empty() {
            return Err(DotulousError::FillManifestArrayNotEmpty)
        }

        println!("Filling files for profile: {}", self.name);
        let Ok(paths) = fs::read_dir(&self.repo_path) else { return Err(DotulousError::FailedReadProfileDirectory) };
        for path in paths {
            let Ok(path) = path else { return Err(DotulousError::FailedReadProfileDirectory) };
            let actual_path = path.path();
            let Ok(stripped_path) = actual_path.strip_prefix(&self.repo_path) else { return Err(DotulousError::FailedReadProfileDirectory) };
            let final_path = stripped_path.to_path_buf();

            println!("  {:?}", final_path);
            self.files.insert(final_path.clone(), final_path.clone());
        }
        println!();
        println!("Done! Make sure to go through them manually to make sure!");

        self.save_manifest()
    }

    pub fn load_profile_to_system(&self, home_path: &Path) {
        println!("Loading profile: {}", self.name);
        if !self.pre_commands.is_empty() {
            println!();
            println!("Running pre-commands.");
            for command in &self.pre_commands {
                println!("  {}", command);
                let command: Result<Output, io::Error> = Command::new("sh")
                    .current_dir(home_path)
                    .arg("-c")
                    .arg(command)
                    .output();
                if command.is_err() {
                    let unwrapped = command.unwrap();
                    println!("  ERROR: Command failed to run (exit code {}): {}", unwrapped.status, String::from_utf8(unwrapped.stderr).unwrap())
                }
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
            if symlink(&source, &destination).is_err() {
                println!("  ERROR: Failed to symlink path: {source:?} -> {destination:?}")
            }
        }

        if !self.post_commands.is_empty() {
            println!();
            println!("Running post-commands.");
            for command in &self.post_commands {
                println!("  {}", command);
                let command: Result<Output, io::Error> = Command::new("sh")
                    .current_dir(home_path)
                    .arg("-c")
                    .arg(command)
                    .output();
                if command.is_err() {
                    let unwrapped = command.unwrap();
                    println!("  ERROR: Command failed to run (exit code {}): {}", unwrapped.status, String::from_utf8(unwrapped.stderr).unwrap())
                }
            }
        }

    }
    pub fn unload_profile_from_system(&self, home_path: &Path) {
        println!("Unloading profile: {}", self.name);
        for file in &self.files {
            let destination: PathBuf = home_path.join(file.1);
            println!("  Removing {:?}", destination);
            if !destination.exists() {
                println!("  WARNING: Destination {destination:?} doesn't exist! Skipping!");
                continue;
            }

            if destination.is_dir() {
                // very basic protection
                if destination == Path::new("/") {
                    panic!("Tried to remove root!");
                }
                if destination == home_path {
                    panic!("Tried to remove home path!");
                }
                if fs::remove_dir_all(&destination).is_err() {
                    println!("  Error: Failed to delete destination {destination:?}.")
                }
            } else if fs::remove_file(&destination).is_err() {
                println!("  Error: Failed to delete destination {destination:?}.")
            }
        }

        if !self.removal_commands.is_empty() {
            println!();
            println!("Running removal commands.");
            for command in &self.removal_commands {
                println!("  {}", command);
                let command: Result<Output, io::Error> = Command::new("sh")
                    .current_dir(home_path)
                    .arg("-c")
                    .arg(command)
                    .output();
                if command.is_err() {
                    let unwrapped = command.unwrap();
                    println!("  ERROR: Command failed to run (exit code {}): {}", unwrapped.status, String::from_utf8(unwrapped.stderr).unwrap())
                }
            }
        }
    }
}

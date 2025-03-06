use std::{collections::HashMap, fs, io, os::unix::fs::symlink, path::{Path, PathBuf}, process::{Command, Output}};

use serde::{Deserialize, Serialize};

use crate::error::DotulousError;

/// A dotfile profile, that the user can load and modify. This should be loaded or at least
/// representitive of the profile's `manifest.json`
/// The profile's directory should be within `repo_path`, with a `manifest.json` file detailing the
/// profile inside of the directory.
///
/// ### Fetching a Profile
///
/// To fetch an already-existing profile, you can use;
/// - [`DotfileProfile::find_profile`] will search for your profile, with `dotulous_path` being the `.dotulous` folder.
/// - *or*, if you already have the location of the profile's directly, you can use [`DotfileProfile::from_manifest`] to load it in directly.
///
/// ### Loading/Unloading Profiles
///
/// To load the profile to the system, call [`DotfileProfile::load_profile_to_system`]. **Take care
/// two profiles are not loaded at once, there's no checks in this function for that!** 
///
/// To unload the profile, deleting all symlinks it created, call [`DotfileProfile::unload_profile_from_system`]. Once again,
/// this function **will not check if it was already loaded**, so if called on an already un-loaded
/// profile, it will still delete any files listed in the manifest.
///
/// ### Saving the Profile
///
/// After modifying the profile's data, you should call [`DotfileProfile::save_manifest`] to save
/// the changed profile `manifest.json` to disk.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DotfileProfile {
    /// The user-friendly name of the profile.
    pub name: String,
    /// The *absolute* path to the profile's `manifest.json`.
    pub manifest_path: PathBuf,
    /// The *absolute* path to the profile's folder itself.
    pub repo_path: PathBuf,
    /// The list of files that should be loaded with the profile. Key is the path relative to the
    /// profile's directory, and the value is where it should be symlinked to in the system upon
    /// loading - or in the case of unloading, what symlink will be deleted.
    files: HashMap<PathBuf, PathBuf>,
    /// A list of commands to run on loading *before* the files are symlinked to the system.
    pre_commands: Vec<String>,
    /// A list of commands to run on loading *after* the files are symlinked to the system.
    post_commands: Vec<String>,
    /// A list of commands to run on unloading, running *after* the files are removed from the system.
    removal_commands: Vec<String>
}
impl DotfileProfile {
    /// Creates a new `DotfileProfile`.
    /// `path` should be an *absolute* path t the profile's folder.
    ///
    /// Note that this function does **not** create the profile on disk. You have to manually make
    /// the path yourself, along with calling [`DotfileProfile::save_manifest`] to create the
    /// `manifest.json`
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

    /// Find a given profile on-disk with the user-friendly `profile_name`, with `dotulous_path`
    /// being the user's `.dotulous` folder.
    /// If the profile is not found, it will return [`Err`] with [`DotulousError::ProfileNotFound`].
    ///
    /// Internally this simply finds if the given profile's path exists using a santized `profile_name`,
    /// calling [`DotfileProfile::from_manifest`] when found.
    pub fn find_profile(dotulous_path: &Path, profile_name: &str) -> Result<DotfileProfile, DotulousError> {
        let folder_name = sanitize_filename::sanitize(profile_name);
        let folder_path: &Path = Path::new(&folder_name);
        let full_path: PathBuf = dotulous_path.join(folder_path);
        if !full_path.exists() {
            return Err(DotulousError::ProfileNotFound)
        }

        // Load the manifest 
        DotfileProfile::from_manifest(&full_path)
    }

    /// Read a profile from disk when you have a known `profile_path` with a `manifest.json` inside
    /// of it.
    ///
    /// This reads the `manifest.json` directly, and deserializes it.
    pub fn from_manifest(profile_path: &Path) -> Result<DotfileProfile, DotulousError> {
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

    /// Save the current profile data to the `manifest.json` of this profile.
    /// This uses the `manifest_path` property to locate the `manifest.json` location.
    ///
    /// The returned [`Result`] does not return anything on success, meaning you should only check
    /// for [`Err`] variants. 
    pub fn save_manifest(&self) -> Result<(), DotulousError> {
        let Ok(serialized) = serde_json::to_string_pretty(self) else { return Err(DotulousError::FailedSerializeManifest) };
        if fs::write(&self.manifest_path, serialized).is_err() { return Err(DotulousError::FailedSaveManifest) }
        Ok(())
    }

    /// Scans the profile's `repo_path` and automatially adds all found files to the manifest's
    /// `files` property, before saving the manifest to disk.
    ///
    /// **Note:** This function prints to stdout, as it is normally called by the user in the CLI.
    ///
    /// This function should only be called if the `files` property is already empty. If not, 
    /// it will return an [`Err`] with [`DotulousError::FillManifestArrayNotEmpty`].
    ///
    /// The returned [`Result`] does not return anything on success, meaning you should only check
    /// for [`Err`] variants. 
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

            println!("  {final_path:?}");
            self.files.insert(final_path.clone(), final_path.clone());
        }
        println!();
        println!("Done! Make sure to go through them manually to make sure!");

        self.save_manifest()
    }

    /// Loads the profile to the system, in three stages;
    /// - It runs any `pre_commands` that are specified. These are ran in a new `sh` shell, with the
    ///   working directory being the user's home folder.
    /// - It will then symlink all the files from the profile's directory to the system, according
    ///   to the `files` property.
    /// - Finally, it will run any `post_commands` in the same way of pre-commands.
    ///
    /// It is **highly advised** to then update the meta via [`Meta::set_current_profile`] & [`Meta::save_meta`].
    /// Otherwise, dotulous will not know what profile is currently loaded.
    ///
    /// **WARNING**: NEVER LOAD TWO PROFILES AT ONCE. The Meta object can only handle one, and
    /// loading two will cause the first profile loaded to be invisible to dotulous, not letting
    /// the user un-load it.
    ///
    /// **Note:** This function prints to stdout, as it is normally called by the user in the CLI.
    /// Upon any errors, the function will simply print to stdout and continue.
    pub fn load_profile_to_system(&self, home_path: &Path) {
        println!("Loading profile: {}", self.name);
        if !self.pre_commands.is_empty() {
            println!();
            println!("Running pre-commands.");
            for command in &self.pre_commands {
                println!("  {command}");
                let command: Result<Output, io::Error> = Command::new("sh")
                    .current_dir(home_path)
                    .arg("-c")
                    .arg(command)
                    .output();
                if command.is_err() {
                    let unwrapped = command.unwrap();
                    println!("  ERROR: Command failed to run (exit code {}): {}", unwrapped.status, String::from_utf8(unwrapped.stderr).unwrap());
                }
            }
        }

        println!();
        for file in &self.files {
            let source: PathBuf = self.repo_path.join(file.0);
            let destination: PathBuf = home_path.join(file.1);
            println!("  {source:?} => {destination:?}");
            if destination.exists() {
                println!("  WARNING: Destination {destination:?} already exists! Skipping!");
                continue;
            }
            if let Err(e) = symlink(&source, &destination) {
                println!("  ERROR: Failed to symlink {source:?} -> {destination:?}: {e}");
            }
        }

        if !self.post_commands.is_empty() {
            println!();
            println!("Running post-commands.");
            for command in &self.post_commands {
                println!("  {command}");
                let command: Result<Output, io::Error> = Command::new("sh")
                    .current_dir(home_path)
                    .arg("-c")
                    .arg(command)
                    .output();
                if command.is_err() {
                    let unwrapped = command.unwrap();
                    println!("  ERROR: Command failed to run (exit code {}): {}", unwrapped.status, String::from_utf8(unwrapped.stderr).unwrap());
                }
            }
        }

    }

    /// Un-loads the profile from system, in two stages;
    /// - It will destroy any files inside the `files` property, removing any symlinks made.
    /// - It will then run any `removal_commands` that are specified. These are ran in a new `sh` shell, with the
    ///   working directory being the user's home folder.
    ///
    /// It is **highly advised** to then update the meta via [`Meta::empty_current_profile`] & [`Meta::save_meta`].
    /// Otherwise, dotulous will not know what profile is currently loaded.
    ///
    /// **WARNING**: NEVER UNLOAD A PROFILE THAT IS NOT ALREADY LOADED. This will blindly try to
    /// delete the files anyway, as the Meta is what's responsible for keeping track of what
    /// profile is loaded.
    ///
    /// **Note:** This function prints to stdout, as it is normally called by the user in the CLI.
    /// Upon any errors, the function will simply print to stdout and continue.
    pub fn unload_profile_from_system(&self, home_path: &Path) {
        println!("Unloading profile: {}", self.name);
        for file in &self.files {
            let destination: PathBuf = home_path.join(file.1);
            println!("  Removing {destination:?}");
            if !destination.exists() {
                println!("  WARNING: Destination {destination:?} doesn't exist! Skipping!");
                continue;
            }

            if destination.is_dir() {
                // very basic protection
                assert!(destination != Path::new("/"), "Tried to remove root!");
                assert!(destination != home_path, "Tried to remove home path!");
                if fs::remove_dir_all(&destination).is_err() {
                    println!("  Error: Failed to delete destination {destination:?}.");
                }
            } else if fs::remove_file(&destination).is_err() {
                println!("  Error: Failed to delete destination {destination:?}.");
            }
        }

        if !self.removal_commands.is_empty() {
            println!();
            println!("Running removal commands.");
            for command in &self.removal_commands {
                println!("  {command}");
                let command: Result<Output, io::Error> = Command::new("sh")
                    .current_dir(home_path)
                    .arg("-c")
                    .arg(command)
                    .output();
                if command.is_err() {
                    let unwrapped = command.unwrap();
                    println!("  ERROR: Command failed to run (exit code {}): {}", unwrapped.status, String::from_utf8(unwrapped.stderr).unwrap());
                }
            }
        }
    }
}

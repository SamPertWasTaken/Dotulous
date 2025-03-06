use std::{fs, path::{Path, PathBuf}};

use serde::{Deserialize, Serialize};

use crate::{error::DotulousError, profile::DotfileProfile};

/// The meta file is dotulous's main way of keeping track of what profile is loaded, where it is,
/// and what other profiles it has already trusted.
/// This file should be stored in the user's `.dotulous` folder, as `meta.json`.
///
/// **This file should never be modified by a normal user.**
///
/// Loading the meta should be done with [`Meta::load_meta`], providing the `.dotulous` path to it.
///
/// ### Currently Loaded Profile 
/// To update the currently loaded profile, use 
/// - [`Meta::set_current_profile`]
/// - [`Meta::empty_current_profile`]
/// 
/// To find and read the currently loaded profile use [`Meta::current_profile`]. This will return
/// the currently loaded profile, *at the time of loading*. 
///
/// ### Trusted Profiles 
/// To trust a profile you can call [`Meta::trust_profile`] - **Only do this with the confirmation
/// of the user!**.
///
/// To check if a given profile's path is trusted, use [`Meta::is_trusted`]
#[derive(Serialize, Deserialize, Debug)]
pub struct Meta {
    /// Stub field, present in the serialized JSON to warn the user to not touch this file.
    do_not_touch_this_file: String,
    /// The currently 
    current_profile: Option<DotfileProfile>,
    /// The currently loaded profile's path, or [`None`] if no profile is loaded.
    profile_path: Option<PathBuf>,
    /// A list of trusted profile paths.
    trusted_profiles: Vec<PathBuf>
}
impl Meta {
    /// Creates a new Meta object, with empty values.
    ///
    /// Note that this function does **not** create the meta file on disk. You have to manually make
    /// the file yourself, and call [`Meta::save_meta`].
    pub fn new() -> Self {
        Self {
            do_not_touch_this_file: "".to_string(),
            current_profile: None,
            profile_path: None,
            trusted_profiles: Vec::new()
        }
    }

    /// Save the current meta data to disk, using `meta.json` inside of the given `dotulous_path`.
    ///
    /// The returned [`Result`] does not return anything on success, meaning you should only check
    /// for [`Err`] variants. 
    pub fn save_meta(&self, dotulous_path: &Path) -> Result<(), DotulousError> {
        let path: PathBuf = dotulous_path.join(Path::new("meta.json"));
        let Ok(serialized) = serde_json::to_string_pretty(self) else {
            return Err(DotulousError::FailedSerializeMeta)
        };
        if fs::write(path, serialized).is_err() {
            return Err(DotulousError::FailedSaveMeta)
        } 
        Ok(())
    }

    /// Load the current meta file from disk, using `meta.json` inside of the given `dotulous_path`.
    /// If the meta file cannot be found, [`Err`] with [`DotulousError::MetaNotFound`] is returned.
    pub fn load_meta(dotulous_path: &Path) -> Result<Meta, DotulousError> {
        let path: PathBuf = dotulous_path.join(Path::new("meta.json"));
        if !path.exists() {
            return Err(DotulousError::MetaNotFound)
        }

        let contents: String = fs::read_to_string(path).expect("Can't read meta file.");
        match serde_json::from_str::<Self>(&contents) {
            Ok(r) => Ok(r),
            Err(_) => Err(DotulousError::FailedDeserializeMeta),
        }
    }

    /// Set the currently loaded profile inside the manifest, changing `current_profile` and
    /// `profile_path`.
    pub fn set_current_profile(&mut self, profile: &DotfileProfile) {
        self.current_profile = Some(profile.clone());
    }
    /// Clear's the current profile, making `current_profile` and `profile_path` to be [`None`].
    pub fn empty_current_profile(&mut self) {
        self.current_profile = None;
        self.profile_path = None;
    }
    /// Returns the current profile, or [`None`] if no profile is currently loaded.
    pub fn current_profile(&self) -> Option<DotfileProfile> {
        self.current_profile.clone()
    }

    /// Trusts the profile path provided, adding it to `trusted_profiles`.
    pub fn trust_profile(&mut self, path: PathBuf) {
        self.trusted_profiles.push(path);
    }
    /// Checks if the profile path provided is trusted and inside `trusted_profiles`.
    pub fn is_trusted(&self, path: &Path) -> bool {
        self.trusted_profiles.contains(&path.to_path_buf())
    }
}

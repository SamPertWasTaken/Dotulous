use std::{fs, path::{Path, PathBuf}};

use serde::{Deserialize, Serialize};

use crate::{error::DotulousError, profile::DotfileProfile};

#[derive(Serialize, Deserialize, Debug)]
pub struct Meta {
    do_not_touch_this_file: String,
    current_profile: Option<String>,
    profile_path: Option<PathBuf>,
    trusted_profiles: Vec<PathBuf>
}
impl Meta {
    pub fn new() -> Self {
        Self {
            do_not_touch_this_file: "".to_string(),
            current_profile: None,
            profile_path: None,
            trusted_profiles: Vec::new()
        }
    }

    pub fn save_meta(&self, manifest_location: &Path) -> Result<(), DotulousError> {
        let path: PathBuf = manifest_location.join(Path::new("meta.json"));
        let Ok(serialized) = serde_json::to_string_pretty(self) else {
            return Err(DotulousError::FailedSerializeMeta)
        };
        if fs::write(path, serialized).is_err() {
            return Err(DotulousError::FailedSaveMeta)
        } 
        Ok(())
    }
    pub fn load_meta(manifest_location: &Path) -> Result<Self, DotulousError> {
        let path: PathBuf = manifest_location.join(Path::new("meta.json"));
        if !path.exists() {
            return Err(DotulousError::MetaNotFound)
        }

        let contents: String = fs::read_to_string(path).expect("Can't read meta file.");
        match serde_json::from_str::<Self>(&contents) {
            Ok(r) => Ok(r),
            Err(_) => Err(DotulousError::FailedDeserializeMeta),
        }
    }

    pub fn set_current_profile(&mut self, profile: &DotfileProfile) {
        self.current_profile = Some(profile.name.clone());
        self.profile_path = Some(profile.repo_path.clone());
    }
    pub fn empty_current_profile(&mut self) {
        self.current_profile = None;
        self.profile_path = None;
    }
    pub fn current_profile_name(&self) -> Option<String> {
        self.current_profile.clone()
    }

    pub fn trust_profile(&mut self, path: PathBuf) {
        self.trusted_profiles.push(path);
    }
    pub fn is_trusted(&self, path: &Path) -> bool {
        self.trusted_profiles.contains(&path.to_path_buf())
    }
}

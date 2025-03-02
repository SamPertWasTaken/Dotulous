use std::{collections::HashMap, env, fs, os::unix::fs::symlink, path::{Path, PathBuf}};

use serde::{Deserialize, Serialize};

use crate::manifest::DotfileManifest;

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

    pub fn save_meta(&self, manifest_location: &Path) {
        let path: PathBuf = manifest_location.join(Path::new("meta.json"));
        let serialized = serde_json::to_string_pretty(self).expect("Unable to serialize meta file to JSON.");
        fs::write(path, serialized).expect("Unable to save meta file.");
    }
    pub fn load_meta(manifest_location: &Path) -> Self {
        let path: PathBuf = manifest_location.join(Path::new("meta.json"));
        if !path.exists() {
            panic!("Can't find meta in profile.");
        }

        let contents: String = fs::read_to_string(path).expect("Can't read meta file.");
        serde_json::from_str(&contents).expect("Unable to deserialize meta.")
    }

    pub fn current_profile_name(&self) -> Option<String> {
        self.current_profile.clone()
    }
    pub fn set_current_profile(&mut self, profile: &DotfileManifest) {
        self.current_profile = Some(profile.name.clone());
        self.profile_path = Some(profile.repo_path.clone());
    }
    pub fn empty_current_profile(&mut self) {
        self.current_profile = None;
        self.profile_path = None;
    }

    pub fn trust_profile(&mut self, path: PathBuf) {
        self.trusted_profiles.push(path);
    }
    pub fn is_trusted(&self, path: &Path) -> bool {
        self.trusted_profiles.contains(&path.to_path_buf())
    }
}

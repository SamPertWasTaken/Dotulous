use std::fmt::Display;

pub enum DotulousError {
    // Profiles
    ProfileNotFound,
    NoManifestInProfile,
    FailedReadManifest,
    FailedDeserializeManifest,
    FailedSerializeManifest,
    FailedSaveManifest,
    FillManifestArrayNotEmpty,
    FailedReadProfileDirectory,

    // Meta
    MetaNotFound,
    FailedSerializeMeta,
    FailedDeserializeMeta,
    FailedSaveMeta,
}
impl DotulousError {
    fn as_str(&self) -> &str {
        match self {
            DotulousError::ProfileNotFound => "Profile was not found.",
            DotulousError::NoManifestInProfile => "No manifest was found inside the profile.",
            DotulousError::FailedReadManifest => "Failed to read profile manifest.",
            DotulousError::FailedDeserializeManifest => "Failed to deserialize profile manifest from JSON.",
            DotulousError::FailedSerializeManifest => "Failed to serialize profile manifest to JSON.",
            DotulousError::FailedSaveManifest => "Failed to save profile manifest to disk.",
            DotulousError::FillManifestArrayNotEmpty => "Manifest files array is already populated.",
            DotulousError::FailedReadProfileDirectory => "Failed to read from profile directory.",

            DotulousError::MetaNotFound => "Meta was not found.",
            DotulousError::FailedSerializeMeta => "Failed to serialize meta to JSON.",
            DotulousError::FailedDeserializeMeta => "Failed to deserialize meta from JSON.",
            DotulousError::FailedSaveMeta => "Failed to save meta to disk.",
        }
    }
}
impl Display for DotulousError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

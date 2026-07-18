use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::AppError;

const MAX_REMOTE_PATH_CHARS: usize = 1023;
const MAX_NAME_CHARS: usize = 255;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct MountId(String);

impl MountId {
    pub fn parse(value: String) -> Result<Self, AppError> {
        let valid = !value.is_empty()
            && value.len() <= 128
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'));
        if !valid {
            return Err(AppError::InvalidInput("mount id"));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct RemotePath(String);

impl RemotePath {
    pub fn parse(value: String) -> Result<Self, AppError> {
        let char_count = value.encode_utf16().count();
        if value.is_empty()
            || !value.starts_with('/')
            || char_count > MAX_REMOTE_PATH_CHARS
            || value.contains('\0')
            || (value.len() > 1 && value.ends_with('/'))
        {
            return Err(AppError::InvalidInput("remote path"));
        }

        if value != "/"
            && value[1..]
                .split('/')
                .any(|segment| segment.is_empty() || matches!(segment, "." | ".."))
        {
            return Err(AppError::InvalidInput("remote path segment"));
        }

        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn require_non_root(&self) -> Result<(), AppError> {
        if self.0 == "/" {
            Err(AppError::InvalidInput("root path cannot be changed"))
        } else {
            Ok(())
        }
    }

    pub fn file_name(&self) -> Result<&str, AppError> {
        self.require_non_root()?;
        self.0
            .rsplit('/')
            .next()
            .filter(|name| !name.is_empty())
            .ok_or(AppError::InvalidInput("remote file name"))
    }

    pub fn parent(&self) -> Result<Self, AppError> {
        self.require_non_root()?;
        let parent = self
            .0
            .rsplit_once('/')
            .map(|(parent, _)| if parent.is_empty() { "/" } else { parent })
            .ok_or(AppError::InvalidInput("remote parent"))?;
        Ok(Self(parent.to_owned()))
    }

    pub fn join(&self, name: &RemoteName) -> Result<Self, AppError> {
        let joined = if self.0 == "/" {
            format!("/{}", name.as_str())
        } else {
            format!("{}/{}", self.0, name.as_str())
        };
        Self::parse(joined)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RemoteName(String);

impl RemoteName {
    pub fn parse(value: String) -> Result<Self, AppError> {
        let char_count = value.encode_utf16().count();
        if value.is_empty()
            || char_count > MAX_NAME_CHARS
            || value.contains('/')
            || value.contains('\0')
            || matches!(value.as_str(), "." | "..")
        {
            return Err(AppError::InvalidInput("remote name"));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug)]
pub struct LocalUploadPath(PathBuf);

impl LocalUploadPath {
    pub async fn from_selected(path: PathBuf) -> Result<Self, AppError> {
        if !path.is_absolute() {
            return Err(AppError::InvalidInput("local upload path"));
        }
        let metadata = tokio::fs::symlink_metadata(&path).await?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err(AppError::InvalidInput("local upload file"));
        }
        Ok(Self(path))
    }

    pub fn as_path(&self) -> &Path {
        &self.0
    }

    pub fn file_name(&self) -> Result<String, AppError> {
        self.0
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_owned)
            .ok_or(AppError::InvalidInput("local file name"))
    }
}

#[derive(Clone, Debug)]
pub struct LocalDownloadPath(PathBuf);

impl LocalDownloadPath {
    pub async fn from_parent(parent: PathBuf, name: &RemoteName) -> Result<Self, AppError> {
        if !parent.is_absolute() {
            return Err(AppError::InvalidInput("local download parent"));
        }
        let metadata = tokio::fs::symlink_metadata(&parent).await?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(AppError::InvalidInput("local download parent"));
        }
        let preferred = safe_suggested_file_name(name);
        for index in 1_u64.. {
            let candidate = if index == 1 {
                preferred.clone()
            } else {
                numbered_file_name(&preferred, index)
            };
            let target = parent.join(candidate);
            if !tokio::fs::try_exists(&target).await? {
                return Self::from_selected(target).await;
            }
        }
        unreachable!()
    }

    pub async fn from_selected(path: PathBuf) -> Result<Self, AppError> {
        if !path.is_absolute() || path.file_name().is_none() {
            return Err(AppError::InvalidInput("local download path"));
        }
        if tokio::fs::try_exists(&path).await? {
            return Err(AppError::Conflict);
        }
        let parent = path
            .parent()
            .ok_or(AppError::InvalidInput("local download parent"))?;
        let metadata = tokio::fs::metadata(parent).await?;
        if !metadata.is_dir() {
            return Err(AppError::InvalidInput("local download parent"));
        }
        Ok(Self(path))
    }

    pub fn as_path(&self) -> &Path {
        &self.0
    }

    pub fn resumable_temporary_path(&self, transfer_id: &str) -> Result<PathBuf, AppError> {
        let transfer_id = uuid::Uuid::parse_str(transfer_id)
            .map_err(|_| AppError::InvalidInput("transfer id"))?;
        if transfer_id.is_nil() {
            return Err(AppError::InvalidInput("transfer id"));
        }
        let file_name = self
            .0
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or(AppError::InvalidInput("local download file name"))?;
        Ok(self
            .0
            .with_file_name(format!(".{file_name}.koofr-part-{transfer_id}")))
    }
}

fn numbered_file_name(preferred: &str, index: u64) -> String {
    let path = Path::new(preferred);
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(preferred);
    match path.extension().and_then(|value| value.to_str()) {
        Some(extension) => format!("{stem} ({index}).{extension}"),
        None => format!("{stem} ({index})"),
    }
}

pub fn safe_suggested_file_name(name: &RemoteName) -> String {
    let mut sanitized: String = name
        .as_str()
        .chars()
        .map(|character| {
            if character.is_control()
                || matches!(
                    character,
                    '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
                )
            {
                '_'
            } else {
                character
            }
        })
        .collect();
    sanitized = sanitized.trim_end_matches(['.', ' ']).to_owned();
    if sanitized.is_empty() {
        return "download".to_owned();
    }

    let stem = sanitized
        .split_once('.')
        .map_or(sanitized.as_str(), |(stem, _)| stem)
        .to_ascii_uppercase();
    let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || stem.strip_prefix("COM").is_some_and(|suffix| {
            matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
        })
        || stem.strip_prefix("LPT").is_some_and(|suffix| {
            matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
        });
    if reserved {
        if sanitized.encode_utf16().count() == MAX_NAME_CHARS {
            sanitized.pop();
        }
        sanitized.insert(0, '_');
    }
    sanitized
}

#[cfg(test)]
mod tests {
    use super::{LocalDownloadPath, MountId, RemoteName, RemotePath, safe_suggested_file_name};

    #[test]
    fn validates_mount_identifiers() {
        assert!(MountId::parse("abc_DEF-123".to_owned()).is_ok());
        assert!(MountId::parse("../mount".to_owned()).is_err());
        assert!(MountId::parse(String::new()).is_err());
    }

    #[test]
    fn rejects_ambiguous_remote_paths() {
        for path in ["", "relative", "/a/../b", "/a//b", "/a/"] {
            assert!(
                RemotePath::parse(path.to_owned()).is_err(),
                "accepted {path}"
            );
        }
        assert!(RemotePath::parse("/资料/计划.txt".to_owned()).is_ok());
    }

    #[test]
    fn joins_remote_paths_without_platform_path_rules() {
        let parent = RemotePath::parse("/资料".to_owned()).expect("valid parent");
        let name = RemoteName::parse("预算.xlsx".to_owned()).expect("valid name");
        assert_eq!(
            parent.join(&name).expect("joined").as_str(),
            "/资料/预算.xlsx"
        );
    }

    #[test]
    fn root_cannot_be_mutated() {
        let root = RemotePath::parse("/".to_owned()).expect("valid root");
        assert!(root.require_non_root().is_err());
    }

    #[test]
    fn sanitizes_windows_download_suggestions() {
        let reserved = RemoteName::parse("CON.txt".to_owned()).expect("valid remote name");
        let invalid = RemoteName::parse("report:final?.pdf".to_owned()).expect("valid remote name");
        assert_eq!(safe_suggested_file_name(&reserved), "_CON.txt");
        assert_eq!(safe_suggested_file_name(&invalid), "report_final_.pdf");
    }

    #[test]
    fn counts_non_bmp_names_as_two_utf16_units() {
        let too_long = "😀".repeat(128);
        assert!(RemoteName::parse(too_long).is_err());
    }

    #[tokio::test]
    async fn chooses_a_numbered_file_name_instead_of_overwriting() {
        // Given
        let directory =
            std::env::temp_dir().join(format!("koofr-download-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).expect("create downloads directory");
        std::fs::write(directory.join("report.pdf"), b"existing").expect("create existing file");
        let name = RemoteName::parse("report.pdf".to_owned()).expect("valid remote name");

        // When
        let target = LocalDownloadPath::from_parent(directory.clone(), &name)
            .await
            .expect("choose available target");

        // Then
        assert_eq!(target.as_path(), directory.join("report (2).pdf"));
        std::fs::remove_dir_all(directory).expect("remove downloads directory");
    }
}

use reqwest::{Method, StatusCode, Url};
use serde::{Deserialize, Serialize};

use super::KoofrApi;
use crate::{
    error::AppError,
    file_ops::{MountId, RemotePath},
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PublicLinkKind {
    Download,
    Upload,
}

impl PublicLinkKind {
    fn resource(self) -> &'static str {
        match self {
            Self::Download => "links",
            Self::Upload => "receivers",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicLink {
    pub id: String,
    pub name: String,
    pub path: String,
    pub counter: u64,
    pub url: String,
    pub short_url: String,
    pub has_password: bool,
    pub kind: PublicLinkKind,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemotePublicLink {
    id: String,
    #[serde(default)]
    name: String,
    path: String,
    #[serde(default, deserialize_with = "deserialize_counter")]
    counter: u64,
    url: String,
    #[serde(default)]
    short_url: String,
    #[serde(default)]
    has_password: bool,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RemoteCounter {
    Number(u64),
    Text(String),
}

fn deserialize_counter<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = RemoteCounter::deserialize(deserializer)?;
    match value {
        RemoteCounter::Number(value) => Ok(value),
        RemoteCounter::Text(value) => value.parse().map_err(serde::de::Error::custom),
    }
}

impl RemotePublicLink {
    fn with_kind(self, kind: PublicLinkKind) -> PublicLink {
        PublicLink {
            id: self.id,
            name: self.name,
            path: self.path,
            counter: self.counter,
            url: self.url,
            short_url: self.short_url,
            has_password: self.has_password,
            kind,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum DownloadList {
    Wrapped { links: Vec<RemotePublicLink> },
    Direct(Vec<RemotePublicLink>),
}

impl DownloadList {
    fn into_links(self) -> Vec<RemotePublicLink> {
        match self {
            Self::Wrapped { links } | Self::Direct(links) => links,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum UploadList {
    Wrapped { receivers: Vec<RemotePublicLink> },
    Direct(Vec<RemotePublicLink>),
}

impl UploadList {
    fn into_links(self) -> Vec<RemotePublicLink> {
        match self {
            Self::Wrapped { receivers } | Self::Direct(receivers) => receivers,
        }
    }
}

#[derive(Serialize)]
struct CreateLinkRequest<'a> {
    path: &'a str,
}

#[derive(Debug)]
struct PublicLinkId(String);

impl PublicLinkId {
    fn parse(value: &str) -> Result<Self, AppError> {
        let valid = !value.is_empty()
            && value.len() <= 256
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'));
        if !valid {
            return Err(AppError::InvalidInput("public link id"));
        }
        Ok(Self(value.to_owned()))
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

impl KoofrApi {
    pub async fn list_public_links(&self, mount_id: &MountId) -> Result<Vec<PublicLink>, AppError> {
        let (downloads, uploads) = tokio::try_join!(
            self.list_public_links_by_kind(mount_id, PublicLinkKind::Download),
            self.list_public_links_by_kind(mount_id, PublicLinkKind::Upload),
        )?;
        Ok(downloads.into_iter().chain(uploads).collect())
    }

    pub async fn create_public_link(
        &self,
        mount_id: &MountId,
        path: &RemotePath,
        kind: PublicLinkKind,
    ) -> Result<PublicLink, AppError> {
        let url = self.public_links_endpoint(mount_id, kind)?;
        let request_path = match (kind, path.as_str()) {
            (PublicLinkKind::Download, value) | (PublicLinkKind::Upload, value @ "/") => {
                value.to_owned()
            }
            (PublicLinkKind::Upload, value) => format!("{value}/"),
        };
        let response = self
            .authenticated_request(Method::POST, url)
            .await?
            .json(&CreateLinkRequest {
                path: &request_path,
            })
            .send()
            .await?;
        let response = Self::expect_status(response, &[StatusCode::OK, StatusCode::CREATED])?;
        let payload: RemotePublicLink = Self::decode_json(response).await?;
        Ok(payload.with_kind(kind))
    }

    pub async fn delete_public_link(
        &self,
        mount_id: &MountId,
        link_id: &str,
        kind: PublicLinkKind,
    ) -> Result<(), AppError> {
        let link_id = PublicLinkId::parse(link_id)?;
        let url = self.public_link_endpoint(mount_id, kind, &link_id)?;
        let response = self
            .authenticated_request(Method::DELETE, url)
            .await?
            .send()
            .await?;
        Self::expect_status(response, &[StatusCode::OK, StatusCode::NO_CONTENT])?;
        Ok(())
    }

    async fn list_public_links_by_kind(
        &self,
        mount_id: &MountId,
        kind: PublicLinkKind,
    ) -> Result<Vec<PublicLink>, AppError> {
        let url = self.public_links_endpoint(mount_id, kind)?;
        let response = self
            .authenticated_request(Method::GET, url)
            .await?
            .send()
            .await?;
        let response = Self::expect_status(response, &[StatusCode::OK])?;
        let links = match kind {
            PublicLinkKind::Download => {
                let payload: DownloadList = Self::decode_json(response).await?;
                payload.into_links()
            }
            PublicLinkKind::Upload => {
                let payload: UploadList = Self::decode_json(response).await?;
                payload.into_links()
            }
        };
        Ok(links.into_iter().map(|link| link.with_kind(kind)).collect())
    }

    fn public_links_endpoint(
        &self,
        mount_id: &MountId,
        kind: PublicLinkKind,
    ) -> Result<Url, AppError> {
        self.endpoint(&["api", "v2.1", "mounts", mount_id.as_str(), kind.resource()])
    }

    fn public_link_endpoint(
        &self,
        mount_id: &MountId,
        kind: PublicLinkKind,
        link_id: &PublicLinkId,
    ) -> Result<Url, AppError> {
        self.endpoint(&[
            "api",
            "v2.1",
            "mounts",
            mount_id.as_str(),
            kind.resource(),
            link_id.as_str(),
        ])
    }
}

#[cfg(test)]
mod tests;

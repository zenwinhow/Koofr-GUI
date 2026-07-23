use reqwest::{Method, StatusCode};
use serde::{Deserialize, Serialize};

use super::KoofrApi;
use crate::error::AppError;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultRepo {
    pub id: String,
    pub name: String,
    pub mount_id: String,
    pub path: String,
    pub salt: Option<String>,
    pub password_validator: String,
    pub password_validator_encrypted: String,
    pub added: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultRepoCreate {
    pub mount_id: String,
    pub path: String,
    pub salt: Option<String>,
    pub password_validator: String,
    pub password_validator_encrypted: String,
}

#[derive(Debug, Deserialize)]
struct VaultRepos {
    repos: Vec<VaultRepo>,
}

impl KoofrApi {
    pub async fn list_vault_repos(&self) -> Result<Vec<VaultRepo>, AppError> {
        let url = self.endpoint(&["api", "v2.1", "vault", "repos"])?;
        let response = self
            .authenticated_request(Method::GET, url)
            .await?
            .send()
            .await?;
        let response = Self::expect_status(response, &[StatusCode::OK])?;
        let payload: VaultRepos = Self::decode_json(response).await?;
        Ok(payload.repos)
    }

    pub async fn create_vault_repo(&self, create: &VaultRepoCreate) -> Result<VaultRepo, AppError> {
        let url = self.endpoint(&["api", "v2.1", "vault", "repos"])?;
        let response = self
            .authenticated_request(Method::POST, url)
            .await?
            .json(create)
            .send()
            .await?;
        let response = Self::expect_status(response, &[StatusCode::CREATED])?;
        Self::decode_json(response).await
    }

    pub async fn remove_vault_repo(&self, repo_id: &str) -> Result<(), AppError> {
        validate_repo_id(repo_id)?;
        let url = self.endpoint(&["api", "v2.1", "vault", "repos", repo_id])?;
        let response = self
            .authenticated_request(Method::DELETE, url)
            .await?
            .send()
            .await?;
        Self::expect_status(response, &[StatusCode::OK, StatusCode::NO_CONTENT])?;
        Ok(())
    }
}

fn validate_repo_id(value: &str) -> Result<(), AppError> {
    let valid = !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-');
    if valid {
        Ok(())
    } else {
        Err(AppError::InvalidInput("vault repo id"))
    }
}

#[cfg(test)]
mod tests {
    use httpmock::{Method::GET, MockServer};

    use super::KoofrApi;

    #[tokio::test]
    async fn lists_vault_repositories_without_exposing_validators_to_the_frontend_layer() {
        let server = MockServer::start();
        let api = KoofrApi::new(&server.base_url()).expect("mock api");
        let token = server.mock(|when, then| {
            when.method(httpmock::Method::POST).path("/token");
            then.status(200)
                .json_body(serde_json::json!({ "token": "test-token" }));
        });
        api.authenticate("test@example.invalid", "test-password")
            .await
            .expect("authenticate");
        token.assert();
        let repos = server.mock(|when, then| {
            when.method(GET)
                .path("/api/v2.1/vault/repos")
                .header("authorization", "Token token=test-token");
            then.status(200).json_body(serde_json::json!({
                "repos": [{
                    "id": "repo-1",
                    "name": "Safe Box",
                    "mountId": "mount-1",
                    "path": "/Safe Box",
                    "salt": "test-salt",
                    "passwordValidator": "validator",
                    "passwordValidatorEncrypted": "encrypted",
                    "added": 123
                }]
            }));
        });

        let result = api.list_vault_repos().await.expect("list repos");
        repos.assert();
        assert_eq!(result[0].name, "Safe Box");
    }
}

use crate::uri_meta::UriMeta;
use anyhow::Context;
use reqwest::{header, Client, Url};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Clone)]
pub struct GApi {
    domain: String,
    auth_token: String,
    client: Client,
}

#[derive(Clone, Debug)]
pub struct GitlabResource {
    url: String,
    auth_token: String,
    client: Client,
    pub meta: GitlabResourceMeta,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct GitlabResourceMeta {
    pub id: u32,
    // exists only for groups
    pub full_path: Option<String>,
    // exists only for repos
    pub path_with_namespace: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Default, Debug)]
pub struct GitlabVariable {
    pub key: String,
    pub value: String,
    pub description: Option<String>,
}

async fn get_page(
    client: Client,
    url: Url,
    page: u32,
    headers: header::HeaderMap,
) -> anyhow::Result<String> {
    let url = url
        .clone()
        .join(&format!("?page={}", page))
        .context("failed to create paged url")?;
    let resp = client.get(url).headers(headers.clone()).send().await?;

    Ok(resp.text().await?)
}
async fn get_all_pages<T: DeserializeOwned>(
    client: &Client,
    url: Url,
    auth_token: &str,
) -> anyhow::Result<Vec<T>> {
    let mut header = header::HeaderMap::new();
    header.insert("PRIVATE-TOKEN", header::HeaderValue::from_str(auth_token)?);

    let response = client
        .get(url.clone())
        .headers(header.clone())
        .send()
        .await?;
    let rheaders = response.headers();

    let total_pages = rheaders
        .get("x-total-pages")
        .context("expected paged result but got something else")?
        .to_str()?
        .parse::<u32>()?;
    let mut tasks = vec![];
    for i in 1..=total_pages {
        let url = url.clone();
        let page = i;
        let header = header.clone();
        let client = client.clone();
        tasks.push(tokio::spawn(async move {
            get_page(client, url, page, header).await
        }));
    }
    let mut result = vec![];
    for task in tasks {
        result.append(&mut serde_json::from_str::<Vec<T>>(&task.await??)?);
    }
    Ok(result)
}

impl GApi {
    pub fn new(domain: &str, token: &str) -> Self {
        Self {
            domain: domain.to_string(),
            auth_token: token.to_string(),
            client: Client::new(),
        }
    }

    pub async fn resource_from_uri(&self, uri: &UriMeta) -> anyhow::Result<GitlabResource> {
        let groups = self.groups().await?;
        let expected_path = uri.tokens.join("/");

        let containing_group = 'a: {
            for group in groups.iter() {
                if group
                    .full_path
                    .as_ref()
                    .is_some_and(|x| *x == expected_path)
                {
                    // early return the requested resource was in fact a group
                    return Ok(GitlabResource {
                        url: format!("https://{}/api/v4/groups/{}", self.domain, group.id)
                            .to_string(),
                        auth_token: self.auth_token.clone(),
                        client: self.client.clone(),
                        meta: group.clone(),
                    });
                }
            }
            let probable_group_path = uri.tokens[..uri.tokens.len() - 1].join("/");
            for group in groups.into_iter() {
                if group
                    .full_path
                    .as_ref()
                    .is_some_and(|x| *x == probable_group_path)
                {
                    break 'a group;
                }
            }
            anyhow::bail!("no containing group found")
        };

        // we've got a group that should contain a project macthing the expected_path
        let projects = self.projects(containing_group.id).await?;
        for project in projects.into_iter() {
            if project
                .path_with_namespace
                .as_ref()
                .is_some_and(|x| *x == expected_path)
            {
                return Ok(GitlabResource {
                    url: format!("https://{}/api/v4/projects/{}", self.domain, project.id)
                        .to_string(),
                    auth_token: self.auth_token.clone(),
                    client: self.client.clone(),
                    meta: project.clone(),
                });
            }
        }
        anyhow::bail!("found no gitlab resource")
    }

    async fn groups(&self) -> anyhow::Result<Vec<GitlabResourceMeta>> {
        let url = Url::parse(&format!("https://{}/api/v4/groups", self.domain))?;
        get_all_pages::<GitlabResourceMeta>(&self.client, url, &self.auth_token).await
    }

    pub async fn projects(&self, group_id: u32) -> anyhow::Result<Vec<GitlabResourceMeta>> {
        let url = Url::parse(&format!(
            "https://{}/api/v4/groups/{}/projects",
            self.domain, group_id
        ))?;
        get_all_pages::<GitlabResourceMeta>(&self.client, url, &self.auth_token).await
    }
}

impl GitlabResource {
    pub async fn variables(&self) -> anyhow::Result<Vec<GitlabVariable>> {
        let url = Url::parse(&(self.url.clone() + "/variables"))?;
        get_all_pages::<GitlabVariable>(&self.client, url, &self.auth_token).await
    }
}

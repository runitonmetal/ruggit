use anyhow::{Context, Result};
use git2::Repository;
use regex::Regex;

static PATTERN_DOMAIN: &str = r"gitlab.*\.[a-z, A-Z, 0-9]*(:|\/)";
static PATTERN_URL_TOKENS: &str = r"[^:|\/]+";

#[derive(PartialEq, Debug)]
pub enum Source {
    Disk(String),
    Web(String),
}

#[derive(PartialEq, Clone, Debug)]
pub enum Resource {
    Repo,
    Group,
}

#[derive(Default, Clone, Debug)]
pub struct UriMeta {
    pub identifier: String,
    pub url: String,
    pub domain: String,
    pub tokens: Vec<String>,
    pub resource: Option<Resource>,
}

fn parse_tokens(url: &str) -> Option<Vec<String>> {
    // match everything after domain delimeter
    let re = Regex::new(PATTERN_URL_TOKENS).unwrap();
    let mut tokens: Vec<String> = re
        .captures_iter(url)
        .map(|c| c.extract::<0>().0.to_string())
        .collect();
    // if all we got is the domain part, then just bail out
    if tokens.len() > 1 {
        // we might get a git url here, and we don't want to preserve anything
        // protocol specific, alas, strip .git
        let last = tokens.last_mut().unwrap();
        if last.ends_with(".git") {
            *last = last[..last.len() - ".git".len()].to_string();
        }
        // skip first match as thats the domain part
        return Some(tokens[1..].to_vec());
    }
    None
}

fn parse_domain(url: &str) -> Option<String> {
    // skip possible leading protocol prefix, git@...
    let reg = Regex::new(PATTERN_DOMAIN).unwrap();
    if let Some(matches) = reg.captures(url) {
        // the first group is always the entire capture
        let domain_part = matches.get(0).unwrap();
        let start = domain_part.start();
        // skip the delimeter (:|/)
        let end = domain_part.end() - 1;
        return Some(url[start..end].to_string());
    }
    None
}

fn make_url(domain: &str, tokens: &[String]) -> String {
    domain.to_string() + "/" + &tokens.join("/")
}

#[cfg(test)]
mod tests {
    use super::*;
    const URLS: [&str; 8] = [
        "git@gitlab.com:org/group/project.git",
        "malformed.git",
        "git@gitlab.com:org",
        "git@gitlab.com:org/",
        "git@gitlab.com:org/group/subgroup/project.git",
        "gitlab.com/foo",
        "gitlab.com:foo/bar",
        "gitlab.selfhosted.com/foo/bar",
    ];

    #[test]
    fn test_parse_domain() {
        let to_parse = vec![
            (URLS[0], Some("gitlab.com".to_string())),
            (URLS[1], None),
            (URLS[2], Some("gitlab.com".to_string())),
            (URLS[3], Some("gitlab.com".to_string())),
            (URLS[4], Some("gitlab.com".to_string())),
            (URLS[5], Some("gitlab.com".to_string())),
            (URLS[6], Some("gitlab.com".to_string())),
            (URLS[7], Some("gitlab.selfhosted.com".to_string())),
        ];

        for (url, expected) in to_parse.into_iter() {
            let result = parse_domain(url);
            assert_eq!(
                expected, result,
                "{} parsed unexpectedly to: {:?}",
                url, result
            );
        }
    }

    #[test]
    fn test_parse_tokens() {
        let scenarios = vec![
            (
                URLS[0],
                Some(
                    vec!["org", "group", "project"]
                        .into_iter()
                        .map(|x| x.to_string())
                        .collect(),
                ),
            ),
            (URLS[1], None),
            (URLS[2], Some(vec!["org".to_string()])),
            (URLS[3], Some(vec!["org".to_string()])),
            (
                URLS[4],
                Some(
                    vec!["org", "group", "subgroup", "project"]
                        .into_iter()
                        .map(|x| x.to_string())
                        .collect(),
                ),
            ),
            (URLS[5], Some(vec!["foo".to_string()])),
            (
                URLS[6],
                Some(
                    vec!["foo", "bar"]
                        .into_iter()
                        .map(|x| x.to_string())
                        .collect(),
                ),
            ),
            (
                URLS[7],
                Some(
                    vec!["foo", "bar"]
                        .into_iter()
                        .map(|x| x.to_string())
                        .collect(),
                ),
            ),
        ];
        for (url, expected) in scenarios {
            let result = parse_tokens(url);
            assert_eq!(
                result, expected,
                "{} parsed unexpectedly to: {:?}",
                url, result
            );
        }
    }
}

fn from_disk(path: &str) -> Result<UriMeta> {
    let repo = Repository::open(path)?;

    for remote in repo.remotes()?.iter() {
        let Some(remote) = remote else {
            continue;
        };

        if remote != "origin" {
            continue;
        }
        let info = repo.find_remote(remote)?;
        let mut repoinfo = UriMeta::default();

        let url = info.url().context("no remote url")?;
        repoinfo.domain = parse_domain(url).context("unable to parse domain")?;
        repoinfo.tokens = parse_tokens(url).context("unable to parse tokens")?;
        repoinfo.identifier = repoinfo.tokens.join("/");
        repoinfo.url = make_url(&repoinfo.domain, &repoinfo.tokens);
        // when parsing on disk it can only be a repo
        repoinfo.resource = Some(Resource::Repo);
        return Ok(repoinfo);
    }
    anyhow::bail!("no repo info found path: {}", path)
}

fn from_web(path: &str) -> Result<UriMeta> {
    let mut repoinfo = UriMeta::default();
    repoinfo.domain = parse_domain(path).context("unable to parse domain")?;
    repoinfo.tokens = parse_tokens(path).context("unable to parse tokens")?;
    repoinfo.identifier = repoinfo.tokens.join("/");
    repoinfo.url = make_url(&repoinfo.domain, &repoinfo.tokens);
    // resource is unset as we don't yet know what it is
    Ok(repoinfo)
}

impl UriMeta {
    pub fn new(source: &Source) -> Result<Self> {
        match source {
            Source::Web(url) => from_web(url),
            Source::Disk(path) => from_disk(path),
        }
    }
}

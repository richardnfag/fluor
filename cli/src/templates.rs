use crate::client::path_segment;
use crate::output::{template_text, templates_text};
use anyhow::{Context, Result, anyhow, bail};
use cargo_generate::{GenerateArgs, TemplatePath, Vcs};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::time::Duration;

const TEMPLATE_PREFIX: &str = "fluor-template-";
const GITHUB_API_VERSION: &str = "2022-11-28";

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Template {
    pub language: String,
    pub repository: String,
    pub git_ref: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct CreatedTemplate {
    pub path: String,
    pub template: Template,
}

pub struct Templates {
    api: String,
    owner: String,
    http: reqwest::Client,
    token: Option<String>,
}

impl Templates {
    pub fn new(api: impl Into<String>, owner: impl Into<String>) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .user_agent("fluor-cli")
            .build()
            .context("failed to build the GitHub client")?;
        let token = std::env::var("GITHUB_TOKEN")
            .or_else(|_| std::env::var("GH_TOKEN"))
            .ok()
            .filter(|token| !token.is_empty());
        Ok(Self {
            api: api.into().trim_end_matches('/').to_string(),
            owner: owner.into(),
            http,
            token,
        })
    }

    pub async fn list(&self) -> Result<Vec<Template>> {
        let mut templates = Vec::new();
        for repo in self.repositories().await? {
            if repo.archived || !repo.name.starts_with(TEMPLATE_PREFIX) {
                continue;
            }
            let language = repo.name[TEMPLATE_PREFIX.len()..].to_string();
            if language.is_empty() {
                continue;
            }
            let git_ref = self.resolve_ref(&repo, None).await?;
            templates.push(Template {
                language,
                repository: repo.full_name,
                git_ref,
                url: repo.html_url,
            });
        }
        templates.sort_by(|left, right| left.language.cmp(&right.language));
        Ok(templates)
    }

    pub async fn create(
        &self,
        language: &str,
        name: &str,
        output: Option<&Path>,
        git_ref: Option<&str>,
    ) -> Result<CreatedTemplate> {
        if name.trim().is_empty() {
            bail!("project name is empty");
        }
        let repo_name = format!("{TEMPLATE_PREFIX}{language}");
        let repo = self.repository(&repo_name).await?;
        if repo.archived {
            bail!("{repo_name} is archived");
        }
        let git_ref = self.resolve_ref(&repo, git_ref).await?;
        let destination = output
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from(name));
        if destination.exists() {
            bail!("{} already exists", destination.display());
        }
        let args = generation_args(&repo.clone_url(), &git_ref, name, &destination)?;
        let path = tokio::task::spawn_blocking(move || cargo_generate::generate(args))
            .await
            .context("template generation task failed")??;
        Ok(CreatedTemplate {
            path: path.display().to_string(),
            template: Template {
                language: language.to_string(),
                repository: repo.full_name,
                git_ref,
                url: repo.html_url,
            },
        })
    }

    async fn repositories(&self) -> Result<Vec<Repo>> {
        let mut url = Some(format!(
            "{}/users/{}/repos?per_page=100&type=owner",
            self.api,
            path_segment(&self.owner)
        ));
        let mut repos = Vec::new();
        while let Some(page) = url.take() {
            let response = self.request(&page).await?;
            let next = next_link(response.headers());
            let body = response
                .text()
                .await
                .context("failed to read GitHub repositories")?;
            let page: Vec<Repo> = serde_json::from_str(&body)
                .with_context(|| format!("invalid GitHub repository list: {body}"))?;
            repos.extend(page);
            url = next;
        }
        Ok(repos)
    }

    async fn repository(&self, name: &str) -> Result<Repo> {
        let url = format!(
            "{}/repos/{}/{}",
            self.api,
            path_segment(&self.owner),
            path_segment(name)
        );
        let response = self.request(&url).await?;
        let body = response
            .text()
            .await
            .context("failed to read the GitHub repository")?;
        serde_json::from_str(&body).with_context(|| format!("invalid GitHub repository: {body}"))
    }

    async fn resolve_ref(&self, repo: &Repo, requested: Option<&str>) -> Result<String> {
        if let Some(git_ref) = requested.filter(|git_ref| !git_ref.is_empty()) {
            return Ok(git_ref.to_string());
        }
        let url = format!(
            "{}/repos/{}/{}/pulls?state=open&per_page=20",
            self.api,
            path_segment(&self.owner),
            path_segment(&repo.name)
        );
        let response = self.request(&url).await?;
        let body = response
            .text()
            .await
            .context("failed to read GitHub pull requests")?;
        let pulls: Vec<Pull> = serde_json::from_str(&body)
            .with_context(|| format!("invalid GitHub pull request list: {body}"))?;
        let head = pulls
            .into_iter()
            .filter(|pull| {
                pull.head
                    .repo
                    .as_ref()
                    .is_some_and(|source| source.full_name == repo.full_name)
            })
            .max_by(|left, right| left.updated_at.cmp(&right.updated_at));
        Ok(head
            .map(|pull| pull.head.sha)
            .unwrap_or_else(|| repo.default_branch.clone()))
    }

    async fn request(&self, url: &str) -> Result<reqwest::Response> {
        let mut request = self
            .http
            .get(url)
            .header("accept", "application/vnd.github+json")
            .header("x-github-api-version", GITHUB_API_VERSION);
        if let Some(token) = &self.token {
            request = request.bearer_auth(token);
        }
        let response = request
            .send()
            .await
            .with_context(|| format!("failed to reach {url}"))?;
        if response.status().is_success() {
            return Ok(response);
        }
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        let message = serde_json::from_str::<GitHubError>(&body)
            .map(|error| error.message)
            .unwrap_or(body);
        Err(anyhow!("GitHub {status}: {message}"))
    }
}

fn generation_args(
    clone_url: &str,
    git_ref: &str,
    name: &str,
    destination: &Path,
) -> Result<GenerateArgs> {
    let destination = if destination.is_absolute() {
        destination.to_path_buf()
    } else {
        std::env::current_dir()
            .context("failed to read the current directory")?
            .join(destination)
    };
    std::fs::create_dir_all(&destination)
        .with_context(|| format!("failed to create {}", destination.display()))?;
    Ok(GenerateArgs {
        name: Some(name.to_string()),
        destination: Some(destination),
        init: true,
        silent: true,
        vcs: Some(Vcs::None),
        allow_commands: true,
        no_workspace: true,
        no_fmt: true,
        template_path: template_path(clone_url, git_ref),
        ..GenerateArgs::default()
    })
}

fn template_path(clone_url: &str, git_ref: &str) -> TemplatePath {
    TemplatePath {
        git: Some(clone_url.to_string()),
        branch: (!is_revision(git_ref)).then(|| git_ref.to_string()),
        revision: is_revision(git_ref).then(|| git_ref.to_string()),
        ..TemplatePath::default()
    }
}

fn is_revision(git_ref: &str) -> bool {
    git_ref.len() >= 7 && git_ref.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub fn templates_output(templates: &[Template], json: bool) -> Result<String> {
    if json {
        Ok(serde_json::to_string_pretty(templates)?)
    } else {
        Ok(templates_text(templates))
    }
}

pub fn created_output(created: &CreatedTemplate, json: bool) -> Result<String> {
    if json {
        Ok(serde_json::to_string_pretty(created)?)
    } else {
        Ok(format!(
            "created {} from {}@{}\n{}",
            created.path,
            created.template.repository,
            created.template.git_ref,
            template_text(&created.template)
        ))
    }
}

fn next_link(headers: &reqwest::header::HeaderMap) -> Option<String> {
    let value = headers.get(reqwest::header::LINK)?.to_str().ok()?;
    value.split(',').find_map(|part| {
        let (url, rest) = part.split_once('>')?;
        if rest.contains("rel=\"next\"") {
            Some(url.trim().trim_start_matches('<').to_string())
        } else {
            None
        }
    })
}

#[derive(Debug, Deserialize)]
struct Repo {
    name: String,
    full_name: String,
    #[serde(default)]
    archived: bool,
    default_branch: String,
    html_url: String,
    #[serde(default)]
    clone_url: String,
}

impl Repo {
    fn clone_url(&self) -> String {
        if self.clone_url.is_empty() {
            format!("https://github.com/{}.git", self.full_name)
        } else {
            self.clone_url.clone()
        }
    }
}

#[derive(Debug, Deserialize)]
struct Pull {
    updated_at: String,
    head: PullHead,
}

#[derive(Debug, Deserialize)]
struct PullHead {
    sha: String,
    repo: Option<PullRepo>,
}

#[derive(Debug, Deserialize)]
struct PullRepo {
    full_name: String,
}

#[derive(Debug, Deserialize)]
struct GitHubError {
    message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use cargo_generate::generate;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn uses_a_commit_as_a_revision_and_a_branch_name_as_a_branch() {
        let revision = template_path(
            "https://github.com/acme/fluor-template-rust.git",
            "0123456789abcdef",
        );
        assert_eq!(revision.revision.as_deref(), Some("0123456789abcdef"));
        assert!(revision.branch.is_none());

        let branch = template_path("https://github.com/acme/fluor-template-rust.git", "main");
        assert_eq!(branch.branch.as_deref(), Some("main"));
        assert!(branch.revision.is_none());
    }

    #[test]
    fn expands_placeholders_with_cargo_generate() {
        let root = tempfile::tempdir().unwrap();
        let template = root.path().join("template");
        std::fs::create_dir(&template).unwrap();
        std::fs::write(
            template.join("{{project-name}}.txt"),
            "{{project-name}} {{crate_name}} {{project-name | pascal_case}}\n",
        )
        .unwrap();
        let destination = root.path().join("sample");
        std::fs::create_dir(&destination).unwrap();
        let path = generate(GenerateArgs {
            name: Some("sample".to_string()),
            destination: Some(destination.clone()),
            init: true,
            silent: true,
            vcs: Some(Vcs::None),
            no_workspace: true,
            no_fmt: true,
            template_path: TemplatePath {
                path: Some(template.display().to_string()),
                ..TemplatePath::default()
            },
            ..GenerateArgs::default()
        })
        .unwrap();

        let source = std::fs::read_to_string(path.join("sample.txt")).unwrap();
        assert_eq!(source, "sample sample Sample\n");
        assert!(!path.join("cargo-generate.toml").exists());
    }

    #[test]
    fn expands_the_go_template_without_treating_composite_literals_as_liquid() {
        let root = tempfile::tempdir().unwrap();
        let template = root.path().join("template");
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fluor-template-go");
        if source.join("main.go").is_file() {
            copy_dir(&source, &template);
        } else {
            std::fs::create_dir(&template).unwrap();
            std::fs::write(
                template.join("main.go"),
                "package main\n\nfunc items() {\n\titems := []map[string]string{\n\t\t{\"name\": \"notebook\"},\n\t}\n\t_ = \"{{project-name}}\"\n}\n",
            )
            .unwrap();
        }
        let destination = root.path().join("out");
        std::fs::create_dir(&destination).unwrap();
        let path = generate(GenerateArgs {
            name: Some("cli-sample-go".to_string()),
            destination: Some(destination),
            init: true,
            silent: true,
            vcs: Some(Vcs::None),
            no_workspace: true,
            no_fmt: true,
            template_path: TemplatePath {
                path: Some(template.display().to_string()),
                ..TemplatePath::default()
            },
            ..GenerateArgs::default()
        })
        .unwrap();

        let generated = std::fs::read_to_string(path.join("main.go")).unwrap();
        assert!(generated.contains("cli-sample-go"));
        assert!(generated.contains("{\"name\": \"notebook\"}"));
        assert!(!generated.contains("{{"));
    }

    fn copy_dir(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).unwrap();
        for entry in std::fs::read_dir(from).unwrap() {
            let entry = entry.unwrap();
            if entry.file_name() == ".git" {
                continue;
            }
            let dest = to.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_dir(&entry.path(), &dest);
            } else {
                std::fs::copy(entry.path(), dest).unwrap();
            }
        }
    }

    #[tokio::test]
    async fn lists_active_templates_from_their_open_pull_requests() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/users/richardnfag/repos"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                repo("fluor-template-rust", false, "main"),
                repo("fluor-template-moonbit", true, "main"),
                repo("fluor", false, "main")
            ])))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/repos/richardnfag/fluor-template-rust/pulls"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {
                    "updated_at": "2026-11-02T12:00:00Z",
                    "head": {
                        "sha": "abc123def456abc123def456abc123def456abc1",
                        "repo": { "full_name": "richardnfag/fluor-template-rust" }
                    }
                }
            ])))
            .mount(&server)
            .await;

        let templates = Templates::new(server.uri(), "richardnfag").unwrap();
        let listed = templates.list().await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].language, "rust");
        assert_eq!(
            listed[0].git_ref,
            "abc123def456abc123def456abc123def456abc1"
        );
    }

    fn repo(name: &str, archived: bool, default_branch: &str) -> serde_json::Value {
        serde_json::json!({
            "name": name,
            "full_name": format!("richardnfag/{name}"),
            "archived": archived,
            "default_branch": default_branch,
            "html_url": format!("https://github.com/richardnfag/{name}"),
            "clone_url": format!("https://github.com/richardnfag/{name}.git")
        })
    }
}

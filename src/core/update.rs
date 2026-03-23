use serde::Deserialize;

pub const GITHUB_RELEASES_LATEST_URL: &str = "https://github.com/Inoxiamo/armando/releases/latest";
pub const GITHUB_RELEASES_API_URL: &str =
    "https://api.github.com/repos/Inoxiamo/armando/releases/latest";
pub const GITHUB_RELEASES_API_LIST_URL: &str =
    "https://api.github.com/repos/Inoxiamo/armando/releases?per_page=20";
pub const GITHUB_BOOTSTRAP_SCRIPT_URL: &str =
    "https://raw.githubusercontent.com/Inoxiamo/armando/master/scripts/release/bootstrap-release.sh";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateAction {
    CopyCommand { command: String },
    OpenReleasePage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateGuide {
    pub platform_label: String,
    pub detail: String,
    pub action: UpdateAction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseInfo {
    pub version: String,
    pub release_url: String,
}

#[derive(Debug, Clone, Deserialize)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    draft: bool,
}

pub async fn fetch_latest_release(include_beta: bool) -> Result<ReleaseInfo, String> {
    let client = reqwest::Client::builder()
        .build()
        .map_err(|err| format!("Could not initialize update client: {err}"))?;

    let release = if include_beta {
        let releases = client
            .get(GITHUB_RELEASES_API_LIST_URL)
            .header(reqwest::header::USER_AGENT, "armando-update-check")
            .header(reqwest::header::ACCEPT, "application/vnd.github+json")
            .send()
            .await
            .map_err(|err| format!("Could not contact GitHub: {err}"))?
            .error_for_status()
            .map_err(|err| format!("GitHub returned an error while checking updates: {err}"))?
            .json::<Vec<GithubRelease>>()
            .await
            .map_err(|err| format!("Could not parse GitHub release information: {err}"))?;

        select_latest_release(releases, true)
            .ok_or_else(|| "No eligible GitHub release found for beta channel.".to_string())?
    } else {
        client
            .get(GITHUB_RELEASES_API_URL)
            .header(reqwest::header::USER_AGENT, "armando-update-check")
            .header(reqwest::header::ACCEPT, "application/vnd.github+json")
            .send()
            .await
            .map_err(|err| format!("Could not contact GitHub: {err}"))?
            .error_for_status()
            .map_err(|err| format!("GitHub returned an error while checking updates: {err}"))?
            .json::<GithubRelease>()
            .await
            .map_err(|err| format!("Could not parse GitHub release information: {err}"))?
    };

    Ok(ReleaseInfo {
        version: normalize_version_tag(&release.tag_name),
        release_url: release.html_url,
    })
}

pub fn update_available(current_version: &str, latest_version: &str) -> bool {
    let Some(current) = parse_version(current_version) else {
        return false;
    };
    let Some(latest) = parse_version(latest_version) else {
        return false;
    };

    latest > current
}

pub fn current_platform_update_guide() -> UpdateGuide {
    platform_update_guide_for(std::env::consts::OS)
}

fn platform_update_guide_for(target_os: &str) -> UpdateGuide {
    match target_os {
        "linux" => UpdateGuide {
            platform_label: "Linux".to_string(),
            detail: "Use the bootstrap installer to download the latest bundle and run the bundled installer.".to_string(),
            action: UpdateAction::CopyCommand {
                command: format!("curl -fsSL {GITHUB_BOOTSTRAP_SCRIPT_URL} | bash"),
            },
        },
        "macos" => UpdateGuide {
            platform_label: "macOS".to_string(),
            detail: "Use the bootstrap installer to download the latest bundle and run the bundled installer.".to_string(),
            action: UpdateAction::CopyCommand {
                command: format!("curl -fsSL {GITHUB_BOOTSTRAP_SCRIPT_URL} | bash"),
            },
        },
        "windows" => UpdateGuide {
            platform_label: "Windows".to_string(),
            detail: "Download the latest .zip release, extract it, then run scripts\\install.ps1 from PowerShell.".to_string(),
            action: UpdateAction::OpenReleasePage,
        },
        _ => UpdateGuide {
            platform_label: target_os.to_string(),
            detail: "Open the latest release page and follow the packaged install instructions for your platform.".to_string(),
            action: UpdateAction::OpenReleasePage,
        },
    }
}

fn parse_version(version: &str) -> Option<ParsedVersion> {
    let normalized = normalize_version_tag(version);
    let mut parts = normalized.splitn(2, '-');
    let core = parts.next()?;
    let prerelease = parts
        .next()
        .map(parse_prerelease_identifiers)
        .unwrap_or_default();

    let numbers = core
        .split('.')
        .map(|part| part.parse::<u64>().ok())
        .collect::<Option<Vec<_>>>()?;

    if numbers.is_empty() {
        return None;
    }

    Some(ParsedVersion {
        numbers,
        prerelease,
    })
}

fn normalize_version_tag(version: &str) -> String {
    version.trim().trim_start_matches('v').to_string()
}

fn select_latest_release(
    releases: Vec<GithubRelease>,
    include_beta: bool,
) -> Option<GithubRelease> {
    releases
        .into_iter()
        .filter(|release| !release.draft)
        .find(|release| include_beta || !release.prerelease)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedVersion {
    numbers: Vec<u64>,
    prerelease: Vec<PrereleaseIdentifier>,
}

impl Ord for ParsedVersion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let number_order = compare_number_slices(&self.numbers, &other.numbers);
        if number_order != std::cmp::Ordering::Equal {
            return number_order;
        }

        match (self.prerelease.is_empty(), other.prerelease.is_empty()) {
            (true, true) => std::cmp::Ordering::Equal,
            (true, false) => std::cmp::Ordering::Greater,
            (false, true) => std::cmp::Ordering::Less,
            (false, false) => compare_prerelease_slices(&self.prerelease, &other.prerelease),
        }
    }
}

impl PartialOrd for ParsedVersion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PrereleaseIdentifier {
    Numeric(u64),
    Alpha(String),
}

impl Ord for PrereleaseIdentifier {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (Self::Numeric(left), Self::Numeric(right)) => left.cmp(right),
            (Self::Alpha(left), Self::Alpha(right)) => left.cmp(right),
            (Self::Numeric(_), Self::Alpha(_)) => std::cmp::Ordering::Less,
            (Self::Alpha(_), Self::Numeric(_)) => std::cmp::Ordering::Greater,
        }
    }
}

impl PartialOrd for PrereleaseIdentifier {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

fn compare_number_slices(left: &[u64], right: &[u64]) -> std::cmp::Ordering {
    for index in 0..left.len().max(right.len()) {
        let left_value = left.get(index).copied().unwrap_or(0);
        let right_value = right.get(index).copied().unwrap_or(0);
        let order = left_value.cmp(&right_value);
        if order != std::cmp::Ordering::Equal {
            return order;
        }
    }
    std::cmp::Ordering::Equal
}

fn compare_prerelease_slices(
    left: &[PrereleaseIdentifier],
    right: &[PrereleaseIdentifier],
) -> std::cmp::Ordering {
    for index in 0..left.len().max(right.len()) {
        match (left.get(index), right.get(index)) {
            (Some(left_id), Some(right_id)) => {
                let order = left_id.cmp(right_id);
                if order != std::cmp::Ordering::Equal {
                    return order;
                }
            }
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (None, None) => return std::cmp::Ordering::Equal,
        }
    }

    std::cmp::Ordering::Equal
}

fn parse_prerelease_identifiers(value: &str) -> Vec<PrereleaseIdentifier> {
    value.split('.').flat_map(split_identifier_chunks).collect()
}

fn split_identifier_chunks(value: &str) -> Vec<PrereleaseIdentifier> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut current_is_digit = None;

    for ch in value.chars() {
        let is_digit = ch.is_ascii_digit();
        match current_is_digit {
            Some(flag) if flag != is_digit => {
                push_identifier_chunk(&mut chunks, &current, flag);
                current.clear();
                current.push(ch);
                current_is_digit = Some(is_digit);
            }
            Some(_) => current.push(ch),
            None => {
                current.push(ch);
                current_is_digit = Some(is_digit);
            }
        }
    }

    if let Some(flag) = current_is_digit {
        push_identifier_chunk(&mut chunks, &current, flag);
    }

    chunks
}

fn push_identifier_chunk(chunks: &mut Vec<PrereleaseIdentifier>, value: &str, is_digit: bool) {
    if value.is_empty() {
        return;
    }

    if is_digit {
        if let Ok(number) = value.parse::<u64>() {
            chunks.push(PrereleaseIdentifier::Numeric(number));
        }
    } else {
        chunks.push(PrereleaseIdentifier::Alpha(value.to_ascii_lowercase()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_newer_release_versions() {
        assert!(update_available("0.0.2-rc1", "0.0.2"));
        assert!(update_available("0.0.2", "0.1.0"));
        assert!(!update_available("0.1.0", "0.0.2"));
        assert!(!update_available("0.0.2", "0.0.2"));
    }

    #[test]
    fn normalizes_leading_v_in_tags() {
        assert!(update_available("0.0.2", "v0.0.3"));
        assert!(!update_available("0.0.3", "v0.0.3"));
    }

    #[test]
    fn compares_prerelease_suffixes_numerically() {
        assert!(update_available("0.0.2-rc1", "0.0.2-rc2"));
        assert!(update_available("0.0.2-beta9", "0.0.2-beta10"));
    }

    #[test]
    fn returns_false_for_invalid_versions() {
        assert!(!update_available("dev-build", "0.0.3"));
        assert!(!update_available("0.0.2", "latest"));
    }

    #[test]
    fn linux_update_guide_prefers_bootstrap_command() {
        let guide = platform_update_guide_for("linux");
        assert_eq!(guide.platform_label, "Linux");
        assert!(guide.detail.contains("bootstrap"));
        match guide.action {
            UpdateAction::CopyCommand { command } => {
                assert!(command.contains(GITHUB_BOOTSTRAP_SCRIPT_URL));
            }
            UpdateAction::OpenReleasePage => panic!("linux should prefer a copyable command"),
        }
    }

    #[test]
    fn exposes_bootstrap_script_url() {
        assert!(GITHUB_BOOTSTRAP_SCRIPT_URL.starts_with("https://"));
        assert!(GITHUB_BOOTSTRAP_SCRIPT_URL.ends_with("bootstrap-release.sh"));
    }

    #[test]
    fn windows_update_guide_prefers_release_page() {
        let guide = platform_update_guide_for("windows");
        assert_eq!(guide.platform_label, "Windows");
        assert!(guide.detail.contains("install.ps1"));
        assert!(matches!(guide.action, UpdateAction::OpenReleasePage));
    }

    #[test]
    fn release_selector_respects_beta_channel() {
        let releases = vec![
            GithubRelease {
                tag_name: "v0.0.4-rc1".to_string(),
                html_url: "https://example/rc".to_string(),
                prerelease: true,
                draft: false,
            },
            GithubRelease {
                tag_name: "v0.0.3".to_string(),
                html_url: "https://example/stable".to_string(),
                prerelease: false,
                draft: false,
            },
        ];
        let beta = select_latest_release(releases.clone(), true).unwrap();
        assert_eq!(beta.tag_name, "v0.0.4-rc1");

        let stable_only = select_latest_release(releases, false).unwrap();
        assert_eq!(stable_only.tag_name, "v0.0.3");
    }
}

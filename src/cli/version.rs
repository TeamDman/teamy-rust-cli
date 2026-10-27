use super::Cli;
use chrono::DateTime;
use chrono::Local;
use chrono::Utc;

impl Cli {
    /// Version string combining package version, git repository metadata, and build time.
    #[must_use]
    pub fn version() -> String {
        let built_at = option_env!("BUILD_TIMESTAMP_UNIX")
            .and_then(|value| value.parse::<i64>().ok())
            .and_then(|timestamp| DateTime::<Utc>::from_timestamp(timestamp, 0))
            .map_or_else(
                || "unknown build time".to_string(),
                |timestamp| {
                    timestamp
                        .with_timezone(&Local)
                        .format("%Y-%m-%d %H:%M:%S %Z")
                        .to_string()
                },
            );

        format!(
            "{} (repo {}, branch {}, rev {}, worktree {}, built {})",
            env!("CARGO_PKG_VERSION"),
            env!("GIT_REPOSITORY_URL"),
            env!("GIT_BRANCH"),
            env!("GIT_REVISION"),
            env!("GIT_WORKTREE_STATUS"),
            built_at,
        )
    }
}

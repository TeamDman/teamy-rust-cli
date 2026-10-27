use std::fmt::Write;
use std::path::Path;

/// Identity replacement belongs to the scaffold contract, not a generic text framework.
#[derive(Debug)]
pub(crate) struct ProjectIdentity {
    name: String,
    repository: Option<String>,
}

impl ProjectIdentity {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn validate(name: &str, repository: Option<&str>) -> eyre::Result<Self> {
        eyre::ensure!(
            !name.is_empty()
                && name.as_bytes()[0].is_ascii_lowercase()
                && name.bytes().all(|byte| byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || byte == b'-'
                    || byte == b'_'),
            "project name must begin with a lowercase letter and contain only lowercase letters, digits, hyphens or underscores"
        );
        // Strict and reserved keywords for the generated crate's Rust 2024 edition.
        // https://doc.rust-lang.org/reference/keywords.html
        eyre::ensure!(
            ![
                "_", "Self", "abstract", "as", "async", "await", "become", "box", "break", "const",
                "continue", "crate", "do", "dyn", "else", "enum", "extern", "false", "final", "fn",
                "for", "gen", "if", "impl", "in", "let", "loop", "macro", "match", "mod", "move",
                "mut", "override", "priv", "pub", "ref", "return", "self", "static", "struct",
                "super", "trait", "true", "try", "type", "typeof", "unsafe", "unsized", "use",
                "virtual", "where", "while", "yield"
            ]
            .contains(&name),
            "project name is a Rust keyword"
        );
        if let Some(repository) = repository {
            let coordinates = repository.strip_prefix("https://github.com/").unwrap_or("");
            let parts: Vec<_> = coordinates.split('/').collect();
            eyre::ensure!(
                parts.len() == 2
                    && parts.iter().all(|part| !part.is_empty()
                        && part.bytes().all(|byte| byte.is_ascii_alphanumeric()
                            || byte == b'-'
                            || byte == b'_'
                            || byte == b'.'))
                    && parts.iter().all(|part| *part != "." && *part != ".."),
                "repository URL must be an explicit https://github.com/owner/repository URL"
            );
        }
        Ok(Self {
            name: name.to_owned(),
            repository: repository.map(str::to_owned),
        })
    }

    pub fn render(&self, relative: &Path, bytes: Vec<u8>) -> Vec<u8> {
        let Ok(mut text) = String::from_utf8(bytes.clone()) else {
            return bytes;
        };
        // Only known text inputs are transformed; icons and other binary resources remain bytes.
        if !matches!(
            relative.extension().and_then(std::ffi::OsStr::to_str),
            Some("rs" | "toml" | "lock" | "md" | "ps1" | "yml" | "yaml" | "rc" | "manifest")
        ) {
            return bytes;
        }
        if relative == Path::new("Cargo.toml") && self.repository.is_none() {
            text = text
                .lines()
                .filter(|line| {
                    !line.starts_with("repository =") && !line.starts_with("documentation =")
                })
                .collect::<Vec<_>>()
                .join("\n")
                + "\n";
        }
        if relative == Path::new("Cargo.toml")
            && let Some(repository) = &self.repository
        {
            let mut metadata = String::new();
            if !text.lines().any(|line| line.starts_with("repository =")) {
                writeln!(metadata, "repository = \"{repository}\"")
                    .expect("write repository metadata");
            }
            if !text.lines().any(|line| line.starts_with("documentation =")) {
                writeln!(metadata, "documentation = \"{repository}\"")
                    .expect("write documentation metadata");
            }
            if !metadata.is_empty() {
                text = text.replacen("[package]\n", &format!("[package]\n{metadata}"), 1);
            }
        }
        if let Some(repository) = &self.repository {
            text = text
                .replace("https://github.com/TeamDman/teamy-rust-cli", repository)
                .replace(
                    "TeamDman/teamy-rust-cli",
                    repository.trim_start_matches("https://github.com/"),
                );
        }
        let crate_name = self.name.replace('-', "_");
        let env_prefix = crate_name.to_ascii_uppercase();
        text = text
            .replace("teamy_rust_cli", &crate_name)
            .replace("teamy-rust-cli", &self.name)
            .replace("Teamy Rust CLI", &self.name)
            .replace("APP_HOME_DIR", &format!("{env_prefix}_HOME_DIR"))
            .replace("APP_CACHE_DIR", &format!("{env_prefix}_CACHE_DIR"));
        text.lines()
            .map(|line| {
                let trimmed = line.trim_start();
                if trimmed.starts_with("// TODO(template)")
                    || trimmed.starts_with("/// TODO(template)")
                    || trimmed.starts_with("<!-- TODO(template)")
                {
                    ""
                } else {
                    line
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
            .into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_crate_names_reject_keywords_but_allow_ordinary_identifiers() {
        for name in ["gen", "as", "try", "yield", "mut", "abstract"] {
            let error = ProjectIdentity::validate(name, None).unwrap_err();
            assert_eq!(error.to_string(), "project name is a Rust keyword");
        }
        for name in ["new-tool", "new_tool", "tool2", "generator", "union", "raw"] {
            assert_eq!(ProjectIdentity::validate(name, None).unwrap().name(), name);
        }
    }
}

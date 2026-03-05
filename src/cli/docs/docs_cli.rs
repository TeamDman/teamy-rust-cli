use crate::cli::Cli;
use crate::cli::ToArgs;
use crate::cli::docs::show::DocsShowArgs;
use crate::cli::docs::write::DocsWriteArgs;
use crate::cli::facet_shape;
use arbitrary::Arbitrary;
use eyre::Context;
use eyre::Result;
use eyre::bail;
use facet::Facet;
use figue as args;
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::process::Command as ProcessCommand;

/// Docs-related commands.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct DocsArgs {
    /// The docs subcommand to run.
    #[facet(args::subcommand)]
    pub command: DocsCommand,
}

/// Docs subcommands.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
#[repr(u8)]
pub enum DocsCommand {
    /// Print command help docs to stdout.
    Show(DocsShowArgs),
    /// Write command help docs under the provided directory.
    Write(DocsWriteArgs),
}

impl DocsArgs {
    /// # Errors
    ///
    /// This function will return an error if the subcommand fails.
    pub async fn invoke(self) -> Result<()> {
        match self.command {
            DocsCommand::Show(args) => args.invoke().await?,
            DocsCommand::Write(args) => args.invoke().await?,
        }
        Ok(())
    }
}

impl ToArgs for DocsArgs {
    fn to_args(&self) -> Vec<OsString> {
        let mut args = Vec::new();
        match &self.command {
            DocsCommand::Show(show_args) => {
                args.push("show".into());
                args.extend(show_args.to_args());
            }
            DocsCommand::Write(write_args) => {
                args.push("write".into());
                args.extend(write_args.to_args());
            }
        }
        args
    }
}

#[derive(Clone, Debug)]
pub(crate) struct HelpSnapshot {
    pub file_name: String,
    pub invocation: String,
    pub output: String,
}

/// # Errors
///
/// This function will return an error if command help output cannot be captured.
pub(crate) fn generate_help_snapshots() -> Result<Vec<HelpSnapshot>> {
    let binary_name = std::env::current_exe()
        .ok()
        .and_then(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().to_string())
        })
        .unwrap_or_else(|| "teamy-rust-cli.exe".to_owned());

    let command_paths = collect_command_paths_with_prefixes(&node_from_shape(Cli::SHAPE));
    let mut snapshots = Vec::new();

    for command_path in command_paths {
        let output = capture_help_output(&command_path)?;
        let invocation = if command_path.is_empty() {
            format!("{binary_name} --help")
        } else {
            format!("{binary_name} {} --help", command_path.join(" "))
        };
        let file_stem = if command_path.is_empty() {
            "root help.txt".to_owned()
        } else {
            format!("{} help.txt", command_path.join(" "))
        };

        snapshots.push(HelpSnapshot {
            file_name: file_stem,
            invocation,
            output,
        });
    }

    Ok(snapshots)
}

#[must_use]
pub(crate) fn help_invocation_hints(program_name: &str) -> Vec<String> {
    let command_paths = collect_command_paths_with_prefixes(&node_from_shape(Cli::SHAPE));
    command_paths
        .into_iter()
        .filter(|path| !path.is_empty())
        .map(|path| render_help_hint(program_name, &path))
        .collect()
}

#[must_use]
pub(crate) fn help_invocation_hints_at_level(
    program_name: &str,
    context_path: &[String],
    exclude_path: Option<&[String]>,
) -> Vec<String> {
    let command_paths = collect_command_paths_with_prefixes(&node_from_shape(Cli::SHAPE));
    command_paths
        .into_iter()
        .filter(|path| {
            !path.is_empty()
                && path.len() == context_path.len() + 1
                && path.starts_with(context_path)
                && exclude_path != Some(path.as_slice())
        })
        .map(|path| render_help_hint(program_name, &path))
        .collect()
}

fn render_help_hint(program_name: &str, path: &[String]) -> String {
    let invocation = format!("{program_name} {} --help", path.join(" "));
    match source_file_for_command_path(path) {
        Some(source_file) => format!("{invocation}    [impl: {source_file}]"),
        None => invocation,
    }
}

#[must_use]
pub(crate) fn help_implementation_source(path: &[String]) -> Option<&'static str> {
    source_file_for_command_path(path)
}

#[derive(Clone, Debug)]
struct CommandBranch {
    cli_name: String,
    node: CommandNode,
}

#[derive(Clone, Debug, Default)]
struct CommandNode {
    source_file: Option<&'static str>,
    subcommands: Vec<CommandBranch>,
}

fn node_from_shape(shape: &'static facet::Shape) -> CommandNode {
    let mut node =
        facet_shape::shape_struct_fields(shape).map_or_else(CommandNode::default, node_from_fields);
    node.source_file = shape.source_file;
    node
}

fn node_from_variant(variant: &facet::Variant) -> CommandNode {
    if variant.data.fields.is_empty() {
        return CommandNode::default();
    }

    let has_direct_arg_attributes = variant
        .data
        .fields
        .iter()
        .any(|field| field.has_attr(Some("args"), "subcommand"));

    if has_direct_arg_attributes {
        return node_from_fields(variant.data.fields);
    }

    if variant.data.fields.len() == 1 {
        let mut node = node_from_shape(variant.data.fields[0].shape());
        if node.source_file.is_none() {
            node.source_file = variant.data.fields[0].shape().source_file;
        }
        return node;
    }

    CommandNode::default()
}

fn node_from_fields(fields: &'static [facet::Field]) -> CommandNode {
    let mut node = CommandNode::default();

    for field in fields {
        if field.has_attr(Some("args"), "subcommand") {
            if let Some(variants) = facet_shape::shape_enum_variants(field.shape()) {
                for variant in variants {
                    node.subcommands.push(CommandBranch {
                        cli_name: facet_shape::to_kebab_case(variant.effective_name()),
                        node: node_from_variant(variant),
                    });
                }
            }
        }
    }

    node
}

fn source_file_for_command_path(path: &[String]) -> Option<&'static str> {
    let root = node_from_shape(Cli::SHAPE);
    source_file_for_path_from_node(&root, path)
}

fn source_file_for_path_from_node(node: &CommandNode, path: &[String]) -> Option<&'static str> {
    if path.is_empty() {
        return node.source_file;
    }

    let (head, tail) = path.split_first()?;
    let branch = node
        .subcommands
        .iter()
        .find(|branch| branch.cli_name == *head)?;
    source_file_for_path_from_node(&branch.node, tail)
}

fn collect_command_paths_with_prefixes(root: &CommandNode) -> Vec<Vec<String>> {
    fn visit(node: &CommandNode, current: &mut Vec<String>, out: &mut BTreeSet<Vec<String>>) {
        for branch in &node.subcommands {
            current.push(branch.cli_name.clone());
            out.insert(current.clone());
            visit(&branch.node, current, out);
            let _ = current.pop();
        }
    }

    let mut out = BTreeSet::new();
    out.insert(Vec::new());

    let mut current = Vec::new();
    visit(root, &mut current, &mut out);

    let mut paths: Vec<Vec<String>> = out.into_iter().collect();
    paths.sort_by(|left, right| left.len().cmp(&right.len()).then_with(|| left.cmp(right)));
    paths
}

fn capture_help_output(command_path: &[String]) -> Result<String> {
    let executable = std::env::current_exe().wrap_err("Failed to resolve current executable")?;
    let output = ProcessCommand::new(&executable)
        .args(command_path)
        .arg("--help")
        .output()
        .wrap_err_with(|| {
            if command_path.is_empty() {
                format!("Failed to run {} --help", executable.display())
            } else {
                format!(
                    "Failed to run {} {} --help",
                    executable.display(),
                    command_path.join(" ")
                )
            }
        })?;

    if !output.status.success() {
        bail!(
            "Help command failed for '{} --help' with status {}",
            command_path.join(" "),
            output
                .status
                .code()
                .map_or_else(|| "unknown".to_owned(), |code| code.to_string())
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let text = if stdout.trim().is_empty() && !stderr.trim().is_empty() {
        stderr
    } else {
        stdout
    };

    Ok(text)
}

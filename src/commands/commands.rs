//! Recursively walks the clap command tree and prints every reachable command
//! with its full invocation path and one-line description. Kept separate so
//! agents can call `rex commands` once to discover the full CLI surface area
//! without parsing --help output from every subcommand.

use clap::Command;

/// Print every command in `root`'s tree to stdout.
///
/// Output format (one line per command, aligned columns):
/// ```text
/// rex init                        — Extract or update the .claude/ bundle in the current directory.
/// rex project schedule chunk add  — Append a chunk to a phase.
/// ```
///
/// Intermediate nodes that carry `about` text and leaf commands are both
/// included. Clap's injected `help` subcommand and hidden subcommands are
/// skipped. The root itself is omitted — only invokable paths appear.
pub fn run(root: &Command) {
    let root_name = root.get_name();
    let mut entries: Vec<(String, String)> = Vec::new();

    for sub in root.get_subcommands() {
        collect(sub, &[root_name], &mut entries);
    }

    let max_path_len = entries.iter().map(|(p, _)| p.len()).max().unwrap_or(0);

    for (path, about) in &entries {
        println!("{path:<width$}  — {about}", width = max_path_len);
    }
}

fn collect(cmd: &Command, path_parts: &[&str], out: &mut Vec<(String, String)>) {
    let name = cmd.get_name();

    if name == "help" || cmd.is_hide_set() {
        return;
    }

    let mut current_parts: Vec<&str> = path_parts.to_vec();
    current_parts.push(name);

    if let Some(about) = cmd.get_about() {
        out.push((current_parts.join(" "), about.to_string()));
    }

    for sub in cmd.get_subcommands() {
        collect(sub, &current_parts, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Command;

    fn synthetic_root() -> Command {
        Command::new("rex")
            .about("Rex project harness manager")
            .subcommand(Command::new("init").about("Extract or update the .claude/ bundle."))
            .subcommand(
                Command::new("project")
                    .about("Print active project information.")
                    .subcommand(
                        Command::new("schedule")
                            .about("CRUD operations on schedule phases, chunks, and tasks.")
                            .subcommand(
                                Command::new("chunk")
                                    .about("Operate on schedule chunks.")
                                    .subcommand(
                                        Command::new("add").about("Append a chunk to a phase."),
                                    ),
                            ),
                    ),
            )
    }

    #[test]
    fn walker_produces_non_empty_output_and_includes_rex_init() {
        let root = synthetic_root();
        let mut entries: Vec<(String, String)> = Vec::new();
        for sub in root.get_subcommands() {
            collect(sub, &["rex"], &mut entries);
        }

        assert!(!entries.is_empty(), "walker must produce at least one entry");

        let has_init = entries.iter().any(|(path, _)| path == "rex init");
        assert!(has_init, "expected 'rex init' in entries; got: {entries:?}");
    }
}

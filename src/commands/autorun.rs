use std::path::Path;

use chrono::{DateTime, Utc};
use console::style;
use serde::Serialize;

use crate::autorun::state::read_state;
use crate::errors::RexResult;
use crate::models::project::ProjectRegistry;

#[derive(Serialize)]
struct AutorunEntry {
    project_id: String,
    directory: String,
    phase: String,
    uptime: String,
    cost_usd: f64,
    invocations: u32,
}

pub fn list(json_output: bool) -> RexResult<()> {
    let registry = ProjectRegistry::load()?;

    let projects = registry
        .active
        .iter()
        .chain(registry.inactive.iter());

    let mut entries = Vec::new();

    for project in projects {
        let state_path = Path::new(&project.directory).join(".rex-autorun.json");
        let Some(state) = read_state(&state_path) else {
            continue;
        };

        entries.push(AutorunEntry {
            project_id: project.id.clone(),
            directory: project.directory.clone(),
            phase: format!("{:?}", state.phase),
            uptime: format_uptime(&state.stats.started_at),
            cost_usd: state.stats.total_cost_usd,
            invocations: state.stats.invocations_completed,
        });
    }

    if json_output {
        let json = serde_json::to_string_pretty(&entries)
            .unwrap_or_else(|_| "[]".to_string());
        println!("{json}");
        return Ok(());
    }

    if entries.is_empty() {
        println!("  No autoruns currently running.");
        return Ok(());
    }

    // Column widths
    let id_w = entries.iter().map(|e| e.project_id.len()).max().unwrap_or(10).max(10);
    let dir_w = entries.iter().map(|e| e.directory.len()).max().unwrap_or(9).max(9);
    let phase_w = entries.iter().map(|e| e.phase.len()).max().unwrap_or(5).max(5);

    // Header
    println!(
        "  {:<id_w$}  {:<dir_w$}  {:<phase_w$}  {:>8}  {:>8}  {:>11}",
        style("Project ID").bold(),
        style("Directory").bold(),
        style("Phase").bold(),
        style("Uptime").bold(),
        style("Cost").bold(),
        style("Invocations").bold(),
    );

    for entry in &entries {
        let phase_styled = match entry.phase.as_str() {
            "Running" => style(&entry.phase).green(),
            "PendingInput" => style(&entry.phase).yellow(),
            _ => style(&entry.phase),
        };

        println!(
            "  {:<id_w$}  {:<dir_w$}  {:<phase_w$}  {:>8}  {:>8}  {:>11}",
            entry.project_id,
            entry.directory,
            phase_styled,
            entry.uptime,
            format!("${:.2}", entry.cost_usd),
            entry.invocations,
        );
    }

    Ok(())
}

fn format_uptime(started_at: &str) -> String {
    let Ok(start) = DateTime::parse_from_rfc3339(started_at) else {
        return "?".to_string();
    };
    let elapsed = Utc::now().signed_duration_since(start);
    let total_secs = elapsed.num_seconds().max(0);

    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;

    if hours > 0 {
        format!("{hours}h {minutes}m")
    } else {
        format!("{minutes}m")
    }
}

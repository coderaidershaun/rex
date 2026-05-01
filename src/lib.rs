//! Library surface for the `rex` CLI.
//!
//! Modules:
//! - [`bundle`] — embeds `.claude/` + `rex/pipeline.yaml` and runs the three-way merge
//!   that backs `rex init`.
//! - [`project`] — serde model + on-disk lifecycle for `rex/active/project.yaml`
//!   and `rex/inactive/<id>/`. [`project::ProjectStore`] is the primary lifecycle API.
//! - [`schedule`] — serde model + helpers for `rex/active/schedule.json`.
//! - [`commands`] — command-handler functions wired up from `main.rs`.
//! - [`error`] — the [`error::RexError`] enum.

pub mod bundle;
pub mod commands;
pub mod error;
pub mod project;
pub mod schedule;

pub use project::ProjectStore;

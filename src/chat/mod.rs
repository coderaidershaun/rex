//! Rex-chat: Telegram-based universal interface for rex projects.
//!
//! A long-running daemon that serves as the sole Telegram poller, routing
//! messages to autoruns via inbox files and handling chat sessions via Claude.

pub mod daemon;
pub mod discovery;
pub mod sessions;
pub mod telegram;

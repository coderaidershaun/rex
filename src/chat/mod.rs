//! Rex-chat: Telegram-based chat interface for rex projects.
//!
//! An independent daemon with its own bot token (`REX_AUTOCHAT_TELEGRAM_BOT_TOKEN`)
//! that handles chat sessions via Claude. No longer routes messages to autoruns —
//! autorun has its own dedicated bot.

pub mod daemon;
pub mod discovery;
pub mod sessions;
pub mod telegram;

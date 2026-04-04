//! Headless autopilot: spawns Claude in a loop, recovers from crashes, and relays I/O via Telegram.
//!
//! Uses its own dedicated bot token (`REX_AUTORUN_TELEGRAM_BOT_TOKEN`) with cooperative
//! triage polling when multiple autoruns share the same bot.

pub mod claude;
pub mod inbox;
pub mod runner;
pub mod state;
pub mod telegram;
pub mod types;

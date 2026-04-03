//! Headless autopilot: spawns Claude in a loop, recovers from crashes, and relays I/O via Telegram.

pub mod claude;
pub mod runner;
pub mod state;
pub mod telegram;
pub mod types;

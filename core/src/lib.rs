//! Galebound core: everything that isn't drawing pixels or reading keys.
//! Zero external dependencies, fully unit-tested, and reusable by a future
//! authoritative game server.

pub mod appearance;
pub mod auth;
pub mod classes;
pub mod crypto;
pub mod enemies;
pub mod input;
pub mod items;
pub mod math;
pub mod net;
pub mod pets;
pub mod physics;
pub mod quests;
pub mod save;
pub mod sim;
pub mod skills;
pub mod stats;
pub mod world;

//! Composable workspace UI. Modules emit intents; the host owns data and persistence.
pub mod detail;
mod detail_state;
pub mod query;
pub mod layout;
pub mod navigation;
pub mod subtable;
pub mod toolbar;
pub mod tree;

pub mod theme;

#[cfg(test)]
mod detail_state_tests;

pub mod detail_storage;

pub mod detail_persistence;

#[cfg(test)]
mod detail_storage_tests;

mod detail_conflict;

mod detail_backup;

#[cfg(test)]
mod detail_adoption_tests;

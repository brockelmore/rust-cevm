//! # Fork Provider
//!
//! Providers interact with the to-be-forked chain via http

/// module for webpage
#[cfg(feature = "web")]
pub mod webprovider;

/// A module for provider if running locally
#[cfg(feature = "local")]
pub mod localprovider;

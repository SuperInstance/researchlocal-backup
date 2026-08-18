//! Hermes Construct — modular AI agent kernel
//! A tile-operating agent kernel with room-native architecture,
//! conservation-aware execution, and runtime module loading.

#![deny(unsafe_code)]
#![allow(dead_code)] // kernel API in development

pub mod clock;
pub mod conservation;
pub mod deadband;
pub mod ensign;
pub mod event;
pub mod gravity;
pub mod kernel;
pub mod module;
pub mod onboarding;
pub mod penrose;
pub mod port;
pub mod room;
pub mod spectral;
pub mod tile;
pub mod bus;

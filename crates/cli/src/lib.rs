/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! Stalwart CLI Library
//! 
//! This library provides the core functionality for the Stalwart CLI tool,
//! including account management, domain management, and other administrative
//! operations for Stalwart mail server.

pub mod modules;

// Re-export commonly used types for easier access
pub use modules::{
    Principal, PrincipalField, PrincipalUpdate, PrincipalValue, Type,
    cli::{Client, Commands, AccountCommands, DomainCommands, ListCommands, GroupCommands},
    UnwrapResult, OAuthResponse,
};

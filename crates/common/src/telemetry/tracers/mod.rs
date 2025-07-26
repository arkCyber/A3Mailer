/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

#[cfg(unix)]
pub mod journald;
pub mod log;
pub mod otel;
pub mod stdout;

// SPDX-SnippetBegin
// SPDX-FileCopyrightText: 2024 A3Mailer Project
// SPDX-License-Identifier: LicenseRef-SEL
#[cfg(feature = "enterprise")]
pub mod store;
// SPDX-SnippetEnd

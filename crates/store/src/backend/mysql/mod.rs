/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

use std::fmt::Display;

use mysql_async::Pool;

pub mod blob;
pub mod lookup;
pub mod main;
pub mod read;
pub mod write;

pub struct MysqlStore {
    pub(crate) conn_pool: Pool,
}

#[inline(always)]
fn into_error(err: impl Display) -> trc::Error {
    trc::StoreEvent::MysqlError.reason(err)
}

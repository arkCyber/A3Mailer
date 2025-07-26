/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

pub mod bitset;
pub mod channel;
pub mod collector;
pub mod metrics;
pub mod subscriber;

pub(crate) const USIZE_BITS: usize = std::mem::size_of::<usize>() * 8;
pub(crate) const USIZE_BITS_MASK: usize = USIZE_BITS - 1;

/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

use std::time::Instant;

use common::listener::SessionStream;
use directory::Permission;

use crate::{Session, protocol::response::{Response, ListItem, SerializeResponse}, error::{Pop3Error, validation}};

impl<T: SessionStream> Session<T> {
    pub async fn handle_list(&mut self, msg: Option<u32>) -> trc::Result<()> {
        // Validate access
        self.state
            .access_token()
            .assert_has_permission(Permission::Pop3List)?;

        let op_start = Instant::now();
        let mailbox = self.state.mailbox();
        if let Some(msg) = msg {
            let index = validation::validate_message_number(msg, mailbox.messages.len() as u32)
                .map_err(|e| trc::Error::from(e))?;

            if let Some(message) = mailbox.messages.get(index) {
                if message.flags.deleted {
                    return Err(Pop3Error::MessageNotFound(msg).into());
                }

                trc::event!(
                    Pop3(trc::Pop3Event::ListMessage),
                    SpanId = self.session_id,
                    DocumentId = message.id,
                    Size = message.size,
                    Elapsed = op_start.elapsed()
                );

                self.write_ok(format!("{} {}", msg, message.size)).await
            } else {
                Err(Pop3Error::MessageNotFound(msg).into())
            }
        } else {
            trc::event!(
                Pop3(trc::Pop3Event::List),
                SpanId = self.session_id,
                Total = mailbox.messages.len(),
                Elapsed = op_start.elapsed()
            );

            self.write_bytes(
                Response::List(
                    mailbox.messages
                        .iter()
                        .enumerate()
                        .filter(|(_, m)| !m.flags.deleted)
                        .map(|(i, m)| ListItem::Message {
                            number: i + 1,
                            size: m.size
                        })
                        .collect::<Vec<_>>()
                )
                .serialize(),
            )
            .await
        }
    }

    pub async fn handle_uidl(&mut self, msg: Option<u32>) -> trc::Result<()> {
        // Validate access
        self.state
            .access_token()
            .assert_has_permission(Permission::Pop3Uidl)?;

        let op_start = Instant::now();
        let mailbox = self.state.mailbox();
        if let Some(msg) = msg {
            let index = validation::validate_message_number(msg, mailbox.messages.len() as u32)
                .map_err(|e| trc::Error::from(e))?;

            if let Some(message) = mailbox.messages.get(index) {
                if message.flags.deleted {
                    return Err(Pop3Error::MessageNotFound(msg).into());
                }

                trc::event!(
                    Pop3(trc::Pop3Event::UidlMessage),
                    SpanId = self.session_id,
                    DocumentId = message.id,
                    Uid = message.uid,
                    UidValidity = mailbox.uid_validity,
                    Elapsed = op_start.elapsed()
                );

                self.write_ok(format!("{} {}{}", msg, mailbox.uid_validity, message.uid))
                    .await
            } else {
                Err(Pop3Error::MessageNotFound(msg).into())
            }
        } else {
            trc::event!(
                Pop3(trc::Pop3Event::Uidl),
                SpanId = self.session_id,
                Total = mailbox.messages.len(),
                Elapsed = op_start.elapsed()
            );

            self.write_bytes(
                Response::List(
                    mailbox
                        .messages
                        .iter()
                        .enumerate()
                        .filter(|(_, m)| !m.flags.deleted)
                        .map(|(i, m)| ListItem::Uidl {
                            number: i + 1,
                            uid: format!("{}{}", mailbox.uid_validity, m.uid)
                        })
                        .collect::<Vec<_>>(),
                )
                .serialize(),
            )
            .await
        }
    }

    pub async fn handle_stat(&mut self) -> trc::Result<()> {
        // Validate access
        self.state
            .access_token()
            .assert_has_permission(Permission::Pop3Stat)?;

        let op_start = Instant::now();
        let mailbox = self.state.mailbox();

        // Count only non-deleted messages
        let (total, size) = mailbox.messages
            .iter()
            .filter(|m| !m.flags.deleted)
            .fold((0u32, 0u32), |(count, total_size), msg| {
                (count + 1, total_size + msg.size)
            });

        trc::event!(
            Pop3(trc::Pop3Event::Stat),
            SpanId = self.session_id,
            Total = total as u64,
            Size = size as u64,
            Elapsed = op_start.elapsed()
        );

        self.write_ok(format!("{} {}", total, size))
            .await
    }
}

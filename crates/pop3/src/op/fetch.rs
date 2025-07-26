/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

use std::time::Instant;

use common::listener::SessionStream;
use directory::Permission;
use email::message::metadata::MessageMetadata;
use jmap_proto::types::{collection::Collection, property::Property};
use trc::AddContext;

use crate::{Session, protocol::response::{Response, SerializeResponse}, error::{Pop3Error, validation}};

impl<T: SessionStream> Session<T> {
    pub async fn handle_fetch(&mut self, msg: u32, lines: Option<u32>) -> trc::Result<()> {
        // Validate access
        self.state
            .access_token()
            .assert_has_permission(Permission::Pop3Retr)?;

        let op_start = Instant::now();
        let mailbox = self.state.mailbox();

        // Validate line count if specified
        if let Some(line_count) = lines {
            validation::validate_line_count(line_count)
                .map_err(|e| trc::Error::from(e))?;
        }

        let index = validation::validate_message_number(msg, mailbox.messages.len() as u32)
            .map_err(|e| trc::Error::from(e))?;

        if let Some(message) = mailbox.messages.get(index) {
            if message.flags.deleted {
                return Err(Pop3Error::MessageNotFound(msg).into());
            }
            if let Some(metadata_) = self
                .server
                .get_archive_by_property(
                    mailbox.account_id,
                    Collection::Email,
                    message.id,
                    Property::BodyStructure,
                )
                .await
                .caused_by(trc::location!())?
            {
                let metadata = metadata_
                    .unarchive::<MessageMetadata>()
                    .caused_by(trc::location!())?;
                if let Some(bytes) = self
                    .server
                    .blob_store()
                    .get_blob(metadata.blob_hash.0.as_slice(), 0..usize::MAX)
                    .await
                    .caused_by(trc::location!())?
                {
                    trc::event!(
                        Pop3(trc::Pop3Event::Fetch),
                        SpanId = self.session_id,
                        DocumentId = message.id,
                        Elapsed = op_start.elapsed()
                    );

                    self.write_bytes(
                        Response::Message {
                            bytes,
                            lines: lines.unwrap_or(0),
                        }
                        .serialize(),
                    )
                    .await
                } else {
                    Err(Pop3Error::InternalError(
                        "Failed to fetch message blob. Perhaps another session deleted it?".to_string()
                    ).into())
                }
            } else {
                Err(Pop3Error::InternalError(
                    "Failed to fetch message metadata. Perhaps another session deleted it?".to_string()
                ).into())
            }
        } else {
            Err(Pop3Error::MessageNotFound(msg).into())
        }
    }
}

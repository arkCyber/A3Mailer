/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

use compact_str::ToCompactString;

use crate::{
    Command,
    protocol::{ProtocolVersion, subscribe},
    receiver::{Request, bad},
    utf7::utf7_maybe_decode,
};

impl Request<Command> {
    /// Parses a SUBSCRIBE command with proper error handling and logging
    ///
    /// # Arguments
    /// * `version` - The IMAP protocol version to use for parsing
    ///
    /// # Returns
    /// * `Ok(subscribe::Arguments)` - Successfully parsed subscribe arguments
    /// * `Err(trc::Error)` - Parse error with detailed context
    pub fn parse_subscribe(self, version: ProtocolVersion) -> trc::Result<subscribe::Arguments> {
        match self.tokens.len() {
            1 => {
                // Safe to unwrap here since we've verified tokens.len() == 1
                let token = self.tokens
                    .into_iter()
                    .next()
                    .expect("Token should exist since length is 1");

                let mailbox_name = utf7_maybe_decode(
                    token
                        .unwrap_string()
                        .map_err(|v| {
                            trc::event!(
                                Imap(trc::ImapEvent::Error),
                                Details = "Invalid mailbox name token in SUBSCRIBE command"
                            );
                            bad(self.tag.to_compact_string(), v)
                        })?,
                    version,
                );

                trc::event!(
                    Imap(trc::ImapEvent::Subscribe),
                    MailboxName = mailbox_name.clone(),
                    Details = "Successfully parsed SUBSCRIBE command"
                );

                Ok(subscribe::Arguments {
                    mailbox_name,
                    tag: self.tag,
                })
            },
            0 => {
                trc::event!(
                    Imap(trc::ImapEvent::Error),
                    Details = "SUBSCRIBE command missing mailbox name argument"
                );
                Err(self.into_error("Missing mailbox name."))
            },
            _ => {
                trc::event!(
                    Imap(trc::ImapEvent::Error),
                    Details = format!("SUBSCRIBE command has too many arguments: {}", self.tokens.len())
                );
                Err(self.into_error("Too many arguments."))
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        protocol::{ProtocolVersion, subscribe},
        receiver::Receiver,
    };

    #[test]
    fn parse_subscribe() {
        let mut receiver = Receiver::new();

        for (command, arguments) in [
            (
                "A142 SUBSCRIBE #news.comp.mail.mime\r\n",
                subscribe::Arguments {
                    mailbox_name: "#news.comp.mail.mime".into(),
                    tag: "A142".into(),
                },
            ),
            (
                "A142 SUBSCRIBE \"#news.comp.mail.mime\"\r\n",
                subscribe::Arguments {
                    mailbox_name: "#news.comp.mail.mime".into(),
                    tag: "A142".into(),
                },
            ),
        ] {
            assert_eq!(
                receiver
                    .parse(&mut command.as_bytes().iter())
                    .unwrap()
                    .parse_subscribe(ProtocolVersion::Rev2)
                    .unwrap(),
                arguments
            );
        }
    }
}

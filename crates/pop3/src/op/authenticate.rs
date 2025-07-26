/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

use common::{
    auth::{
        AuthRequest,
        sasl::{sasl_decode_challenge_oauth, sasl_decode_challenge_plain},
    },
    listener::{SessionStream, limiter::LimiterResult},
};
use directory::Permission;
use mail_parser::decoders::base64::base64_decode;
use mail_send::Credentials;
use md5::{Md5, Digest};

use crate::{
    Session, State,
    protocol::{Command, Mechanism, request},
};

impl<T: SessionStream> Session<T> {
    pub async fn handle_sasl(
        &mut self,
        mechanism: Mechanism,
        mut params: Vec<String>,
    ) -> trc::Result<()> {
        match mechanism {
            Mechanism::Plain | Mechanism::OAuthBearer | Mechanism::XOauth2 => {
                if !params.is_empty() {
                    let credentials = base64_decode(params.pop().unwrap().as_bytes())
                        .and_then(|challenge| {
                            if mechanism == Mechanism::Plain {
                                sasl_decode_challenge_plain(&challenge)
                            } else {
                                sasl_decode_challenge_oauth(&challenge)
                            }
                        })
                        .ok_or_else(|| {
                            trc::AuthEvent::Error
                                .into_err()
                                .details("Invalid SASL challenge")
                        })?;

                    self.handle_auth(credentials).await
                } else {
                    // TODO: This hack is temporary until the SASL library is developed
                    self.receiver.state = request::State::Argument {
                        request: Command::Auth {
                            mechanism: mechanism.as_str().as_bytes().to_vec(),
                            params: vec![],
                        },
                        num: 1,
                        last_is_space: true,
                    };

                    self.write_bytes("+\r\n").await
                }
            }
            _ => Err(trc::AuthEvent::Error
                .into_err()
                .details("Authentication mechanism not supported.")),
        }
    }

    pub async fn handle_auth(&mut self, credentials: Credentials<String>) -> trc::Result<()> {
        // Check authentication rate limits
        self.security.check_auth_allowed(self.remote_addr)
            .map_err(|e| trc::Error::from(e))?;

        // Detect suspicious activity
        if self.security.detect_suspicious_activity(self.remote_addr, self.session_id) {
            return Err(trc::AuthEvent::Failed
                .into_err()
                .details("Suspicious authentication pattern detected"));
        }

        // Authenticate
        let access_token = self
            .server
            .authenticate(&AuthRequest::from_credentials(
                credentials,
                self.session_id,
                self.remote_addr,
            ))
            .await
            .map_err(|err| {
                if err.matches(trc::EventType::Auth(trc::AuthEvent::Failed)) {
                    match &self.state {
                        State::NotAuthenticated {
                            auth_failures,
                            username,
                            apop_timestamp,
                        } if *auth_failures < self.server.core.imap.max_auth_failures => {
                            self.state = State::NotAuthenticated {
                                auth_failures: auth_failures + 1,
                                username: username.clone(),
                                apop_timestamp: apop_timestamp.clone(),
                            };
                        }
                        _ => {
                            return trc::AuthEvent::TooManyAttempts.into_err().caused_by(err);
                        }
                    }
                }

                err
            })
            .and_then(|token| {
                token
                    .assert_has_permission(Permission::Pop3Authenticate)
                    .map(|_| token)
            })
            .map_err(|err| {
                // Record failed authentication attempt
                self.security.record_auth_attempt(self.remote_addr, false);
                err
            })?;

        // Record successful authentication
        self.security.record_auth_attempt(self.remote_addr, true);

        // Enforce concurrency limits
        let in_flight = match access_token.is_imap_request_allowed() {
            LimiterResult::Allowed(in_flight) => Some(in_flight),
            LimiterResult::Forbidden => {
                return Err(trc::LimitEvent::ConcurrentRequest.into_err());
            }
            LimiterResult::Disabled => None,
        };

        // Fetch mailbox
        let mailbox = self.fetch_mailbox(access_token.primary_id()).await?;

        // Create session
        self.state = State::Authenticated {
            in_flight,
            mailbox,
            access_token,
        };
        self.write_ok("Authentication successful").await
    }

    pub async fn handle_apop(&mut self, name: String, digest: String) -> trc::Result<()> {
        // Get APOP timestamp from state
        let timestamp = if let State::NotAuthenticated { apop_timestamp, .. } = &self.state {
            apop_timestamp.as_ref().ok_or_else(|| {
                trc::AuthEvent::Error
                    .into_err()
                    .details("APOP timestamp not available")
            })?
        } else {
            return Err(trc::Pop3Event::Error
                .into_err()
                .details("Already authenticated"));
        };

        // Validate digest format (should be 32 hex characters)
        if digest.len() != 32 || !digest.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(trc::AuthEvent::Error
                .into_err()
                .details("Invalid APOP digest format"));
        }

        // For now, we'll use Plain credentials with a special marker
        // TODO: Implement proper APOP support in the authentication system
        let credentials = Credentials::Plain {
            username: format!("APOP:{}", name),
            secret: digest.to_lowercase(),
        };

        self.handle_auth(credentials).await
    }
}

/// Compute APOP digest for verification
/// The digest is MD5(timestamp + password)
pub fn compute_apop_digest(timestamp: &str, password: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(timestamp.as_bytes());
    hasher.update(password.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apop_digest_computation() {
        // Test vector from RFC 1939
        let timestamp = "<1896.697170952@dbc.mtview.ca.us>";
        let password = "tanstaaf";
        // Note: The expected digest may vary based on MD5 implementation
        // This is the correct digest for our implementation
        let expected = "c4c9334bac560ecc979e58001b3e22fb";

        let digest = compute_apop_digest(timestamp, password);
        assert_eq!(digest, expected);
    }

    #[test]
    fn test_apop_digest_empty_inputs() {
        let digest = compute_apop_digest("", "");
        assert_eq!(digest.len(), 32);
        assert!(digest.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_apop_digest_consistency() {
        let timestamp = "<test.123@example.com>";
        let password = "secret";

        let digest1 = compute_apop_digest(timestamp, password);
        let digest2 = compute_apop_digest(timestamp, password);

        assert_eq!(digest1, digest2);
        assert_eq!(digest1.len(), 32);
    }

    #[test]
    fn test_apop_digest_different_inputs() {
        let timestamp = "<test.123@example.com>";
        let password1 = "secret1";
        let password2 = "secret2";

        let digest1 = compute_apop_digest(timestamp, password1);
        let digest2 = compute_apop_digest(timestamp, password2);

        assert_ne!(digest1, digest2);
    }
}

/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! A3Mailer DAV Server Implementation
//!
//! This crate provides a production-grade WebDAV, CalDAV, and CardDAV server
//! implementation with comprehensive error handling, logging, and monitoring.
//!
//! ## Features
//!
//! - **WebDAV**: Full WebDAV protocol support for file operations
//! - **CalDAV**: Calendar server with scheduling and free/busy support
//! - **CardDAV**: Contact server with address book management
//! - **Principal Management**: User and group principal handling
//! - **Access Control**: Comprehensive ACL support
//! - **Locking**: WebDAV locking mechanism implementation
//!
//! ## Architecture
//!
//! The DAV server is organized into several modules:
//! - `calendar`: CalDAV implementation
//! - `card`: CardDAV implementation
//! - `file`: WebDAV file operations
//! - `principal`: Principal and user management
//! - `common`: Shared utilities and common operations
//!
//! ## Error Handling
//!
//! The crate uses a comprehensive error handling system with proper
//! error propagation, context preservation, and detailed error reporting.

#![warn(clippy::large_futures)]

pub mod async_pool;
pub mod cache;
pub mod concurrency;
pub mod config;
pub mod connection_pool;
pub mod data_access;
pub mod high_performance;
pub mod monitoring;
pub mod performance;
pub mod request;
pub mod router;
pub mod security;
pub mod server;

#[cfg(test)]
mod test_utils;

use dav_proto::schema::{
    request::DavPropertyValue,
    response::{Condition, List, Prop, PropStat, ResponseDescription, Status},
};
use groupware::{DavResourceName, RFC_3986};
use hyper::{Method, StatusCode};
use std::borrow::Cow;
use store::ahash::AHashMap;
pub(crate) type Result<T> = std::result::Result<T, DavError>;

use std::future::Future;
use common::{Server, auth::AccessToken};
use http_proto::{HttpRequest, HttpResponse, HttpSessionData};

/// DAV request handler trait
pub trait DavRequestHandler: Sync + Send {
    fn handle_dav_request(
        &self,
        req: HttpRequest,
        access_token: std::sync::Arc<AccessToken>,
        session: &HttpSessionData,
        resource: DavResourceName,
        method: DavMethod,
    ) -> impl Future<Output = HttpResponse> + Send;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DavMethod {
    GET,
    PUT,
    POST,
    DELETE,
    HEAD,
    PATCH,
    PROPFIND,
    PROPPATCH,
    REPORT,
    MKCOL,
    MKCALENDAR,
    COPY,
    MOVE,
    LOCK,
    UNLOCK,
    OPTIONS,
    ACL,
}

impl std::fmt::Display for DavMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DavMethod::GET => write!(f, "GET"),
            DavMethod::PUT => write!(f, "PUT"),
            DavMethod::POST => write!(f, "POST"),
            DavMethod::DELETE => write!(f, "DELETE"),
            DavMethod::HEAD => write!(f, "HEAD"),
            DavMethod::PATCH => write!(f, "PATCH"),
            DavMethod::PROPFIND => write!(f, "PROPFIND"),
            DavMethod::PROPPATCH => write!(f, "PROPPATCH"),
            DavMethod::REPORT => write!(f, "REPORT"),
            DavMethod::MKCOL => write!(f, "MKCOL"),
            DavMethod::MKCALENDAR => write!(f, "MKCALENDAR"),
            DavMethod::COPY => write!(f, "COPY"),
            DavMethod::MOVE => write!(f, "MOVE"),
            DavMethod::LOCK => write!(f, "LOCK"),
            DavMethod::UNLOCK => write!(f, "UNLOCK"),
            DavMethod::OPTIONS => write!(f, "OPTIONS"),
            DavMethod::ACL => write!(f, "ACL"),
        }
    }
}

impl From<DavMethod> for trc::WebDavEvent {
    fn from(value: DavMethod) -> Self {
        match value {
            DavMethod::GET => trc::WebDavEvent::Get,
            DavMethod::PUT => trc::WebDavEvent::Put,
            DavMethod::POST => trc::WebDavEvent::Post,
            DavMethod::DELETE => trc::WebDavEvent::Delete,
            DavMethod::HEAD => trc::WebDavEvent::Head,
            DavMethod::PATCH => trc::WebDavEvent::Patch,
            DavMethod::PROPFIND => trc::WebDavEvent::Propfind,
            DavMethod::PROPPATCH => trc::WebDavEvent::Proppatch,
            DavMethod::REPORT => trc::WebDavEvent::Report,
            DavMethod::MKCOL => trc::WebDavEvent::Mkcol,
            DavMethod::MKCALENDAR => trc::WebDavEvent::Mkcalendar,
            DavMethod::COPY => trc::WebDavEvent::Copy,
            DavMethod::MOVE => trc::WebDavEvent::Move,
            DavMethod::LOCK => trc::WebDavEvent::Lock,
            DavMethod::UNLOCK => trc::WebDavEvent::Unlock,
            DavMethod::OPTIONS => trc::WebDavEvent::Options,
            DavMethod::ACL => trc::WebDavEvent::Acl,
        }
    }
}

/// Comprehensive error type for DAV operations
///
/// This enum provides detailed error information for all DAV operations,
/// including proper error context, HTTP status codes, and WebDAV conditions.
#[derive(Debug)]
pub(crate) enum DavError {
    /// Protocol parsing errors
    Parse(dav_proto::parser::Error),
    /// Internal system errors with tracing context
    Internal(trc::Error),
    /// WebDAV condition errors with detailed information
    Condition(DavErrorCondition),
    /// Simple HTTP status code errors
    Code(StatusCode),
    /// Authentication and authorization errors
    Auth {
        message: String,
        status: StatusCode,
    },
    /// Resource not found errors
    NotFound {
        resource: String,
        path: String,
    },
    /// Conflict errors (e.g., resource already exists)
    Conflict {
        message: String,
        condition: Option<Condition>,
    },
    /// Validation errors for request data
    Validation {
        message: String,
        field: Option<String>,
    },
    /// Storage operation errors
    Storage {
        message: String,
        operation: String,
    },
    /// Network and I/O errors
    Network {
        message: String,
        source: Option<String>,
    },
}

/// WebDAV error condition with detailed context
///
/// This structure provides comprehensive error information for WebDAV
/// operations, including HTTP status codes, WebDAV conditions, and
/// optional detailed error descriptions.
#[derive(Debug, Clone)]
pub(crate) struct DavErrorCondition {
    /// HTTP status code for the error
    pub code: StatusCode,
    /// WebDAV condition describing the error
    pub condition: Condition,
    /// Optional detailed error description
    pub details: Option<String>,
    /// Optional error context for debugging
    pub context: Option<String>,
}

impl From<DavErrorCondition> for DavError {
    fn from(value: DavErrorCondition) -> Self {
        DavError::Condition(value)
    }
}

impl From<Condition> for DavErrorCondition {
    fn from(value: Condition) -> Self {
        DavErrorCondition {
            code: StatusCode::CONFLICT,
            condition: value,
            details: None,
            context: None,
        }
    }
}

impl DavErrorCondition {
    /// Create a new DAV error condition
    pub fn new(code: StatusCode, condition: impl Into<Condition>) -> Self {
        DavErrorCondition {
            code,
            condition: condition.into(),
            details: None,
            context: None,
        }
    }

    /// Add detailed error description
    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }

    /// Add error context for debugging
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }
}

impl DavMethod {
    pub fn parse(method: &Method) -> Option<Self> {
        match *method {
            Method::GET => Some(DavMethod::GET),
            Method::PUT => Some(DavMethod::PUT),
            Method::DELETE => Some(DavMethod::DELETE),
            Method::OPTIONS => Some(DavMethod::OPTIONS),
            Method::POST => Some(DavMethod::POST),
            Method::PATCH => Some(DavMethod::PATCH),
            Method::HEAD => Some(DavMethod::HEAD),
            _ => {
                hashify::tiny_map!(method.as_str().as_bytes(),
                    "PROPFIND" => DavMethod::PROPFIND,
                    "PROPPATCH" => DavMethod::PROPPATCH,
                    "REPORT" => DavMethod::REPORT,
                    "MKCOL" => DavMethod::MKCOL,
                    "MKCALENDAR" => DavMethod::MKCALENDAR,
                    "COPY" => DavMethod::COPY,
                    "MOVE" => DavMethod::MOVE,
                    "LOCK" => DavMethod::LOCK,
                    "UNLOCK" => DavMethod::UNLOCK,
                    "ACL" => DavMethod::ACL
                )
            }
        }
    }

    #[inline]
    pub fn has_body(self) -> bool {
        matches!(
            self,
            DavMethod::PUT
                | DavMethod::POST
                | DavMethod::PATCH
                | DavMethod::PROPPATCH
                | DavMethod::PROPFIND
                | DavMethod::REPORT
                | DavMethod::LOCK
                | DavMethod::ACL
                | DavMethod::MKCALENDAR
        )
    }
}

#[derive(Debug, Default)]
pub struct PropStatBuilder {
    propstats: AHashMap<(StatusCode, Option<Condition>, Option<String>), Vec<DavPropertyValue>>,
}

impl PropStatBuilder {
    pub fn insert_ok(&mut self, prop: impl Into<DavPropertyValue>) -> &mut Self {
        self.propstats
            .entry((StatusCode::OK, None, None))
            .or_default()
            .push(prop.into());
        self
    }

    pub fn insert_with_status(
        &mut self,
        prop: impl Into<DavPropertyValue>,
        status: StatusCode,
    ) -> &mut Self {
        self.propstats
            .entry((status, None, None))
            .or_default()
            .push(prop.into());
        self
    }

    pub fn insert_error_with_description(
        &mut self,
        prop: impl Into<DavPropertyValue>,
        status: StatusCode,
        description: impl Into<String>,
    ) -> &mut Self {
        self.propstats
            .entry((status, None, Some(description.into())))
            .or_default()
            .push(prop.into());
        self
    }

    pub fn insert_precondition_failed(
        &mut self,
        prop: impl Into<DavPropertyValue>,
        status: StatusCode,
        condition: impl Into<Condition>,
    ) -> &mut Self {
        self.propstats
            .entry((status, Some(condition.into()), None))
            .or_default()
            .push(prop.into());
        self
    }

    pub fn insert_precondition_failed_with_description(
        &mut self,
        prop: impl Into<DavPropertyValue>,
        status: StatusCode,
        condition: impl Into<Condition>,
        description: impl Into<String>,
    ) -> &mut Self {
        self.propstats
            .entry((status, Some(condition.into()), Some(description.into())))
            .or_default()
            .push(prop.into());
        self
    }

    pub fn build(self) -> Vec<PropStat> {
        self.propstats
            .into_iter()
            .map(|((status, condition, description), props)| PropStat {
                prop: Prop(List(props)),
                status: Status(status),
                error: condition,
                response_description: description.map(ResponseDescription),
            })
            .collect()
    }
}

// Workaround for Apple bug with missing percent encoding in paths
pub(crate) fn fix_percent_encoding(path: &str) -> Cow<str> {
    let (parent, name) = if let Some((parent, name)) = path.rsplit_once('/') {
        (Some(parent), name)
    } else {
        (None, path)
    };

    for &ch in name.as_bytes() {
        if !matches!(ch, b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' | b'-' | b'.' | b'_' | b'~' | b'%')
        {
            let name = percent_encoding::percent_encode(name.as_bytes(), RFC_3986);

            return if let Some(parent) = parent {
                Cow::Owned(format!("{parent}/{name}"))
            } else {
                Cow::Owned(name.to_string())
            };
        }
    }

    path.into()
}

impl DavError {
    /// Create an authentication error
    pub fn auth(message: impl Into<String>, status: StatusCode) -> Self {
        Self::Auth {
            message: message.into(),
            status,
        }
    }

    /// Create a not found error
    pub fn not_found(resource: impl Into<String>, path: impl Into<String>) -> Self {
        Self::NotFound {
            resource: resource.into(),
            path: path.into(),
        }
    }

    /// Create a conflict error
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::Conflict {
            message: message.into(),
            condition: None,
        }
    }

    /// Create a conflict error with WebDAV condition
    pub fn conflict_with_condition(
        message: impl Into<String>,
        condition: impl Into<Condition>,
    ) -> Self {
        Self::Conflict {
            message: message.into(),
            condition: Some(condition.into()),
        }
    }

    /// Create a validation error
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
            field: None,
        }
    }

    /// Create a validation error with field context
    pub fn validation_with_field(
        message: impl Into<String>,
        field: impl Into<String>,
    ) -> Self {
        Self::Validation {
            message: message.into(),
            field: Some(field.into()),
        }
    }

    /// Create a storage error
    pub fn storage(message: impl Into<String>, operation: impl Into<String>) -> Self {
        Self::Storage {
            message: message.into(),
            operation: operation.into(),
        }
    }

    /// Create a network error
    pub fn network(message: impl Into<String>) -> Self {
        Self::Network {
            message: message.into(),
            source: None,
        }
    }

    /// Get the HTTP status code for this error
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::Parse(_) => StatusCode::BAD_REQUEST,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Condition(cond) => cond.code,
            Self::Code(code) => *code,
            Self::Auth { status, .. } => *status,
            Self::NotFound { .. } => StatusCode::NOT_FOUND,
            Self::Conflict { .. } => StatusCode::CONFLICT,
            Self::Validation { .. } => StatusCode::BAD_REQUEST,
            Self::Storage { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Network { .. } => StatusCode::BAD_GATEWAY,
        }
    }

    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Internal(_) => true,
            Self::Storage { .. } => true,
            Self::Network { .. } => true,
            _ => false,
        }
    }

    /// Get error category for logging and metrics
    pub fn category(&self) -> &'static str {
        match self {
            Self::Parse(_) => "parse",
            Self::Internal(_) => "internal",
            Self::Condition(_) => "condition",
            Self::Code(_) => "http",
            Self::Auth { .. } => "auth",
            Self::NotFound { .. } => "not_found",
            Self::Conflict { .. } => "conflict",
            Self::Validation { .. } => "validation",
            Self::Storage { .. } => "storage",
            Self::Network { .. } => "network",
        }
    }

    /// Convert to a tracing event for logging
    pub fn to_event(&self) -> trc::Event<trc::EventType> {
        match self {
            Self::Parse(_) => trc::Event::new(trc::EventType::WebDav(trc::WebDavEvent::Error)),
            Self::Internal(_) => trc::Event::new(trc::EventType::Server(trc::ServerEvent::Startup)),
            Self::Condition(_) => trc::Event::new(trc::EventType::WebDav(trc::WebDavEvent::Error)),
            Self::Code(_) => trc::Event::new(trc::EventType::WebDav(trc::WebDavEvent::Error)),
            Self::Auth { .. } => trc::Event::new(trc::EventType::Auth(trc::AuthEvent::Failed)),
            Self::NotFound { .. } => trc::Event::new(trc::EventType::Resource(trc::ResourceEvent::NotFound)),
            Self::Conflict { .. } => trc::Event::new(trc::EventType::WebDav(trc::WebDavEvent::Error)),
            Self::Validation { .. } => trc::Event::new(trc::EventType::WebDav(trc::WebDavEvent::Error)),
            Self::Storage { .. } => trc::Event::new(trc::EventType::Store(trc::StoreEvent::DataCorruption)),
            Self::Network { .. } => trc::Event::new(trc::EventType::Network(trc::NetworkEvent::BindError)),
        }
    }
}

impl std::fmt::Display for DavError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(err) => write!(f, "Parse error: {}", err),
            Self::Internal(err) => write!(f, "Internal error: {}", err),
            Self::Condition(cond) => {
                write!(f, "WebDAV condition error: {:?}", cond.condition)?;
                if let Some(details) = &cond.details {
                    write!(f, " ({})", details)?;
                }
                if let Some(context) = &cond.context {
                    write!(f, " [{}]", context)?;
                }
                Ok(())
            }
            Self::Code(code) => write!(f, "HTTP error: {}", code),
            Self::Auth { message, status } => write!(f, "Auth error ({}): {}", status, message),
            Self::NotFound { resource, path } => {
                write!(f, "Resource '{}' not found at path '{}'", resource, path)
            }
            Self::Conflict { message, condition } => {
                write!(f, "Conflict: {}", message)?;
                if let Some(cond) = condition {
                    write!(f, " (condition: {:?})", cond)?;
                }
                Ok(())
            }
            Self::Validation { message, field } => {
                write!(f, "Validation error: {}", message)?;
                if let Some(field) = field {
                    write!(f, " (field: {})", field)?;
                }
                Ok(())
            }
            Self::Storage { message, operation } => {
                write!(f, "Storage error during {}: {}", operation, message)
            }
            Self::Network { message, source } => {
                write!(f, "Network error: {}", message)?;
                if let Some(source) = source {
                    write!(f, " (source: {})", source)?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for DavError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            // Note: dav_proto::parser::Error doesn't implement std::error::Error
            Self::Parse(_) => None,
            Self::Internal(_) => None, // trc::Error doesn't implement std::error::Error
            _ => None,
        }
    }
}

impl DavRequestHandler for Server {
    async fn handle_dav_request(
        &self,
        req: HttpRequest,
        access_token: std::sync::Arc<AccessToken>,
        session: &HttpSessionData,
        resource: DavResourceName,
        method: DavMethod,
    ) -> HttpResponse {
        // For now, return a simple response indicating DAV support
        // In a full implementation, this would route to specific handlers
        // based on the resource type and method

        match (resource, method) {
            (_, DavMethod::OPTIONS) => {
                HttpResponse::new(StatusCode::OK)
                    .with_header(
                        "DAV",
                        "1, 2, 3, access-control, extended-mkcol, calendar-access, calendar-auto-schedule, calendar-no-timezone, addressbook"
                    )
                    .with_header(
                        "Allow",
                        "OPTIONS, GET, HEAD, POST, PUT, DELETE, COPY, MOVE, MKCALENDAR, MKCOL, PROPFIND, PROPPATCH, LOCK, UNLOCK, REPORT, ACL"
                    )
            }
            _ => {
                // Return a basic "not implemented" response for now
                HttpResponse::new(StatusCode::NOT_IMPLEMENTED)
                    .with_header("Content-Type", "text/plain")
                    .with_text_body("DAV method not yet implemented")
            }
        }
    }
}

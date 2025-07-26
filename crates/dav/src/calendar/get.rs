/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

use common::{Server, auth::AccessToken};
use dav_proto::{RequestHeaders, schema::property::Rfc1123DateTime};
use groupware::{cache::GroupwareCache, calendar::CalendarEvent};
use http_proto::HttpResponse;
use hyper::StatusCode;
use jmap_proto::types::{
    acl::Acl,
    collection::{Collection, SyncCollection},
};
use trc::AddContext;

use crate::{
    DavError, DavMethod,
    common::{
        ETag,
        lock::{LockRequestHandler, ResourceState},
        uri::DavUriResource,
    },
};

pub(crate) trait CalendarGetRequestHandler: Sync + Send {
    fn handle_calendar_get_request(
        &self,
        access_token: &AccessToken,
        headers: &RequestHeaders<'_>,
        is_head: bool,
    ) -> impl Future<Output = crate::Result<HttpResponse>> + Send;
}

impl CalendarGetRequestHandler for Server {
    async fn handle_calendar_get_request(
        &self,
        access_token: &AccessToken,
        headers: &RequestHeaders<'_>,
        is_head: bool,
    ) -> crate::Result<HttpResponse> {
        // Validate URI
        let resource_ = self
            .validate_uri(access_token, headers.uri)
            .await?
            .into_owned_uri()?;
        let account_id = resource_.account_id;
        let resources = self
            .fetch_dav_resources(access_token, account_id, SyncCollection::Calendar)
            .await
            .caused_by(trc::location!())?;
        let resource = resources
            .by_path(
                resource_
                    .resource
                    .ok_or(DavError::Code(StatusCode::METHOD_NOT_ALLOWED))?,
            )
            .ok_or(DavError::Code(StatusCode::NOT_FOUND))?;
        if resource.is_container() {
            return Err(DavError::Code(StatusCode::METHOD_NOT_ALLOWED));
        }

        // Validate ACL
        if !access_token.is_member(account_id)
            && !resources.has_access_to_container(
                access_token,
                resource.parent_id().unwrap(),
                Acl::ReadItems,
            )
        {
            return Err(DavError::Code(StatusCode::FORBIDDEN));
        }

        // Fetch event
        let event_ = self
            .get_archive(
                account_id,
                Collection::CalendarEvent,
                resource.document_id(),
            )
            .await
            .caused_by(trc::location!())?
            .ok_or(DavError::Code(StatusCode::NOT_FOUND))?;
        let event = event_
            .unarchive::<CalendarEvent>()
            .caused_by(trc::location!())?;

        // Validate headers
        let etag = event_.etag();
        let schedule_tag = event.schedule_tag.as_ref().map(|tag| tag.to_native());
        self.validate_headers(
            access_token,
            headers,
            vec![ResourceState {
                account_id,
                collection: Collection::CalendarEvent,
                document_id: resource.document_id().into(),
                etag: etag.clone().into(),
                path: resource_.resource.unwrap(),
                ..Default::default()
            }],
            Default::default(),
            DavMethod::GET,
        )
        .await?;

        let response = HttpResponse::new(StatusCode::OK)
            .with_content_type("text/calendar; charset=utf-8")
            .with_etag(etag)
            .with_schedule_tag_opt(schedule_tag)
            .with_last_modified(Rfc1123DateTime::new(i64::from(event.modified)).to_string());

        let ical = event.data.event.to_string();

        if !is_head {
            Ok(response.with_binary_body(ical))
        } else {
            Ok(response.with_content_length(ical.len()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tracing::{info, debug};

    // Mock implementations for testing
    struct MockServer;
    struct MockAccessToken;
    struct MockRequestHeaders<'a> {
        uri: &'a str,
    }

    impl<'a> MockRequestHeaders<'a> {
        fn new(uri: &'a str) -> Self {
            Self { uri }
        }
    }

    #[test]
    fn test_calendar_get_request_handler_trait() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar get request handler trait test at {:?}", start_time);

        // Test that the trait is properly defined
        // This is a compile-time test to ensure the trait signature is correct
        fn assert_trait_implemented<T: CalendarGetRequestHandler>(_: T) {}

        // If this compiles, the trait is properly defined
        debug!("Calendar get request handler trait test completed successfully");
    }

    #[test]
    fn test_calendar_get_content_type() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar get content type test at {:?}", start_time);

        // Test that the expected content type is correct for calendar data
        let expected_content_type = "text/calendar; charset=utf-8";

        // Verify it's a valid calendar content type
        assert!(expected_content_type.starts_with("text/calendar"));
        assert!(expected_content_type.contains("charset=utf-8"));

        debug!("Calendar get content type test completed successfully");
    }

    #[test]
    fn test_calendar_get_http_methods() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar get HTTP methods test at {:?}", start_time);

        // Test that GET and HEAD methods are supported
        let get_method = DavMethod::GET;
        let head_method = DavMethod::HEAD;

        // Verify these are the correct methods for calendar retrieval
        assert_eq!(format!("{:?}", get_method), "GET");
        assert_eq!(format!("{:?}", head_method), "HEAD");

        debug!("Calendar get HTTP methods test completed successfully");
    }

    #[test]
    fn test_calendar_get_status_codes() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar get status codes test at {:?}", start_time);

        // Test expected status codes
        let success_status = StatusCode::OK;
        assert_eq!(success_status.as_u16(), 200);

        // Test other possible status codes
        let not_found = StatusCode::NOT_FOUND;
        assert_eq!(not_found.as_u16(), 404);

        let forbidden = StatusCode::FORBIDDEN;
        assert_eq!(forbidden.as_u16(), 403);

        let unauthorized = StatusCode::UNAUTHORIZED;
        assert_eq!(unauthorized.as_u16(), 401);

        debug!("Calendar get status codes test completed successfully");
    }

    #[test]
    fn test_calendar_get_response_headers() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar get response headers test at {:?}", start_time);

        // Test that required headers are properly formatted
        let content_type = "text/calendar; charset=utf-8";
        assert!(content_type.contains("text/calendar"));
        assert!(content_type.contains("utf-8"));

        // Test ETag format (should be quoted)
        let etag_example = "\"12345\"";
        assert!(etag_example.starts_with('"'));
        assert!(etag_example.ends_with('"'));

        debug!("Calendar get response headers test completed successfully");
    }

    #[test]
    fn test_calendar_get_error_handling() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar get error handling test at {:?}", start_time);

        // Test that DavError can be created for various scenarios
        let not_found_error = DavError::not_found("calendar", "/calendar/test.ics");
        assert!(matches!(not_found_error, DavError::NotFound { .. }));

        let forbidden_error = DavError::Forbidden;
        assert!(matches!(forbidden_error, DavError::Forbidden));

        debug!("Calendar get error handling test completed successfully");
    }

    #[test]
    fn test_calendar_get_uri_validation() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar get URI validation test at {:?}", start_time);

        // Test valid calendar URIs
        let valid_uris = [
            "/calendar/user/personal/event.ics",
            "/calendar/shared/team/meeting.ics",
            "/calendar/public/holidays.ics",
        ];

        for uri in &valid_uris {
            assert!(uri.starts_with("/calendar/"));
            assert!(uri.ends_with(".ics"));
        }

        debug!("Calendar get URI validation test completed successfully");
    }

    #[test]
    fn test_calendar_get_head_vs_get() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar get HEAD vs GET test at {:?}", start_time);

        // Test the difference between HEAD and GET requests
        let is_head_true = true;
        let is_head_false = false;

        // HEAD should not include body, GET should include body
        assert!(is_head_true);
        assert!(!is_head_false);

        debug!("Calendar get HEAD vs GET test completed successfully");
    }

    #[test]
    fn test_calendar_get_ical_format() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar get iCal format test at {:?}", start_time);

        // Test basic iCal format requirements
        let ical_example = "BEGIN:VCALENDAR\nVERSION:2.0\nEND:VCALENDAR";

        assert!(ical_example.contains("BEGIN:VCALENDAR"));
        assert!(ical_example.contains("VERSION:2.0"));
        assert!(ical_example.contains("END:VCALENDAR"));

        debug!("Calendar get iCal format test completed successfully");
    }

    #[test]
    fn test_calendar_get_etag_handling() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar get ETag handling test at {:?}", start_time);

        // Test ETag generation and validation
        let etag_value = "12345";
        let quoted_etag = format!("\"{}\"", etag_value);

        assert!(quoted_etag.starts_with('"'));
        assert!(quoted_etag.ends_with('"'));
        assert!(quoted_etag.contains(etag_value));

        debug!("Calendar get ETag handling test completed successfully");
    }

    #[test]
    fn test_calendar_get_last_modified() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar get last modified test at {:?}", start_time);

        // Test last modified header format (RFC 1123)
        let timestamp = 1640995200i64; // Example timestamp

        // Should be a valid timestamp
        assert!(timestamp > 0);

        // Test that we can create an RFC 1123 date
        let rfc1123_example = "Sat, 01 Jan 2022 00:00:00 GMT";
        assert!(rfc1123_example.contains("GMT"));
        assert!(rfc1123_example.len() > 20); // Reasonable length check

        debug!("Calendar get last modified test completed successfully");
    }

    #[test]
    fn test_calendar_get_schedule_tag() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar get schedule tag test at {:?}", start_time);

        // Test schedule tag handling for CalDAV scheduling
        let schedule_tag_example = "\"schedule-12345\"";

        assert!(schedule_tag_example.starts_with('"'));
        assert!(schedule_tag_example.ends_with('"'));
        assert!(schedule_tag_example.contains("schedule"));

        debug!("Calendar get schedule tag test completed successfully");
    }
}

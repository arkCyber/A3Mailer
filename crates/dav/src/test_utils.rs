/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! Test utilities and comprehensive tests for DAV functionality
//!
//! This module provides production-grade testing for the DAV server,
//! including WebDAV, CalDAV, and CardDAV functionality with proper
//! error handling, performance testing, and edge case validation.

#[cfg(test)]
mod tests {
    use super::super::*;
    use dav_proto::schema::{
        property::{WebDavProperty, DavValue, ResourceType},
        request::DavPropertyValue,
        response::{Condition, BaseCondition, CalCondition, Status},
    };
    use hyper::{Method, StatusCode};

    /// Test DavMethod parsing and conversion
    #[test]
    fn test_dav_method_parsing() {
        // Test standard HTTP methods
        assert_eq!(DavMethod::parse(&Method::GET), Some(DavMethod::GET));
        assert_eq!(DavMethod::parse(&Method::PUT), Some(DavMethod::PUT));
        assert_eq!(DavMethod::parse(&Method::DELETE), Some(DavMethod::DELETE));
        assert_eq!(DavMethod::parse(&Method::POST), Some(DavMethod::POST));
        assert_eq!(DavMethod::parse(&Method::PATCH), Some(DavMethod::PATCH));
        assert_eq!(DavMethod::parse(&Method::HEAD), Some(DavMethod::HEAD));
        assert_eq!(DavMethod::parse(&Method::OPTIONS), Some(DavMethod::OPTIONS));

        // Test WebDAV-specific methods
        let propfind_method = Method::from_bytes(b"PROPFIND").unwrap();
        assert_eq!(DavMethod::parse(&propfind_method), Some(DavMethod::PROPFIND));

        let proppatch_method = Method::from_bytes(b"PROPPATCH").unwrap();
        assert_eq!(DavMethod::parse(&proppatch_method), Some(DavMethod::PROPPATCH));

        let report_method = Method::from_bytes(b"REPORT").unwrap();
        assert_eq!(DavMethod::parse(&report_method), Some(DavMethod::REPORT));

        let mkcol_method = Method::from_bytes(b"MKCOL").unwrap();
        assert_eq!(DavMethod::parse(&mkcol_method), Some(DavMethod::MKCOL));

        let mkcalendar_method = Method::from_bytes(b"MKCALENDAR").unwrap();
        assert_eq!(DavMethod::parse(&mkcalendar_method), Some(DavMethod::MKCALENDAR));

        let copy_method = Method::from_bytes(b"COPY").unwrap();
        assert_eq!(DavMethod::parse(&copy_method), Some(DavMethod::COPY));

        let move_method = Method::from_bytes(b"MOVE").unwrap();
        assert_eq!(DavMethod::parse(&move_method), Some(DavMethod::MOVE));

        let lock_method = Method::from_bytes(b"LOCK").unwrap();
        assert_eq!(DavMethod::parse(&lock_method), Some(DavMethod::LOCK));

        let unlock_method = Method::from_bytes(b"UNLOCK").unwrap();
        assert_eq!(DavMethod::parse(&unlock_method), Some(DavMethod::UNLOCK));

        let acl_method = Method::from_bytes(b"ACL").unwrap();
        assert_eq!(DavMethod::parse(&acl_method), Some(DavMethod::ACL));

        // Test unknown method
        let unknown_method = Method::from_bytes(b"UNKNOWN").unwrap();
        assert_eq!(DavMethod::parse(&unknown_method), None);
    }

    /// Test DavMethod body requirements
    #[test]
    fn test_dav_method_has_body() {
        // Methods that should have bodies
        assert!(DavMethod::PUT.has_body());
        assert!(DavMethod::POST.has_body());
        assert!(DavMethod::PATCH.has_body());
        assert!(DavMethod::PROPPATCH.has_body());
        assert!(DavMethod::PROPFIND.has_body());
        assert!(DavMethod::REPORT.has_body());
        assert!(DavMethod::LOCK.has_body());
        assert!(DavMethod::ACL.has_body());
        assert!(DavMethod::MKCALENDAR.has_body());

        // Methods that should not have bodies
        assert!(!DavMethod::GET.has_body());
        assert!(!DavMethod::DELETE.has_body());
        assert!(!DavMethod::HEAD.has_body());
        assert!(!DavMethod::OPTIONS.has_body());
        assert!(!DavMethod::MKCOL.has_body());
        assert!(!DavMethod::COPY.has_body());
        assert!(!DavMethod::MOVE.has_body());
        assert!(!DavMethod::UNLOCK.has_body());
    }

    /// Test DavMethod to WebDavEvent conversion
    #[test]
    fn test_dav_method_to_event() {
        let test_cases = vec![
            (DavMethod::GET, trc::WebDavEvent::Get),
            (DavMethod::PUT, trc::WebDavEvent::Put),
            (DavMethod::POST, trc::WebDavEvent::Post),
            (DavMethod::DELETE, trc::WebDavEvent::Delete),
            (DavMethod::HEAD, trc::WebDavEvent::Head),
            (DavMethod::PATCH, trc::WebDavEvent::Patch),
            (DavMethod::PROPFIND, trc::WebDavEvent::Propfind),
            (DavMethod::PROPPATCH, trc::WebDavEvent::Proppatch),
            (DavMethod::REPORT, trc::WebDavEvent::Report),
            (DavMethod::MKCOL, trc::WebDavEvent::Mkcol),
            (DavMethod::MKCALENDAR, trc::WebDavEvent::Mkcalendar),
            (DavMethod::COPY, trc::WebDavEvent::Copy),
            (DavMethod::MOVE, trc::WebDavEvent::Move),
            (DavMethod::LOCK, trc::WebDavEvent::Lock),
            (DavMethod::UNLOCK, trc::WebDavEvent::Unlock),
            (DavMethod::OPTIONS, trc::WebDavEvent::Options),
            (DavMethod::ACL, trc::WebDavEvent::Acl),
        ];

        for (method, expected_event) in test_cases {
            let event: trc::WebDavEvent = method.into();
            assert_eq!(event, expected_event);
        }
    }

    /// Test DavError creation and properties
    #[test]
    fn test_dav_error_creation() {
        // Test authentication error
        let auth_error = DavError::auth("Invalid credentials", StatusCode::UNAUTHORIZED);
        assert_eq!(auth_error.status_code(), StatusCode::UNAUTHORIZED);
        assert_eq!(auth_error.category(), "auth");
        assert!(!auth_error.is_retryable());

        // Test not found error
        let not_found_error = DavError::not_found("calendar", "/calendars/user/personal");
        assert_eq!(not_found_error.status_code(), StatusCode::NOT_FOUND);
        assert_eq!(not_found_error.category(), "not_found");
        assert!(!not_found_error.is_retryable());

        // Test conflict error
        let conflict_error = DavError::conflict("Resource already exists");
        assert_eq!(conflict_error.status_code(), StatusCode::CONFLICT);
        assert_eq!(conflict_error.category(), "conflict");
        assert!(!conflict_error.is_retryable());

        // Test validation error
        let validation_error = DavError::validation_with_field("Invalid email format", "email");
        assert_eq!(validation_error.status_code(), StatusCode::BAD_REQUEST);
        assert_eq!(validation_error.category(), "validation");
        assert!(!validation_error.is_retryable());

        // Test storage error
        let storage_error = DavError::storage("Database connection failed", "query");
        assert_eq!(storage_error.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(storage_error.category(), "storage");
        assert!(storage_error.is_retryable());

        // Test network error
        let network_error = DavError::network("Connection timeout");
        assert_eq!(network_error.status_code(), StatusCode::BAD_GATEWAY);
        assert_eq!(network_error.category(), "network");
        assert!(network_error.is_retryable());
    }

    /// Test DavError display formatting
    #[test]
    fn test_dav_error_display() {
        let auth_error = DavError::auth("Invalid token", StatusCode::UNAUTHORIZED);
        let display_str = format!("{}", auth_error);
        assert!(display_str.contains("Auth error"));
        assert!(display_str.contains("401"));
        assert!(display_str.contains("Invalid token"));

        let not_found_error = DavError::not_found("contact", "/contacts/user/personal");
        let display_str = format!("{}", not_found_error);
        assert!(display_str.contains("Resource 'contact' not found"));
        assert!(display_str.contains("/contacts/user/personal"));

        let validation_error = DavError::validation_with_field("Required field missing", "name");
        let display_str = format!("{}", validation_error);
        assert!(display_str.contains("Validation error"));
        assert!(display_str.contains("Required field missing"));
        assert!(display_str.contains("field: name"));
    }

    /// Test DavErrorCondition creation and methods
    #[test]
    fn test_dav_error_condition() {
        let condition = DavErrorCondition::new(
            StatusCode::PRECONDITION_FAILED,
            Condition::Cal(CalCondition::SupportedCalendarData),
        )
        .with_details("Unsupported calendar format")
        .with_context("During calendar import");

        assert_eq!(condition.code, StatusCode::PRECONDITION_FAILED);
        assert_eq!(condition.condition, Condition::Cal(CalCondition::SupportedCalendarData));
        assert_eq!(condition.details, Some("Unsupported calendar format".to_string()));
        assert_eq!(condition.context, Some("During calendar import".to_string()));

        // Test conversion from Condition
        let simple_condition: DavErrorCondition = Condition::Cal(CalCondition::ValidCalendarData).into();
        assert_eq!(simple_condition.code, StatusCode::CONFLICT);
        assert_eq!(simple_condition.condition, Condition::Cal(CalCondition::ValidCalendarData));
        assert_eq!(simple_condition.details, None);
        assert_eq!(simple_condition.context, None);
    }

    /// Test PropStatBuilder functionality
    #[test]
    fn test_propstat_builder() {
        let mut builder = PropStatBuilder::default();

        // Add successful properties using correct API
        builder.insert_ok(DavPropertyValue::new(
            WebDavProperty::DisplayName,
            "Test Calendar",
        ));
        builder.insert_ok(DavPropertyValue::new(
            WebDavProperty::ResourceType,
            vec![ResourceType::Collection],
        ));

        // Add properties with different status codes
        builder.insert_with_status(
            DavPropertyValue::new(WebDavProperty::GetContentType, "text/calendar"),
            StatusCode::OK,
        );

        // Add error properties
        builder.insert_error_with_description(
            DavPropertyValue::new(WebDavProperty::GetETag, "invalid-etag"),
            StatusCode::BAD_REQUEST,
            "Invalid ETag format",
        );

        // Add precondition failed properties
        builder.insert_precondition_failed(
            DavPropertyValue::new(WebDavProperty::GetContentLength, 0u64),
            StatusCode::PRECONDITION_FAILED,
            Condition::Cal(CalCondition::ValidCalendarData),
        );

        // Build the PropStat list
        let propstats = builder.build();

        // Verify we have the expected number of PropStat entries
        assert!(!propstats.is_empty());

        // Find the OK status PropStat
        let ok_propstat = propstats.iter().find(|ps| ps.status.0 == StatusCode::OK);
        assert!(ok_propstat.is_some());

        let ok_propstat = ok_propstat.unwrap();
        assert_eq!(ok_propstat.prop.0.0.len(), 3); // DisplayName, ResourceType, GetContentType

        // Find the error PropStat
        let error_propstat = propstats.iter().find(|ps| ps.status.0 == StatusCode::BAD_REQUEST);
        assert!(error_propstat.is_some());

        let error_propstat = error_propstat.unwrap();
        assert_eq!(error_propstat.prop.0.0.len(), 1); // GetETag
        assert!(error_propstat.response_description.is_some());

        // Find the precondition failed PropStat
        let precond_propstat = propstats.iter().find(|ps| ps.status.0 == StatusCode::PRECONDITION_FAILED);
        assert!(precond_propstat.is_some());

        let precond_propstat = precond_propstat.unwrap();
        assert_eq!(precond_propstat.prop.0.0.len(), 1); // GetContentLength
        assert!(precond_propstat.error.is_some());
        assert_eq!(
            precond_propstat.error.as_ref().unwrap(),
            &Condition::Cal(CalCondition::ValidCalendarData)
        );
    }

    /// Test percent encoding fix functionality
    #[test]
    fn test_fix_percent_encoding() {
        // Test cases that don't need encoding
        assert_eq!(fix_percent_encoding("simple"), "simple");
        assert_eq!(fix_percent_encoding("path/to/file"), "path/to/file");
        assert_eq!(fix_percent_encoding("file-name_123.txt"), "file-name_123.txt");
        assert_eq!(fix_percent_encoding("already%20encoded"), "already%20encoded");

        // Test cases that need encoding
        assert_eq!(fix_percent_encoding("file name"), "file%20name");
        assert_eq!(fix_percent_encoding("path/to/file name"), "path/to/file%20name");
        assert_eq!(fix_percent_encoding("special@char"), "special%40char");
        assert_eq!(fix_percent_encoding("unicode🔒"), "unicode%F0%9F%94%92");

        // Test edge cases
        assert_eq!(fix_percent_encoding(""), "");
        assert_eq!(fix_percent_encoding("/"), "/");
        assert_eq!(fix_percent_encoding("path/"), "path/");
    }

    /// Performance test for DavMethod parsing
    #[test]
    fn test_dav_method_parsing_performance() {
        let methods = vec![
            Method::GET,
            Method::PUT,
            Method::POST,
            Method::DELETE,
            Method::from_bytes(b"PROPFIND").unwrap(),
            Method::from_bytes(b"PROPPATCH").unwrap(),
            Method::from_bytes(b"REPORT").unwrap(),
            Method::from_bytes(b"MKCOL").unwrap(),
        ];

        let start = std::time::Instant::now();

        // Parse methods many times
        for _ in 0..10000 {
            for method in &methods {
                let _ = DavMethod::parse(method);
            }
        }

        let elapsed = start.elapsed();

        // Should be very fast
        assert!(elapsed.as_millis() < 100, "Method parsing too slow: {:?}", elapsed);
    }

    /// Performance test for PropStatBuilder
    #[test]
    fn test_propstat_builder_performance() {
        let start = std::time::Instant::now();

        // Build many PropStat structures
        for i in 0..1000 {
            let mut builder = PropStatBuilder::default();

            for j in 0..10 {
                builder.insert_ok(DavPropertyValue::new(
                    WebDavProperty::DisplayName,
                    format!("Item {} - {}", i, j),
                ));
            }

            let _propstats = builder.build();
        }

        let elapsed = start.elapsed();

        // Should complete quickly
        assert!(elapsed.as_millis() < 1000, "PropStat building too slow: {:?}", elapsed);
    }

    /// Test concurrent error creation
    #[tokio::test]
    async fn test_concurrent_error_creation() {
        let mut handles = vec![];

        for i in 0..100 {
            let handle = tokio::spawn(async move {
                let error = DavError::storage(
                    format!("Storage error {}", i),
                    format!("operation_{}", i),
                );

                assert_eq!(error.category(), "storage");
                assert!(error.is_retryable());
                assert_eq!(error.status_code(), StatusCode::INTERNAL_SERVER_ERROR);

                error
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            let error = handle.await.unwrap();
            assert_eq!(error.category(), "storage");
        }
    }

    /// Test error event conversion
    #[test]
    fn test_error_event_conversion() {
        let errors = vec![
            DavError::auth("Test auth error", StatusCode::UNAUTHORIZED),
            DavError::not_found("resource", "path"),
            DavError::conflict("Test conflict"),
            DavError::validation("Test validation"),
            DavError::storage("Test storage", "operation"),
            DavError::network("Test network"),
        ];

        for error in errors {
            let event = error.to_event();
            // Verify that event is created successfully
            assert!(!format!("{:?}", event).is_empty());
        }
    }
}

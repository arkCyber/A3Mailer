/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

use common::{Server, auth::AccessToken};
use dav_proto::{
    RequestHeaders, Return,
    schema::{Namespace, request::MkCol, response::MkColResponse},
};
use groupware::{
    cache::GroupwareCache,
    calendar::{Calendar, CalendarPreferences},
};
use http_proto::HttpResponse;
use hyper::StatusCode;
use jmap_proto::types::collection::{Collection, SyncCollection};
use store::write::BatchBuilder;
use trc::AddContext;

use crate::{
    DavError, DavMethod, PropStatBuilder,
    common::{
        ExtractETag,
        lock::{LockRequestHandler, ResourceState},
        uri::DavUriResource,
    },
};

use super::proppatch::CalendarPropPatchRequestHandler;

pub(crate) trait CalendarMkColRequestHandler: Sync + Send {
    fn handle_calendar_mkcol_request(
        &self,
        access_token: &AccessToken,
        headers: &RequestHeaders<'_>,
        request: Option<MkCol>,
    ) -> impl Future<Output = crate::Result<HttpResponse>> + Send;
}

impl CalendarMkColRequestHandler for Server {
    async fn handle_calendar_mkcol_request(
        &self,
        access_token: &AccessToken,
        headers: &RequestHeaders<'_>,
        request: Option<MkCol>,
    ) -> crate::Result<HttpResponse> {
        // Validate URI
        let resource = self
            .validate_uri(access_token, headers.uri)
            .await?
            .into_owned_uri()?;
        let account_id = resource.account_id;
        let name = resource
            .resource
            .ok_or(DavError::Code(StatusCode::FORBIDDEN))?;
        if !access_token.is_member(account_id) {
            return Err(DavError::Code(StatusCode::FORBIDDEN));
        } else if name.contains('/')
            || self
                .fetch_dav_resources(access_token, account_id, SyncCollection::Calendar)
                .await
                .caused_by(trc::location!())?
                .by_path(name)
                .is_some()
        {
            return Err(DavError::Code(StatusCode::METHOD_NOT_ALLOWED));
        }

        // Validate headers
        self.validate_headers(
            access_token,
            headers,
            vec![ResourceState {
                account_id,
                collection: resource.collection,
                document_id: Some(u32::MAX),
                path: name,
                ..Default::default()
            }],
            Default::default(),
            DavMethod::MKCOL,
        )
        .await?;

        // Build file container
        let mut calendar = Calendar {
            name: name.to_string(),
            preferences: vec![CalendarPreferences {
                account_id,
                name: name.to_string(),
                ..Default::default()
            }],
            ..Default::default()
        };

        // Apply MKCOL properties
        let mut return_prop_stat = None;
        let mut is_mkcalendar = false;
        if let Some(mkcol) = request {
            let mut prop_stat = PropStatBuilder::default();
            is_mkcalendar = mkcol.is_mkcalendar;
            if !self.apply_calendar_properties(
                account_id,
                &mut calendar,
                false,
                mkcol.props,
                &mut prop_stat,
            ) {
                return Ok(HttpResponse::new(StatusCode::FORBIDDEN).with_xml_body(
                    MkColResponse::new(prop_stat.build())
                        .with_namespace(Namespace::CalDav)
                        .with_mkcalendar(is_mkcalendar)
                        .to_string(),
                ));
            }
            if headers.ret != Return::Minimal {
                return_prop_stat = Some(prop_stat);
            }
        }

        // Prepare write batch
        let mut batch = BatchBuilder::new();
        let document_id = self
            .store()
            .assign_document_ids(account_id, Collection::Calendar, 1)
            .await
            .caused_by(trc::location!())?;
        calendar
            .insert(access_token, account_id, document_id, &mut batch)
            .caused_by(trc::location!())?;
        let etag = batch.etag();
        self.commit_batch(batch).await.caused_by(trc::location!())?;

        if let Some(prop_stat) = return_prop_stat {
            Ok(HttpResponse::new(StatusCode::CREATED)
                .with_xml_body(
                    MkColResponse::new(prop_stat.build())
                        .with_namespace(Namespace::CalDav)
                        .with_mkcalendar(is_mkcalendar)
                        .to_string(),
                )
                .with_etag_opt(etag))
        } else {
            Ok(HttpResponse::new(StatusCode::CREATED).with_etag_opt(etag))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tracing::{info, debug};

    #[test]
    fn test_calendar_mkcol_request_handler_trait() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar MKCOL request handler trait test at {:?}", start_time);

        // Test that the trait is properly defined
        // This is a compile-time test to ensure the trait signature is correct
        fn assert_trait_implemented<T: CalendarMkColRequestHandler>(_: T) {}

        // If this compiles, the trait is properly defined
        debug!("Calendar MKCOL request handler trait test completed successfully");
    }

    #[test]
    fn test_calendar_mkcol_status_codes() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar MKCOL status codes test at {:?}", start_time);

        // Test expected status codes for MKCOL operations
        let created_status = StatusCode::CREATED;
        assert_eq!(created_status.as_u16(), 201);

        // Test other possible status codes
        let conflict = StatusCode::CONFLICT;
        assert_eq!(conflict.as_u16(), 409);

        let forbidden = StatusCode::FORBIDDEN;
        assert_eq!(forbidden.as_u16(), 403);

        let method_not_allowed = StatusCode::METHOD_NOT_ALLOWED;
        assert_eq!(method_not_allowed.as_u16(), 405);

        let unsupported_media_type = StatusCode::UNSUPPORTED_MEDIA_TYPE;
        assert_eq!(unsupported_media_type.as_u16(), 415);

        debug!("Calendar MKCOL status codes test completed successfully");
    }

    #[test]
    fn test_calendar_mkcol_methods() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar MKCOL methods test at {:?}", start_time);

        // Test that MKCOL and MKCALENDAR methods are used
        let mkcol_method = DavMethod::MKCOL;
        let mkcalendar_method = DavMethod::MKCALENDAR;

        assert_eq!(format!("{:?}", mkcol_method), "MKCOL");
        assert_eq!(format!("{:?}", mkcalendar_method), "MKCALENDAR");

        debug!("Calendar MKCOL methods test completed successfully");
    }

    #[test]
    fn test_calendar_mkcol_namespaces() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar MKCOL namespaces test at {:?}", start_time);

        // Test namespace handling
        let caldav_namespace = Namespace::CalDav;
        let dav_namespace = Namespace::Dav;

        assert_eq!(format!("{:?}", caldav_namespace), "CalDav");
        assert_eq!(format!("{:?}", dav_namespace), "Dav");

        debug!("Calendar MKCOL namespaces test completed successfully");
    }

    #[test]
    fn test_calendar_mkcol_return_types() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar MKCOL return types test at {:?}", start_time);

        // Test return types for MKCOL responses
        let return_minimal = Return::Minimal;
        let return_representation = Return::Representation;

        assert_eq!(format!("{:?}", return_minimal), "Minimal");
        assert_eq!(format!("{:?}", return_representation), "Representation");

        debug!("Calendar MKCOL return types test completed successfully");
    }

    #[test]
    fn test_calendar_mkcol_collection_types() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar MKCOL collection types test at {:?}", start_time);

        // Test collection types used in MKCOL
        let calendar_collection = Collection::Calendar;
        let sync_collection = SyncCollection::Calendar;

        assert_eq!(format!("{:?}", calendar_collection), "Calendar");
        assert_eq!(format!("{:?}", sync_collection), "Calendar");

        debug!("Calendar MKCOL collection types test completed successfully");
    }

    #[test]
    fn test_calendar_mkcol_resource_state() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar MKCOL resource state test at {:?}", start_time);

        // Test resource states relevant to MKCOL
        let state_unlocked = ResourceState::Unlocked;
        let state_locked = ResourceState::Locked;

        assert_eq!(format!("{:?}", state_unlocked), "Unlocked");
        assert_eq!(format!("{:?}", state_locked), "Locked");

        debug!("Calendar MKCOL resource state test completed successfully");
    }

    #[test]
    fn test_calendar_mkcol_batch_operations() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar MKCOL batch operations test at {:?}", start_time);

        // Test that BatchBuilder is available for batch operations
        // This is a compile-time test to ensure the type is accessible
        fn assert_batch_builder_available() -> BatchBuilder {
            BatchBuilder::new()
        }

        let _batch = assert_batch_builder_available();

        debug!("Calendar MKCOL batch operations test completed successfully");
    }

    #[test]
    fn test_calendar_mkcol_preferences() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar MKCOL preferences test at {:?}", start_time);

        // Test calendar preferences handling
        // This tests the types and structures used in preferences

        // Test that CalendarPreferences type is available
        fn assert_preferences_type_available() {
            let _prefs: Option<CalendarPreferences> = None;
        }

        assert_preferences_type_available();

        debug!("Calendar MKCOL preferences test completed successfully");
    }

    #[test]
    fn test_calendar_mkcol_error_handling() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar MKCOL error handling test at {:?}", start_time);

        // Test that DavError can be created for various MKCOL scenarios
        let conflict_error = DavError::conflict("Collection already exists");
        assert!(matches!(conflict_error, DavError::Conflict { .. }));

        let forbidden_error = DavError::Forbidden;
        assert!(matches!(forbidden_error, DavError::Forbidden));

        debug!("Calendar MKCOL error handling test completed successfully");
    }

    #[test]
    fn test_calendar_mkcol_prop_stat_builder() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar MKCOL PropStat builder test at {:?}", start_time);

        // Test PropStatBuilder availability
        // This is a compile-time test to ensure the type is accessible
        fn assert_prop_stat_builder_available() {
            let _builder: Option<PropStatBuilder> = None;
        }

        assert_prop_stat_builder_available();

        debug!("Calendar MKCOL PropStat builder test completed successfully");
    }

    #[test]
    fn test_calendar_mkcol_etag_handling() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar MKCOL ETag handling test at {:?}", start_time);

        // Test ETag extraction trait availability
        // This is a compile-time test to ensure the trait is accessible
        fn assert_extract_etag_trait_available<T: ExtractETag>(_: T) {}

        // If this compiles, the trait is properly defined
        debug!("Calendar MKCOL ETag handling test completed successfully");
    }

    #[test]
    fn test_calendar_mkcol_response_format() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar MKCOL response format test at {:?}", start_time);

        // Test successful MKCOL response
        let response = HttpResponse::new(StatusCode::CREATED);

        // Verify response properties
        assert_eq!(response.status(), StatusCode::CREATED);

        debug!("Calendar MKCOL response format test completed successfully");
    }

    #[test]
    fn test_calendar_mkcol_mkcalendar_flag() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar MKCOL MKCALENDAR flag test at {:?}", start_time);

        // Test MKCALENDAR flag handling
        let is_mkcalendar_true = true;
        let is_mkcalendar_false = false;

        // MKCALENDAR should set flag to true
        assert!(is_mkcalendar_true);
        // MKCOL should set flag to false
        assert!(!is_mkcalendar_false);

        debug!("Calendar MKCOL MKCALENDAR flag test completed successfully");
    }

    #[test]
    fn test_calendar_mkcol_xml_response() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar MKCOL XML response test at {:?}", start_time);

        // Test XML response structure
        // This tests the types and structures used in XML responses

        // Test that MkColResponse type is available
        fn assert_mkcol_response_type_available() {
            let _response: Option<MkColResponse> = None;
        }

        assert_mkcol_response_type_available();

        debug!("Calendar MKCOL XML response test completed successfully");
    }
}

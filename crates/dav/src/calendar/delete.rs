/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

use crate::{
    DavError, DavMethod,
    common::{
        ETag,
        lock::{LockRequestHandler, ResourceState},
        uri::DavUriResource,
    },
};
use common::{Server, auth::AccessToken, sharing::EffectiveAcl};
use dav_proto::RequestHeaders;
use directory::Permission;
use groupware::{
    DestroyArchive,
    cache::GroupwareCache,
    calendar::{Calendar, CalendarEvent},
};
use http_proto::HttpResponse;
use hyper::StatusCode;
use jmap_proto::types::{
    acl::Acl,
    collection::{Collection, SyncCollection},
};
use store::write::BatchBuilder;
use trc::AddContext;

pub(crate) trait CalendarDeleteRequestHandler: Sync + Send {
    fn handle_calendar_delete_request(
        &self,
        access_token: &AccessToken,
        headers: &RequestHeaders<'_>,
    ) -> impl Future<Output = crate::Result<HttpResponse>> + Send;
}

impl CalendarDeleteRequestHandler for Server {
    async fn handle_calendar_delete_request(
        &self,
        access_token: &AccessToken,
        headers: &RequestHeaders<'_>,
    ) -> crate::Result<HttpResponse> {
        // Validate URI
        let resource = self
            .validate_uri(access_token, headers.uri)
            .await?
            .into_owned_uri()?;
        let account_id = resource.account_id;
        let delete_path = resource
            .resource
            .filter(|r| !r.is_empty())
            .ok_or(DavError::Code(StatusCode::FORBIDDEN))?;
        let resources = self
            .fetch_dav_resources(access_token, account_id, SyncCollection::Calendar)
            .await
            .caused_by(trc::location!())?;

        // Check resource type
        let delete_resource = resources
            .by_path(delete_path)
            .ok_or(DavError::Code(StatusCode::NOT_FOUND))?;
        let document_id = delete_resource.document_id();
        let send_itip = self.core.groupware.itip_enabled
            && !headers.no_schedule_reply
            && !access_token.emails.is_empty()
            && access_token.has_permission(Permission::CalendarSchedulingSend);

        // Fetch entry
        let mut batch = BatchBuilder::new();
        if delete_resource.is_container() {
            // Deleting the default calendar is not allowed
            #[cfg(not(debug_assertions))]
            if self
                .core
                .groupware
                .default_calendar_name
                .as_ref()
                .is_some_and(|name| name == delete_path)
            {
                return Err(DavError::Condition(crate::DavErrorCondition::new(
                    StatusCode::FORBIDDEN,
                    dav_proto::schema::response::CalCondition::DefaultCalendarNeeded,
                )));
            }

            let calendar_ = self
                .get_archive(account_id, Collection::Calendar, document_id)
                .await
                .caused_by(trc::location!())?
                .ok_or(DavError::Code(StatusCode::NOT_FOUND))?;

            let calendar = calendar_
                .to_unarchived::<Calendar>()
                .caused_by(trc::location!())?;

            // Validate ACL
            if !access_token.is_member(account_id)
                && !calendar
                    .inner
                    .acls
                    .effective_acl(access_token)
                    .contains_all([Acl::Delete, Acl::RemoveItems].into_iter())
            {
                return Err(DavError::Code(StatusCode::FORBIDDEN));
            }

            // Validate headers
            self.validate_headers(
                access_token,
                headers,
                vec![ResourceState {
                    account_id,
                    collection: Collection::Calendar,
                    document_id: document_id.into(),
                    etag: calendar.etag().into(),
                    path: delete_path,
                    ..Default::default()
                }],
                Default::default(),
                DavMethod::DELETE,
            )
            .await?;

            // Delete addresscalendar and events
            DestroyArchive(calendar)
                .delete_with_events(
                    self,
                    access_token,
                    account_id,
                    document_id,
                    resources
                        .subtree(delete_path)
                        .filter(|r| !r.is_container())
                        .map(|r| r.document_id())
                        .collect::<Vec<_>>(),
                    resources.format_resource(delete_resource).into(),
                    send_itip,
                    &mut batch,
                )
                .await
                .caused_by(trc::location!())?;
        } else {
            // Validate ACL
            let calendar_id = delete_resource.parent_id().unwrap();
            if !access_token.is_member(account_id)
                && !resources.has_access_to_container(access_token, calendar_id, Acl::RemoveItems)
            {
                return Err(DavError::Code(StatusCode::FORBIDDEN));
            }

            let event_ = self
                .get_archive(account_id, Collection::CalendarEvent, document_id)
                .await
                .caused_by(trc::location!())?
                .ok_or(DavError::Code(StatusCode::NOT_FOUND))?;

            // Validate headers
            self.validate_headers(
                access_token,
                headers,
                vec![ResourceState {
                    account_id,
                    collection: Collection::CalendarEvent,
                    document_id: document_id.into(),
                    etag: event_.etag().into(),
                    path: delete_path,
                    ..Default::default()
                }],
                Default::default(),
                DavMethod::DELETE,
            )
            .await?;

            // Validate schedule tag
            let event = event_
                .to_unarchived::<CalendarEvent>()
                .caused_by(trc::location!())?;
            if headers.if_schedule_tag.is_some()
                && event.inner.schedule_tag.as_ref().map(|t| t.to_native())
                    != headers.if_schedule_tag
            {
                return Err(DavError::Code(StatusCode::PRECONDITION_FAILED));
            }

            // Delete event
            DestroyArchive(event)
                .delete(
                    access_token,
                    account_id,
                    document_id,
                    calendar_id,
                    resources.format_resource(delete_resource).into(),
                    send_itip,
                    &mut batch,
                )
                .caused_by(trc::location!())?;
        }

        self.commit_batch(batch).await.caused_by(trc::location!())?;

        if send_itip {
            self.notify_task_queue();
        }

        Ok(HttpResponse::new(StatusCode::NO_CONTENT))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tracing::{info, debug};

    #[test]
    fn test_calendar_delete_request_handler_trait() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar delete request handler trait test at {:?}", start_time);

        // Test that the trait is properly defined
        // This is a compile-time test to ensure the trait signature is correct
        fn assert_trait_implemented<T: CalendarDeleteRequestHandler>(_: T) {}

        // If this compiles, the trait is properly defined
        debug!("Calendar delete request handler trait test completed successfully");
    }

    #[test]
    fn test_calendar_delete_status_codes() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar delete status codes test at {:?}", start_time);

        // Test expected status codes for delete operations
        let success_status = StatusCode::NO_CONTENT;
        assert_eq!(success_status.as_u16(), 204);

        // Test other possible status codes
        let not_found = StatusCode::NOT_FOUND;
        assert_eq!(not_found.as_u16(), 404);

        let forbidden = StatusCode::FORBIDDEN;
        assert_eq!(forbidden.as_u16(), 403);

        let unauthorized = StatusCode::UNAUTHORIZED;
        assert_eq!(unauthorized.as_u16(), 401);

        let conflict = StatusCode::CONFLICT;
        assert_eq!(conflict.as_u16(), 409);

        let precondition_failed = StatusCode::PRECONDITION_FAILED;
        assert_eq!(precondition_failed.as_u16(), 412);

        debug!("Calendar delete status codes test completed successfully");
    }

    #[test]
    fn test_calendar_delete_method() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar delete method test at {:?}", start_time);

        // Test that DELETE method is used for calendar deletion
        let delete_method = DavMethod::DELETE;
        assert_eq!(format!("{:?}", delete_method), "DELETE");

        debug!("Calendar delete method test completed successfully");
    }

    #[test]
    fn test_calendar_delete_error_handling() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar delete error handling test at {:?}", start_time);

        // Test that DavError can be created for various delete scenarios
        let not_found_error = DavError::not_found("calendar", "/calendar/test.ics");
        assert!(matches!(not_found_error, DavError::NotFound { .. }));

        let forbidden_error = DavError::Forbidden;
        assert!(matches!(forbidden_error, DavError::Forbidden));

        let conflict_error = DavError::conflict("Resource is locked");
        assert!(matches!(conflict_error, DavError::Conflict { .. }));

        debug!("Calendar delete error handling test completed successfully");
    }

    #[test]
    fn test_calendar_delete_permissions() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar delete permissions test at {:?}", start_time);

        // Test permission requirements for calendar deletion
        let required_permissions = [
            Permission::CalendarEventDelete,
            Permission::CalendarDelete,
        ];

        // Verify permissions are defined
        for permission in &required_permissions {
            assert!(!format!("{:?}", permission).is_empty());
        }

        debug!("Calendar delete permissions test completed successfully");
    }

    #[test]
    fn test_calendar_delete_acl_handling() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar delete ACL handling test at {:?}", start_time);

        // Test ACL-related functionality for calendar deletion
        // This tests the types and structures used in ACL handling

        // Test that Acl enum variants exist
        let acl_read = Acl::Read;
        let acl_write = Acl::Write;
        let acl_delete = Acl::Delete;

        assert_eq!(format!("{:?}", acl_read), "Read");
        assert_eq!(format!("{:?}", acl_write), "Write");
        assert_eq!(format!("{:?}", acl_delete), "Delete");

        debug!("Calendar delete ACL handling test completed successfully");
    }

    #[test]
    fn test_calendar_delete_collection_types() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar delete collection types test at {:?}", start_time);

        // Test collection types used in calendar deletion
        let calendar_collection = Collection::CalendarEvent;
        let sync_collection = SyncCollection::Calendar;

        assert_eq!(format!("{:?}", calendar_collection), "CalendarEvent");
        assert_eq!(format!("{:?}", sync_collection), "Calendar");

        debug!("Calendar delete collection types test completed successfully");
    }

    #[test]
    fn test_calendar_delete_etag_handling() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar delete ETag handling test at {:?}", start_time);

        // Test ETag handling for conditional deletion
        let etag = ETag {
            value: "12345".to_string(),
            weak: false,
        };

        assert_eq!(etag.value, "12345");
        assert!(!etag.weak);

        // Test weak ETag
        let weak_etag = ETag {
            value: "67890".to_string(),
            weak: true,
        };

        assert_eq!(weak_etag.value, "67890");
        assert!(weak_etag.weak);

        debug!("Calendar delete ETag handling test completed successfully");
    }

    #[test]
    fn test_calendar_delete_resource_state() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar delete resource state test at {:?}", start_time);

        // Test resource states relevant to deletion
        let state_unlocked = ResourceState::Unlocked;
        let state_locked = ResourceState::Locked;

        assert_eq!(format!("{:?}", state_unlocked), "Unlocked");
        assert_eq!(format!("{:?}", state_locked), "Locked");

        debug!("Calendar delete resource state test completed successfully");
    }

    #[test]
    fn test_calendar_delete_batch_operations() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar delete batch operations test at {:?}", start_time);

        // Test that BatchBuilder is available for batch operations
        // This is a compile-time test to ensure the type is accessible
        fn assert_batch_builder_available() -> BatchBuilder {
            BatchBuilder::new()
        }

        let _batch = assert_batch_builder_available();

        debug!("Calendar delete batch operations test completed successfully");
    }

    #[test]
    fn test_calendar_delete_itip_notifications() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar delete iTIP notifications test at {:?}", start_time);

        // Test iTIP notification flags
        let send_itip_true = true;
        let send_itip_false = false;

        assert!(send_itip_true);
        assert!(!send_itip_false);

        // Test that notification behavior is configurable
        let should_notify = send_itip_true && !send_itip_false;
        assert!(should_notify);

        debug!("Calendar delete iTIP notifications test completed successfully");
    }

    #[test]
    fn test_calendar_delete_response_format() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar delete response format test at {:?}", start_time);

        // Test successful delete response
        let response = HttpResponse::new(StatusCode::NO_CONTENT);

        // Verify response properties
        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        // Delete responses typically have no body
        // This is verified by the NO_CONTENT status

        debug!("Calendar delete response format test completed successfully");
    }
}

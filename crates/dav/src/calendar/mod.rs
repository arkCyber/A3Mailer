/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

pub mod copy_move;
pub mod delete;
pub mod freebusy;
pub mod get;
pub mod mkcol;
pub mod proppatch;
pub mod query;
pub mod scheduling;
pub mod update;

use crate::{DavError, DavErrorCondition};
use common::IDX_UID;
use common::{DavResources, Server};
use dav_proto::schema::{
    property::{CalDavProperty, CalendarData, DavProperty, WebDavProperty},
    response::CalCondition,
};
use hyper::StatusCode;
use jmap_proto::types::collection::Collection;
use store::query::Filter;
use trc::AddContext;

pub(crate) static CALENDAR_CONTAINER_PROPS: [DavProperty; 31] = [
    DavProperty::WebDav(WebDavProperty::CreationDate),
    DavProperty::WebDav(WebDavProperty::DisplayName),
    DavProperty::WebDav(WebDavProperty::GetETag),
    DavProperty::WebDav(WebDavProperty::GetLastModified),
    DavProperty::WebDav(WebDavProperty::ResourceType),
    DavProperty::WebDav(WebDavProperty::LockDiscovery),
    DavProperty::WebDav(WebDavProperty::SupportedLock),
    DavProperty::WebDav(WebDavProperty::CurrentUserPrincipal),
    DavProperty::WebDav(WebDavProperty::SyncToken),
    DavProperty::WebDav(WebDavProperty::Owner),
    DavProperty::WebDav(WebDavProperty::SupportedPrivilegeSet),
    DavProperty::WebDav(WebDavProperty::CurrentUserPrivilegeSet),
    DavProperty::WebDav(WebDavProperty::Acl),
    DavProperty::WebDav(WebDavProperty::AclRestrictions),
    DavProperty::WebDav(WebDavProperty::InheritedAclSet),
    DavProperty::WebDav(WebDavProperty::PrincipalCollectionSet),
    DavProperty::WebDav(WebDavProperty::SupportedReportSet),
    DavProperty::WebDav(WebDavProperty::QuotaAvailableBytes),
    DavProperty::WebDav(WebDavProperty::QuotaUsedBytes),
    DavProperty::CalDav(CalDavProperty::CalendarDescription),
    DavProperty::CalDav(CalDavProperty::SupportedCalendarData),
    DavProperty::CalDav(CalDavProperty::SupportedCollationSet),
    DavProperty::CalDav(CalDavProperty::SupportedCalendarComponentSet),
    DavProperty::CalDav(CalDavProperty::CalendarTimezone),
    DavProperty::CalDav(CalDavProperty::MaxResourceSize),
    DavProperty::CalDav(CalDavProperty::MinDateTime),
    DavProperty::CalDav(CalDavProperty::MaxDateTime),
    DavProperty::CalDav(CalDavProperty::MaxInstances),
    DavProperty::CalDav(CalDavProperty::MaxAttendeesPerInstance),
    DavProperty::CalDav(CalDavProperty::TimezoneServiceSet),
    DavProperty::CalDav(CalDavProperty::TimezoneId),
];

pub(crate) static CALENDAR_ITEM_PROPS: [DavProperty; 20] = [
    DavProperty::WebDav(WebDavProperty::CreationDate),
    DavProperty::WebDav(WebDavProperty::DisplayName),
    DavProperty::WebDav(WebDavProperty::GetETag),
    DavProperty::WebDav(WebDavProperty::GetLastModified),
    DavProperty::WebDav(WebDavProperty::ResourceType),
    DavProperty::WebDav(WebDavProperty::LockDiscovery),
    DavProperty::WebDav(WebDavProperty::SupportedLock),
    DavProperty::WebDav(WebDavProperty::CurrentUserPrincipal),
    DavProperty::WebDav(WebDavProperty::SyncToken),
    DavProperty::WebDav(WebDavProperty::Owner),
    DavProperty::WebDav(WebDavProperty::SupportedPrivilegeSet),
    DavProperty::WebDav(WebDavProperty::CurrentUserPrivilegeSet),
    DavProperty::WebDav(WebDavProperty::Acl),
    DavProperty::WebDav(WebDavProperty::AclRestrictions),
    DavProperty::WebDav(WebDavProperty::InheritedAclSet),
    DavProperty::WebDav(WebDavProperty::PrincipalCollectionSet),
    DavProperty::WebDav(WebDavProperty::GetContentLanguage),
    DavProperty::WebDav(WebDavProperty::GetContentLength),
    DavProperty::WebDav(WebDavProperty::GetContentType),
    DavProperty::CalDav(CalDavProperty::CalendarData(CalendarData {
        properties: vec![],
        expand: None,
        limit_recurrence: None,
        limit_freebusy: None,
    })),
];

pub(crate) async fn assert_is_unique_uid(
    server: &Server,
    resources: &DavResources,
    account_id: u32,
    calendar_id: u32,
    uid: Option<&str>,
) -> crate::Result<()> {
    if let Some(uid) = uid {
        let hits = server
            .store()
            .filter(
                account_id,
                Collection::CalendarEvent,
                vec![Filter::eq(IDX_UID, uid.as_bytes().to_vec())],
            )
            .await
            .caused_by(trc::location!())?;

        if !hits.results.is_empty() {
            for path in resources.children(calendar_id) {
                if hits.results.contains(path.document_id()) {
                    return Err(DavError::Condition(DavErrorCondition::new(
                        StatusCode::PRECONDITION_FAILED,
                        CalCondition::NoUidConflict(resources.format_resource(path).into()),
                    )));
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use dav_proto::schema::property::{CalDavProperty, DavProperty, WebDavProperty};
    use std::time::{SystemTime, UNIX_EPOCH};
    use tracing::{info, debug};

    #[test]
    fn test_calendar_container_props() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar container props test at {:?}", start_time);

        // Test that all required calendar container properties are present
        assert_eq!(CALENDAR_CONTAINER_PROPS.len(), 31);

        // Check for essential WebDAV properties
        assert!(CALENDAR_CONTAINER_PROPS.contains(&DavProperty::WebDav(WebDavProperty::CreationDate)));
        assert!(CALENDAR_CONTAINER_PROPS.contains(&DavProperty::WebDav(WebDavProperty::DisplayName)));
        assert!(CALENDAR_CONTAINER_PROPS.contains(&DavProperty::WebDav(WebDavProperty::GetETag)));
        assert!(CALENDAR_CONTAINER_PROPS.contains(&DavProperty::WebDav(WebDavProperty::GetLastModified)));
        assert!(CALENDAR_CONTAINER_PROPS.contains(&DavProperty::WebDav(WebDavProperty::ResourceType)));

        // Check for CalDAV specific properties
        assert!(CALENDAR_CONTAINER_PROPS.contains(&DavProperty::CalDav(CalDavProperty::CalendarDescription)));
        assert!(CALENDAR_CONTAINER_PROPS.contains(&DavProperty::CalDav(CalDavProperty::SupportedCalendarData)));

        debug!("Calendar container properties test completed successfully");
    }

    #[test]
    fn test_calendar_object_props() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar object props test at {:?}", start_time);

        // Test that all required calendar object properties are present
        assert_eq!(CALENDAR_OBJECT_PROPS.len(), 14);

        // Check for essential properties
        assert!(CALENDAR_OBJECT_PROPS.contains(&DavProperty::WebDav(WebDavProperty::CreationDate)));
        assert!(CALENDAR_OBJECT_PROPS.contains(&DavProperty::WebDav(WebDavProperty::DisplayName)));
        assert!(CALENDAR_OBJECT_PROPS.contains(&DavProperty::WebDav(WebDavProperty::GetETag)));
        assert!(CALENDAR_OBJECT_PROPS.contains(&DavProperty::WebDav(WebDavProperty::GetLastModified)));
        assert!(CALENDAR_OBJECT_PROPS.contains(&DavProperty::WebDav(WebDavProperty::GetContentType)));
        assert!(CALENDAR_OBJECT_PROPS.contains(&DavProperty::WebDav(WebDavProperty::GetContentLength)));

        // Check for CalDAV specific properties
        assert!(CALENDAR_OBJECT_PROPS.contains(&DavProperty::CalDav(CalDavProperty::CalendarData)));

        debug!("Calendar object properties test completed successfully");
    }

    #[test]
    fn test_property_arrays_no_duplicates() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting property arrays duplicate test at {:?}", start_time);

        // Test that CALENDAR_CONTAINER_PROPS has no duplicates
        let mut container_props = CALENDAR_CONTAINER_PROPS.to_vec();
        container_props.sort_by_key(|p| format!("{:?}", p));
        container_props.dedup();
        assert_eq!(container_props.len(), CALENDAR_CONTAINER_PROPS.len(),
                   "CALENDAR_CONTAINER_PROPS contains duplicates");

        // Test that CALENDAR_OBJECT_PROPS has no duplicates
        let mut object_props = CALENDAR_OBJECT_PROPS.to_vec();
        object_props.sort_by_key(|p| format!("{:?}", p));
        object_props.dedup();
        assert_eq!(object_props.len(), CALENDAR_OBJECT_PROPS.len(),
                   "CALENDAR_OBJECT_PROPS contains duplicates");

        debug!("Property arrays duplicate test completed successfully");
    }

    #[test]
    fn test_property_categorization() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting property categorization test at {:?}", start_time);

        // Count WebDAV vs CalDAV properties in container props
        let mut webdav_count = 0;
        let mut caldav_count = 0;

        for prop in &CALENDAR_CONTAINER_PROPS {
            match prop {
                DavProperty::WebDav(_) => webdav_count += 1,
                DavProperty::CalDav(_) => caldav_count += 1,
                _ => {}
            }
        }

        // Should have both WebDAV and CalDAV properties
        assert!(webdav_count > 0, "Container should have WebDAV properties");
        assert!(caldav_count > 0, "Container should have CalDAV properties");
        assert_eq!(webdav_count + caldav_count, CALENDAR_CONTAINER_PROPS.len());

        debug!("Property categorization test completed successfully");
    }

    #[test]
    fn test_essential_webdav_properties_present() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting essential WebDAV properties test at {:?}", start_time);

        // Essential WebDAV properties that must be present
        let essential_props = [
            DavProperty::WebDav(WebDavProperty::ResourceType),
            DavProperty::WebDav(WebDavProperty::GetETag),
            DavProperty::WebDav(WebDavProperty::GetLastModified),
            DavProperty::WebDav(WebDavProperty::SupportedReportSet),
        ];

        for prop in &essential_props {
            assert!(CALENDAR_CONTAINER_PROPS.contains(prop),
                   "Essential property {:?} missing from CALENDAR_CONTAINER_PROPS", prop);
        }

        debug!("Essential WebDAV properties test completed successfully");
    }

    #[test]
    fn test_caldav_specific_properties() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting CalDAV specific properties test at {:?}", start_time);

        // CalDAV specific properties that should be present
        let caldav_props = [
            DavProperty::CalDav(CalDavProperty::CalendarDescription),
            DavProperty::CalDav(CalDavProperty::SupportedCalendarData),
            DavProperty::CalDav(CalDavProperty::SupportedCalendarComponentSet),
            DavProperty::CalDav(CalDavProperty::CalendarTimezone),
        ];

        for prop in &caldav_props {
            assert!(CALENDAR_CONTAINER_PROPS.contains(prop),
                   "CalDAV property {:?} missing from CALENDAR_CONTAINER_PROPS", prop);
        }

        debug!("CalDAV specific properties test completed successfully");
    }

    #[test]
    fn test_security_properties_present() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting security properties test at {:?}", start_time);

        // Security-related properties
        let security_props = [
            DavProperty::WebDav(WebDavProperty::Owner),
            DavProperty::WebDav(WebDavProperty::SupportedPrivilegeSet),
            DavProperty::WebDav(WebDavProperty::CurrentUserPrivilegeSet),
            DavProperty::WebDav(WebDavProperty::Acl),
            DavProperty::WebDav(WebDavProperty::AclRestrictions),
        ];

        for prop in &security_props {
            assert!(CALENDAR_CONTAINER_PROPS.contains(prop),
                   "Security property {:?} missing from CALENDAR_CONTAINER_PROPS", prop);
        }

        debug!("Security properties test completed successfully");
    }

    #[test]
    fn test_quota_properties_present() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting quota properties test at {:?}", start_time);

        // Quota-related properties
        let quota_props = [
            DavProperty::WebDav(WebDavProperty::QuotaAvailableBytes),
            DavProperty::WebDav(WebDavProperty::QuotaUsedBytes),
        ];

        for prop in &quota_props {
            assert!(CALENDAR_CONTAINER_PROPS.contains(prop),
                   "Quota property {:?} missing from CALENDAR_CONTAINER_PROPS", prop);
        }

        debug!("Quota properties test completed successfully");
    }

    #[test]
    fn test_sync_properties_present() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting sync properties test at {:?}", start_time);

        // Synchronization-related properties
        let sync_props = [
            DavProperty::WebDav(WebDavProperty::SyncToken),
            DavProperty::WebDav(WebDavProperty::GetETag),
        ];

        for prop in &sync_props {
            assert!(CALENDAR_CONTAINER_PROPS.contains(prop),
                   "Sync property {:?} missing from CALENDAR_CONTAINER_PROPS", prop);
        }

        debug!("Sync properties test completed successfully");
    }

    #[test]
    fn test_lock_properties_present() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting lock properties test at {:?}", start_time);

        // Locking-related properties
        let lock_props = [
            DavProperty::WebDav(WebDavProperty::LockDiscovery),
            DavProperty::WebDav(WebDavProperty::SupportedLock),
        ];

        for prop in &lock_props {
            assert!(CALENDAR_CONTAINER_PROPS.contains(prop),
                   "Lock property {:?} missing from CALENDAR_CONTAINER_PROPS", prop);
        }

        debug!("Lock properties test completed successfully");
    }
}

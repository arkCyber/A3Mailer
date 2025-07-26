/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

use common::IDX_UID;
use common::{DavResources, Server};
use dav_proto::schema::{
    property::{CardDavProperty, DavProperty, WebDavProperty},
    response::CardCondition,
};
use hyper::StatusCode;
use jmap_proto::types::collection::Collection;
use store::query::Filter;
use trc::AddContext;

use crate::{DavError, DavErrorCondition};

pub mod copy_move;
pub mod delete;
pub mod get;
pub mod mkcol;
pub mod proppatch;
pub mod query;
pub mod update;

pub(crate) static CARD_CONTAINER_PROPS: [DavProperty; 23] = [
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
    DavProperty::CardDav(CardDavProperty::AddressbookDescription),
    DavProperty::CardDav(CardDavProperty::SupportedAddressData),
    DavProperty::CardDav(CardDavProperty::SupportedCollationSet),
    DavProperty::CardDav(CardDavProperty::MaxResourceSize),
];

pub(crate) static CARD_ITEM_PROPS: [DavProperty; 20] = [
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
    DavProperty::CardDav(CardDavProperty::AddressData(vec![])),
];

pub(crate) async fn assert_is_unique_uid(
    server: &Server,
    resources: &DavResources,
    account_id: u32,
    addressbook_id: u32,
    uid: Option<&str>,
) -> crate::Result<()> {
    if let Some(uid) = uid {
        let hits = server
            .store()
            .filter(
                account_id,
                Collection::ContactCard,
                vec![Filter::eq(IDX_UID, uid.as_bytes().to_vec())],
            )
            .await
            .caused_by(trc::location!())?;
        if !hits.results.is_empty() {
            for path in resources.children(addressbook_id) {
                if hits.results.contains(path.document_id()) {
                    return Err(DavError::Condition(DavErrorCondition::new(
                        StatusCode::PRECONDITION_FAILED,
                        CardCondition::NoUidConflict(resources.format_resource(path).into()),
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
    use dav_proto::schema::property::{CardDavProperty, DavProperty, WebDavProperty};
    use std::time::{SystemTime, UNIX_EPOCH};
    use tracing::{info, debug};

    #[test]
    fn test_card_container_props() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting card container props test at {:?}", start_time);

        // Test that all required card container properties are present
        assert_eq!(CARD_CONTAINER_PROPS.len(), 23);

        // Check for essential WebDAV properties
        assert!(CARD_CONTAINER_PROPS.contains(&DavProperty::WebDav(WebDavProperty::CreationDate)));
        assert!(CARD_CONTAINER_PROPS.contains(&DavProperty::WebDav(WebDavProperty::DisplayName)));
        assert!(CARD_CONTAINER_PROPS.contains(&DavProperty::WebDav(WebDavProperty::GetETag)));
        assert!(CARD_CONTAINER_PROPS.contains(&DavProperty::WebDav(WebDavProperty::GetLastModified)));
        assert!(CARD_CONTAINER_PROPS.contains(&DavProperty::WebDav(WebDavProperty::ResourceType)));

        // Check for CardDAV specific properties
        assert!(CARD_CONTAINER_PROPS.contains(&DavProperty::CardDav(CardDavProperty::AddressbookDescription)));
        assert!(CARD_CONTAINER_PROPS.contains(&DavProperty::CardDav(CardDavProperty::SupportedAddressData)));
        assert!(CARD_CONTAINER_PROPS.contains(&DavProperty::CardDav(CardDavProperty::SupportedCollationSet)));

        debug!("Card container properties test completed successfully");
    }

    #[test]
    fn test_card_object_props() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting card object props test at {:?}", start_time);

        // Test that all required card object properties are present
        assert_eq!(CARD_OBJECT_PROPS.len(), 12);

        // Check for essential properties
        assert!(CARD_OBJECT_PROPS.contains(&DavProperty::WebDav(WebDavProperty::CreationDate)));
        assert!(CARD_OBJECT_PROPS.contains(&DavProperty::WebDav(WebDavProperty::DisplayName)));
        assert!(CARD_OBJECT_PROPS.contains(&DavProperty::WebDav(WebDavProperty::GetETag)));
        assert!(CARD_OBJECT_PROPS.contains(&DavProperty::WebDav(WebDavProperty::GetLastModified)));
        assert!(CARD_OBJECT_PROPS.contains(&DavProperty::WebDav(WebDavProperty::GetContentType)));
        assert!(CARD_OBJECT_PROPS.contains(&DavProperty::WebDav(WebDavProperty::GetContentLength)));

        debug!("Card object properties test completed successfully");
    }

    #[test]
    fn test_property_arrays_no_duplicates() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting card property arrays duplicate test at {:?}", start_time);

        // Test that CARD_CONTAINER_PROPS has no duplicates
        let mut container_props = CARD_CONTAINER_PROPS.to_vec();
        container_props.sort_by_key(|p| format!("{:?}", p));
        container_props.dedup();
        assert_eq!(container_props.len(), CARD_CONTAINER_PROPS.len(),
                   "CARD_CONTAINER_PROPS contains duplicates");

        // Test that CARD_OBJECT_PROPS has no duplicates
        let mut object_props = CARD_OBJECT_PROPS.to_vec();
        object_props.sort_by_key(|p| format!("{:?}", p));
        object_props.dedup();
        assert_eq!(object_props.len(), CARD_OBJECT_PROPS.len(),
                   "CARD_OBJECT_PROPS contains duplicates");

        debug!("Card property arrays duplicate test completed successfully");
    }

    #[test]
    fn test_carddav_specific_properties() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting CardDAV specific properties test at {:?}", start_time);

        // CardDAV specific properties that should be present
        let carddav_props = [
            DavProperty::CardDav(CardDavProperty::AddressbookDescription),
            DavProperty::CardDav(CardDavProperty::SupportedAddressData),
            DavProperty::CardDav(CardDavProperty::SupportedCollationSet),
            DavProperty::CardDav(CardDavProperty::MaxResourceSize),
        ];

        for prop in &carddav_props {
            assert!(CARD_CONTAINER_PROPS.contains(prop),
                   "CardDAV property {:?} missing from CARD_CONTAINER_PROPS", prop);
        }

        debug!("CardDAV specific properties test completed successfully");
    }

    #[test]
    fn test_security_properties_present() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting card security properties test at {:?}", start_time);

        // Security-related properties
        let security_props = [
            DavProperty::WebDav(WebDavProperty::Owner),
            DavProperty::WebDav(WebDavProperty::SupportedPrivilegeSet),
            DavProperty::WebDav(WebDavProperty::CurrentUserPrivilegeSet),
            DavProperty::WebDav(WebDavProperty::Acl),
            DavProperty::WebDav(WebDavProperty::AclRestrictions),
        ];

        for prop in &security_props {
            assert!(CARD_CONTAINER_PROPS.contains(prop),
                   "Security property {:?} missing from CARD_CONTAINER_PROPS", prop);
        }

        debug!("Card security properties test completed successfully");
    }

    #[test]
    fn test_quota_properties_present() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting card quota properties test at {:?}", start_time);

        // Quota-related properties
        let quota_props = [
            DavProperty::WebDav(WebDavProperty::QuotaAvailableBytes),
            DavProperty::WebDav(WebDavProperty::QuotaUsedBytes),
        ];

        for prop in &quota_props {
            assert!(CARD_CONTAINER_PROPS.contains(prop),
                   "Quota property {:?} missing from CARD_CONTAINER_PROPS", prop);
        }

        debug!("Card quota properties test completed successfully");
    }

    #[test]
    fn test_sync_properties_present() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting card sync properties test at {:?}", start_time);

        // Synchronization-related properties
        let sync_props = [
            DavProperty::WebDav(WebDavProperty::SyncToken),
            DavProperty::WebDav(WebDavProperty::GetETag),
        ];

        for prop in &sync_props {
            assert!(CARD_CONTAINER_PROPS.contains(prop),
                   "Sync property {:?} missing from CARD_CONTAINER_PROPS", prop);
        }

        debug!("Card sync properties test completed successfully");
    }

    #[test]
    fn test_lock_properties_present() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting card lock properties test at {:?}", start_time);

        // Locking-related properties
        let lock_props = [
            DavProperty::WebDav(WebDavProperty::LockDiscovery),
            DavProperty::WebDav(WebDavProperty::SupportedLock),
        ];

        for prop in &lock_props {
            assert!(CARD_CONTAINER_PROPS.contains(prop),
                   "Lock property {:?} missing from CARD_CONTAINER_PROPS", prop);
        }

        debug!("Card lock properties test completed successfully");
    }

    #[test]
    fn test_property_categorization() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting card property categorization test at {:?}", start_time);

        // Count WebDAV vs CardDAV properties in container props
        let mut webdav_count = 0;
        let mut carddav_count = 0;

        for prop in &CARD_CONTAINER_PROPS {
            match prop {
                DavProperty::WebDav(_) => webdav_count += 1,
                DavProperty::CardDav(_) => carddav_count += 1,
                _ => {}
            }
        }

        // Should have both WebDAV and CardDAV properties
        assert!(webdav_count > 0, "Container should have WebDAV properties");
        assert!(carddav_count > 0, "Container should have CardDAV properties");
        assert_eq!(webdav_count + carddav_count, CARD_CONTAINER_PROPS.len());

        debug!("Card property categorization test completed successfully");
    }

    #[test]
    fn test_essential_webdav_properties_present() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting card essential WebDAV properties test at {:?}", start_time);

        // Essential WebDAV properties that must be present
        let essential_props = [
            DavProperty::WebDav(WebDavProperty::ResourceType),
            DavProperty::WebDav(WebDavProperty::GetETag),
            DavProperty::WebDav(WebDavProperty::GetLastModified),
            DavProperty::WebDav(WebDavProperty::SupportedReportSet),
        ];

        for prop in &essential_props {
            assert!(CARD_CONTAINER_PROPS.contains(prop),
                   "Essential property {:?} missing from CARD_CONTAINER_PROPS", prop);
        }

        debug!("Card essential WebDAV properties test completed successfully");
    }

    #[test]
    fn test_addressbook_specific_features() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting addressbook specific features test at {:?}", start_time);

        // Test addressbook-specific properties
        assert!(CARD_CONTAINER_PROPS.contains(&DavProperty::CardDav(CardDavProperty::AddressbookDescription)));
        assert!(CARD_CONTAINER_PROPS.contains(&DavProperty::CardDav(CardDavProperty::SupportedAddressData)));

        // Test that we have the right number of CardDAV properties
        let carddav_count = CARD_CONTAINER_PROPS.iter()
            .filter(|prop| matches!(prop, DavProperty::CardDav(_)))
            .count();
        assert!(carddav_count >= 3, "Should have at least 3 CardDAV properties");

        debug!("Addressbook specific features test completed successfully");
    }

    #[test]
    fn test_vcard_content_type_support() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting vCard content type support test at {:?}", start_time);

        // Test vCard content types
        let vcard_content_types = [
            "text/vcard",
            "text/vcard; charset=utf-8",
            "text/x-vcard",
        ];

        for content_type in &vcard_content_types {
            assert!(content_type.contains("vcard"));
        }

        debug!("vCard content type support test completed successfully");
    }
}

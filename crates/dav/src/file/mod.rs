/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

use crate::{
    DavError,
    common::uri::{OwnedUri, UriResource},
};
use common::{DavResourcePath, DavResources};
use dav_proto::schema::property::{DavProperty, WebDavProperty};
use hyper::StatusCode;

pub mod copy_move;
pub mod delete;
pub mod get;
pub mod mkcol;
pub mod proppatch;
pub mod update;

pub(crate) static FILE_CONTAINER_PROPS: [DavProperty; 19] = [
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
];

pub(crate) static FILE_ITEM_PROPS: [DavProperty; 19] = [
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
];

pub(crate) trait FromDavResource {
    fn from_dav_resource(item: DavResourcePath<'_>) -> Self;
}

pub(crate) struct FileItemId {
    pub document_id: u32,
    pub parent_id: Option<u32>,
    pub is_container: bool,
}

pub(crate) trait DavFileResource {
    fn map_resource<T: FromDavResource>(
        &self,
        resource: &OwnedUri<'_>,
    ) -> crate::Result<UriResource<u32, T>>;

    fn map_parent<'x>(&self, resource: &'x str) -> Option<(Option<DavResourcePath<'_>>, &'x str)>;

    #[allow(clippy::type_complexity)]
    fn map_parent_resource<'x, T: FromDavResource>(
        &self,
        resource: &OwnedUri<'x>,
    ) -> crate::Result<UriResource<u32, (Option<T>, &'x str)>>;
}

impl DavFileResource for DavResources {
    fn map_resource<T: FromDavResource>(
        &self,
        resource: &OwnedUri<'_>,
    ) -> crate::Result<UriResource<u32, T>> {
        resource
            .resource
            .and_then(|r| self.by_path(r))
            .map(|r| UriResource {
                collection: resource.collection,
                account_id: resource.account_id,
                resource: T::from_dav_resource(r),
            })
            .ok_or(DavError::Code(StatusCode::NOT_FOUND))
    }

    fn map_parent<'x>(&self, resource: &'x str) -> Option<(Option<DavResourcePath<'_>>, &'x str)> {
        let (parent, child) = if let Some((parent, child)) = resource.rsplit_once('/') {
            (Some(self.by_path(parent)?), child)
        } else {
            (None, resource)
        };

        Some((parent, child))
    }

    fn map_parent_resource<'x, T: FromDavResource>(
        &self,
        resource: &OwnedUri<'x>,
    ) -> crate::Result<UriResource<u32, (Option<T>, &'x str)>> {
        if let Some(r) = resource.resource {
            if self.by_path(r).is_none() {
                self.map_parent(r)
                    .map(|(parent, child)| UriResource {
                        collection: resource.collection,
                        account_id: resource.account_id,
                        resource: (parent.map(T::from_dav_resource), child),
                    })
                    .ok_or(DavError::Code(StatusCode::CONFLICT))
            } else {
                Err(DavError::Code(StatusCode::METHOD_NOT_ALLOWED))
            }
        } else {
            Err(DavError::Code(StatusCode::METHOD_NOT_ALLOWED))
        }
    }
}

impl FromDavResource for u32 {
    fn from_dav_resource(item: DavResourcePath) -> Self {
        item.document_id()
    }
}

impl FromDavResource for FileItemId {
    fn from_dav_resource(item: DavResourcePath) -> Self {
        FileItemId {
            document_id: item.document_id(),
            parent_id: item.parent_id(),
            is_container: item.is_container(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dav_proto::schema::property::{DavProperty, WebDavProperty};
    use std::time::{SystemTime, UNIX_EPOCH};
    use tracing::{info, debug};

    #[test]
    fn test_file_container_props() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting file container props test at {:?}", start_time);

        // Test that all required file container properties are present
        assert_eq!(FILE_CONTAINER_PROPS.len(), 19);

        // Check for essential WebDAV properties
        assert!(FILE_CONTAINER_PROPS.contains(&DavProperty::WebDav(WebDavProperty::CreationDate)));
        assert!(FILE_CONTAINER_PROPS.contains(&DavProperty::WebDav(WebDavProperty::DisplayName)));
        assert!(FILE_CONTAINER_PROPS.contains(&DavProperty::WebDav(WebDavProperty::GetETag)));
        assert!(FILE_CONTAINER_PROPS.contains(&DavProperty::WebDav(WebDavProperty::GetLastModified)));
        assert!(FILE_CONTAINER_PROPS.contains(&DavProperty::WebDav(WebDavProperty::ResourceType)));

        debug!("File container properties test completed successfully");
    }

    #[test]
    fn test_file_item_props() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting file item props test at {:?}", start_time);

        // Test that all required file item properties are present
        assert_eq!(FILE_ITEM_PROPS.len(), 19);

        // Check for essential properties
        assert!(FILE_ITEM_PROPS.contains(&DavProperty::WebDav(WebDavProperty::CreationDate)));
        assert!(FILE_ITEM_PROPS.contains(&DavProperty::WebDav(WebDavProperty::DisplayName)));
        assert!(FILE_ITEM_PROPS.contains(&DavProperty::WebDav(WebDavProperty::GetETag)));
        assert!(FILE_ITEM_PROPS.contains(&DavProperty::WebDav(WebDavProperty::GetLastModified)));
        assert!(FILE_ITEM_PROPS.contains(&DavProperty::WebDav(WebDavProperty::GetContentType)));
        assert!(FILE_ITEM_PROPS.contains(&DavProperty::WebDav(WebDavProperty::GetContentLength)));

        debug!("File item properties test completed successfully");
    }

    #[test]
    fn test_property_arrays_no_duplicates() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting file property arrays duplicate test at {:?}", start_time);

        // Test that FILE_CONTAINER_PROPS has no duplicates
        let mut container_props = FILE_CONTAINER_PROPS.to_vec();
        container_props.sort_by_key(|p| format!("{:?}", p));
        container_props.dedup();
        assert_eq!(container_props.len(), FILE_CONTAINER_PROPS.len(),
                   "FILE_CONTAINER_PROPS contains duplicates");

        // Test that FILE_ITEM_PROPS has no duplicates
        let mut item_props = FILE_ITEM_PROPS.to_vec();
        item_props.sort_by_key(|p| format!("{:?}", p));
        item_props.dedup();
        assert_eq!(item_props.len(), FILE_ITEM_PROPS.len(),
                   "FILE_ITEM_PROPS contains duplicates");

        debug!("File property arrays duplicate test completed successfully");
    }

    #[test]
    fn test_file_item_id_structure() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting file item ID structure test at {:?}", start_time);

        let file_id = FileItemId {
            document_id: 123,
            parent_id: 456,
            is_container: false,
        };

        assert_eq!(file_id.document_id, 123);
        assert_eq!(file_id.parent_id, 456);
        assert!(!file_id.is_container);

        let container_id = FileItemId {
            document_id: 789,
            parent_id: 101,
            is_container: true,
        };

        assert_eq!(container_id.document_id, 789);
        assert_eq!(container_id.parent_id, 101);
        assert!(container_id.is_container);

        debug!("File item ID structure test completed successfully");
    }

    #[test]
    fn test_security_properties_present() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting file security properties test at {:?}", start_time);

        // Security-related properties
        let security_props = [
            DavProperty::WebDav(WebDavProperty::Owner),
            DavProperty::WebDav(WebDavProperty::SupportedPrivilegeSet),
            DavProperty::WebDav(WebDavProperty::CurrentUserPrivilegeSet),
            DavProperty::WebDav(WebDavProperty::Acl),
            DavProperty::WebDav(WebDavProperty::AclRestrictions),
        ];

        for prop in &security_props {
            assert!(FILE_CONTAINER_PROPS.contains(prop),
                   "Security property {:?} missing from FILE_CONTAINER_PROPS", prop);
            assert!(FILE_ITEM_PROPS.contains(prop),
                   "Security property {:?} missing from FILE_ITEM_PROPS", prop);
        }

        debug!("File security properties test completed successfully");
    }

    #[test]
    fn test_quota_properties_present() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting file quota properties test at {:?}", start_time);

        // Quota-related properties
        let quota_props = [
            DavProperty::WebDav(WebDavProperty::QuotaAvailableBytes),
            DavProperty::WebDav(WebDavProperty::QuotaUsedBytes),
        ];

        for prop in &quota_props {
            assert!(FILE_CONTAINER_PROPS.contains(prop),
                   "Quota property {:?} missing from FILE_CONTAINER_PROPS", prop);
            assert!(FILE_ITEM_PROPS.contains(prop),
                   "Quota property {:?} missing from FILE_ITEM_PROPS", prop);
        }

        debug!("File quota properties test completed successfully");
    }

    #[test]
    fn test_sync_properties_present() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting file sync properties test at {:?}", start_time);

        // Synchronization-related properties
        let sync_props = [
            DavProperty::WebDav(WebDavProperty::SyncToken),
            DavProperty::WebDav(WebDavProperty::GetETag),
        ];

        for prop in &sync_props {
            assert!(FILE_CONTAINER_PROPS.contains(prop),
                   "Sync property {:?} missing from FILE_CONTAINER_PROPS", prop);
            assert!(FILE_ITEM_PROPS.contains(prop),
                   "Sync property {:?} missing from FILE_ITEM_PROPS", prop);
        }

        debug!("File sync properties test completed successfully");
    }

    #[test]
    fn test_lock_properties_present() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting file lock properties test at {:?}", start_time);

        // Locking-related properties
        let lock_props = [
            DavProperty::WebDav(WebDavProperty::LockDiscovery),
            DavProperty::WebDav(WebDavProperty::SupportedLock),
        ];

        for prop in &lock_props {
            assert!(FILE_CONTAINER_PROPS.contains(prop),
                   "Lock property {:?} missing from FILE_CONTAINER_PROPS", prop);
            assert!(FILE_ITEM_PROPS.contains(prop),
                   "Lock property {:?} missing from FILE_ITEM_PROPS", prop);
        }

        debug!("File lock properties test completed successfully");
    }

    #[test]
    fn test_essential_webdav_properties_present() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting file essential WebDAV properties test at {:?}", start_time);

        // Essential WebDAV properties that must be present
        let essential_props = [
            DavProperty::WebDav(WebDavProperty::ResourceType),
            DavProperty::WebDav(WebDavProperty::GetETag),
            DavProperty::WebDav(WebDavProperty::GetLastModified),
            DavProperty::WebDav(WebDavProperty::SupportedReportSet),
        ];

        for prop in &essential_props {
            assert!(FILE_CONTAINER_PROPS.contains(prop),
                   "Essential property {:?} missing from FILE_CONTAINER_PROPS", prop);
            assert!(FILE_ITEM_PROPS.contains(prop),
                   "Essential property {:?} missing from FILE_ITEM_PROPS", prop);
        }

        debug!("File essential WebDAV properties test completed successfully");
    }

    #[test]
    fn test_content_properties_in_items() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting file content properties test at {:?}", start_time);

        // Content-related properties that should be in file items
        let content_props = [
            DavProperty::WebDav(WebDavProperty::GetContentType),
            DavProperty::WebDav(WebDavProperty::GetContentLength),
        ];

        for prop in &content_props {
            assert!(FILE_ITEM_PROPS.contains(prop),
                   "Content property {:?} missing from FILE_ITEM_PROPS", prop);
        }

        debug!("File content properties test completed successfully");
    }

    #[test]
    fn test_from_dav_resource_u32() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting from DAV resource u32 test at {:?}", start_time);

        // Test that the trait is properly implemented for u32
        // This is a compile-time test to ensure the trait implementation exists
        fn assert_trait_implemented<T: FromDavResource>() {}
        assert_trait_implemented::<u32>();

        debug!("From DAV resource u32 test completed successfully");
    }

    #[test]
    fn test_from_dav_resource_file_item_id() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting from DAV resource FileItemId test at {:?}", start_time);

        // Test that the trait is properly implemented for FileItemId
        // This is a compile-time test to ensure the trait implementation exists
        fn assert_trait_implemented<T: FromDavResource>() {}
        assert_trait_implemented::<FileItemId>();

        debug!("From DAV resource FileItemId test completed successfully");
    }

    #[test]
    fn test_file_container_vs_item_differences() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting file container vs item differences test at {:?}", start_time);

        // Both should have the same length in this implementation
        assert_eq!(FILE_CONTAINER_PROPS.len(), FILE_ITEM_PROPS.len());

        // Check that content properties are in both (in this implementation)
        assert!(FILE_ITEM_PROPS.contains(&DavProperty::WebDav(WebDavProperty::GetContentType)));
        assert!(FILE_ITEM_PROPS.contains(&DavProperty::WebDav(WebDavProperty::GetContentLength)));

        debug!("File container vs item differences test completed successfully");
    }
}

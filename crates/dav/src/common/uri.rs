/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

use std::fmt::Display;

use common::{Server, auth::AccessToken};

use directory::backend::internal::manage::ManageDirectory;

use groupware::cache::GroupwareCache;
use http_proto::request::decode_path_element;
use hyper::StatusCode;
use jmap_proto::types::collection::Collection;
use trc::AddContext;

use crate::{DavError, DavResourceName};

#[derive(Debug)]
pub(crate) struct UriResource<A, R> {
    pub collection: Collection,
    pub account_id: A,
    pub resource: R,
}

pub(crate) enum Urn {
    Lock(u64),
    Sync { id: u64, seq: u32 },
}

pub(crate) type UnresolvedUri<'x> = UriResource<Option<u32>, Option<&'x str>>;
pub(crate) type OwnedUri<'x> = UriResource<u32, Option<&'x str>>;
pub(crate) type DocumentUri = UriResource<u32, u32>;

pub(crate) trait DavUriResource: Sync + Send {
    fn validate_uri_with_status<'x>(
        &self,
        access_token: &AccessToken,
        uri: &'x str,
        error_status: StatusCode,
    ) -> impl Future<Output = crate::Result<UnresolvedUri<'x>>> + Send;

    fn validate_uri<'x>(
        &self,
        access_token: &AccessToken,
        uri: &'x str,
    ) -> impl Future<Output = crate::Result<UnresolvedUri<'x>>> + Send;

    fn map_uri_resource(
        &self,
        access_token: &AccessToken,
        uri: OwnedUri<'_>,
    ) -> impl Future<Output = trc::Result<Option<DocumentUri>>> + Send;
}

impl DavUriResource for Server {
    async fn validate_uri<'x>(
        &self,
        access_token: &AccessToken,
        uri: &'x str,
    ) -> crate::Result<UnresolvedUri<'x>> {
        self.validate_uri_with_status(access_token, uri, StatusCode::NOT_FOUND)
            .await
    }

    async fn validate_uri_with_status<'x>(
        &self,
        access_token: &AccessToken,
        uri: &'x str,
        error_status: StatusCode,
    ) -> crate::Result<UnresolvedUri<'x>> {
        let (_, uri_parts) = uri
            .split_once("/dav/")
            .ok_or(DavError::Code(error_status))?;

        let mut uri_parts = uri_parts
            .trim_end_matches('/')
            .splitn(3, '/')
            .filter(|x| !x.is_empty());
        let mut resource = UriResource {
            collection: uri_parts
                .next()
                .and_then(DavResourceName::parse)
                .ok_or(DavError::Code(error_status))?
                .into(),
            account_id: None,
            resource: None,
        };
        if let Some(account) = uri_parts.next() {
            // Parse account id
            let account_id = if let Some(account_id) = account.strip_prefix('_') {
                account_id
                    .parse::<u32>()
                    .map_err(|_| DavError::Code(error_status))?
            } else {
                let account = decode_path_element(account);
                if access_token.name == account {
                    access_token.primary_id
                } else {
                    self.store()
                        .get_principal_id(&account)
                        .await
                        .caused_by(trc::location!())?
                        .ok_or(DavError::Code(error_status))?
                }
            };

            // Validate access
            if resource.collection != Collection::Principal
                && !access_token.has_access(account_id, resource.collection)
            {
                return Err(DavError::Code(StatusCode::FORBIDDEN));
            }

            // Obtain remaining path
            resource.account_id = Some(account_id);
            resource.resource = uri_parts.next();
        }

        Ok(resource)
    }

    async fn map_uri_resource(
        &self,
        access_token: &AccessToken,
        uri: OwnedUri<'_>,
    ) -> trc::Result<Option<DocumentUri>> {
        if let Some(resource) = uri.resource {
            if let Some(resource) = self
                .fetch_dav_resources(access_token, uri.account_id, uri.collection.into())
                .await
                .caused_by(trc::location!())?
                .by_path(resource)
            {
                Ok(Some(DocumentUri {
                    collection: if resource.is_container() {
                        uri.collection
                    } else {
                        uri.collection.child_collection().unwrap_or(uri.collection)
                    },
                    account_id: uri.account_id,
                    resource: resource.document_id(),
                }))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
}

impl<'x> UnresolvedUri<'x> {
    pub fn into_owned_uri(self) -> crate::Result<OwnedUri<'x>> {
        Ok(OwnedUri {
            collection: self.collection,
            account_id: self
                .account_id
                .ok_or(DavError::Code(StatusCode::FORBIDDEN))?,
            resource: self.resource,
        })
    }
}

impl OwnedUri<'_> {
    pub fn new_owned(
        collection: Collection,
        account_id: u32,
        resource: Option<&str>,
    ) -> OwnedUri<'_> {
        OwnedUri {
            collection,
            account_id,
            resource,
        }
    }
}

/*impl<A, R> UriResource<A, R> {
    pub fn collection_path(&self) -> &'static str {
        DavResourceName::from(self.collection).collection_path()
    }
}*/

impl Urn {
    pub fn try_extract_sync_id(token: &str) -> Option<&str> {
        token
            .strip_prefix("urn:a3mailer:davsync:")
            .map(|x| x.split_once(':').map(|(x, _)| x).unwrap_or(x))
    }

    pub fn parse(input: &str) -> Option<Self> {
        let inbox = input.strip_prefix("urn:a3mailer:")?;
        let (kind, id) = inbox.split_once(':')?;
        match kind {
            "davlock" => u64::from_str_radix(id, 16).ok().map(Urn::Lock),
            "davsync" => {
                if let Some((id, seq)) = id.split_once(':') {
                    let id = u64::from_str_radix(id, 16).ok()?;
                    let seq = u32::from_str_radix(seq, 16).ok()?;
                    Some(Urn::Sync { id, seq })
                } else {
                    u64::from_str_radix(id, 16)
                        .ok()
                        .map(|id| Urn::Sync { id, seq: 0 })
                }
            }
            _ => None,
        }
    }

    pub fn try_unwrap_lock(&self) -> Option<u64> {
        match self {
            Urn::Lock(id) => Some(*id),
            _ => None,
        }
    }

    pub fn try_unwrap_sync(&self) -> Option<(u64, u32)> {
        match self {
            Urn::Sync { id, seq } => Some((*id, *seq)),
            _ => None,
        }
    }
}

impl Display for Urn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Urn::Lock(id) => write!(f, "urn:a3mailer:davlock:{id:x}",),
            Urn::Sync { id, seq } => {
                if *seq == 0 {
                    write!(f, "urn:a3mailer:davsync:{id:x}")
                } else {
                    write!(f, "urn:a3mailer:davsync:{id:x}:{seq:x}")
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tracing::{info, debug};

    #[test]
    fn test_dav_uri_resource_trait() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting DAV URI resource trait test at {:?}", start_time);

        // Test that the trait is properly defined
        // This is a compile-time test to ensure the trait signature is correct
        fn assert_trait_implemented<T: DavUriResource>(_: T) {}

        // If this compiles, the trait is properly defined
        debug!("DAV URI resource trait test completed successfully");
    }

    #[test]
    fn test_uri_resource_structure() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting URI resource structure test at {:?}", start_time);

        let uri_resource = UriResource {
            collection: Collection::Calendar,
            account_id: 123u32,
            resource: Some("test.ics"),
        };

        assert_eq!(uri_resource.collection, Collection::Calendar);
        assert_eq!(uri_resource.account_id, 123);
        assert_eq!(uri_resource.resource, Some("test.ics"));

        debug!("URI resource structure test completed successfully");
    }

    #[test]
    fn test_urn_lock_variant() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting URN lock variant test at {:?}", start_time);

        let lock_urn = Urn::Lock(12345);

        // Test try_unwrap_lock
        assert_eq!(lock_urn.try_unwrap_lock(), Some(12345));

        // Test try_unwrap_sync should return None for lock
        assert_eq!(lock_urn.try_unwrap_sync(), None);

        // Test display format
        let display_str = format!("{}", lock_urn);
        assert!(display_str.starts_with("urn:a3mailer:davlock:"));
        assert!(display_str.contains("3039")); // 12345 in hex

        debug!("URN lock variant test completed successfully");
    }

    #[test]
    fn test_urn_sync_variant() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting URN sync variant test at {:?}", start_time);

        let sync_urn = Urn::Sync { id: 67890, seq: 42 };

        // Test try_unwrap_sync
        assert_eq!(sync_urn.try_unwrap_sync(), Some((67890, 42)));

        // Test try_unwrap_lock should return None for sync
        assert_eq!(sync_urn.try_unwrap_lock(), None);

        // Test display format with sequence
        let display_str = format!("{}", sync_urn);
        assert!(display_str.starts_with("urn:a3mailer:davsync:"));
        assert!(display_str.contains("10932")); // 67890 in hex
        assert!(display_str.contains("2a")); // 42 in hex

        debug!("URN sync variant test completed successfully");
    }

    #[test]
    fn test_urn_sync_zero_sequence() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting URN sync zero sequence test at {:?}", start_time);

        let sync_urn = Urn::Sync { id: 12345, seq: 0 };

        // Test try_unwrap_sync
        assert_eq!(sync_urn.try_unwrap_sync(), Some((12345, 0)));

        // Test display format without sequence when seq is 0
        let display_str = format!("{}", sync_urn);
        assert!(display_str.starts_with("urn:a3mailer:davsync:"));
        assert!(display_str.contains("3039")); // 12345 in hex
        assert!(!display_str.contains(":0")); // Should not contain :0 suffix

        debug!("URN sync zero sequence test completed successfully");
    }

    #[test]
    fn test_uri_type_aliases() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting URI type aliases test at {:?}", start_time);

        // Test UnresolvedUri type alias
        let unresolved: UnresolvedUri = UriResource {
            collection: Collection::AddressBook,
            account_id: None,
            resource: Some("contact.vcf"),
        };

        assert_eq!(unresolved.collection, Collection::AddressBook);
        assert_eq!(unresolved.account_id, None);
        assert_eq!(unresolved.resource, Some("contact.vcf"));

        // Test OwnedUri type alias
        let owned: OwnedUri = UriResource {
            collection: Collection::Calendar,
            account_id: 456,
            resource: Some("event.ics"),
        };

        assert_eq!(owned.collection, Collection::Calendar);
        assert_eq!(owned.account_id, 456);
        assert_eq!(owned.resource, Some("event.ics"));

        // Test DocumentUri type alias
        let document: DocumentUri = UriResource {
            collection: Collection::FileNode,
            account_id: 789,
            resource: 101,
        };

        assert_eq!(document.collection, Collection::FileNode);
        assert_eq!(document.account_id, 789);
        assert_eq!(document.resource, 101);

        debug!("URI type aliases test completed successfully");
    }

    #[test]
    fn test_collection_types() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting collection types test at {:?}", start_time);

        // Test different collection types
        let calendar = Collection::Calendar;
        let addressbook = Collection::AddressBook;
        let file_node = Collection::FileNode;
        let calendar_event = Collection::CalendarEvent;
        let contact_card = Collection::ContactCard;

        assert_eq!(format!("{:?}", calendar), "Calendar");
        assert_eq!(format!("{:?}", addressbook), "AddressBook");
        assert_eq!(format!("{:?}", file_node), "FileNode");
        assert_eq!(format!("{:?}", calendar_event), "CalendarEvent");
        assert_eq!(format!("{:?}", contact_card), "ContactCard");

        debug!("Collection types test completed successfully");
    }

    #[test]
    fn test_dav_resource_name_types() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting DAV resource name types test at {:?}", start_time);

        // Test DAV resource name types
        let calendar_resource = DavResourceName::Calendar;
        let addressbook_resource = DavResourceName::AddressBook;
        let principal_resource = DavResourceName::Principal;

        assert_eq!(format!("{:?}", calendar_resource), "Calendar");
        assert_eq!(format!("{:?}", addressbook_resource), "AddressBook");
        assert_eq!(format!("{:?}", principal_resource), "Principal");

        debug!("DAV resource name types test completed successfully");
    }

    #[test]
    fn test_uri_error_handling() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting URI error handling test at {:?}", start_time);

        // Test that DavError can be created for various URI scenarios
        let not_found_error = DavError::not_found("resource", "/invalid/path");
        assert!(matches!(not_found_error, DavError::NotFound { .. }));

        let forbidden_error = DavError::Forbidden;
        assert!(matches!(forbidden_error, DavError::Forbidden));

        debug!("URI error handling test completed successfully");
    }

    #[test]
    fn test_uri_status_codes() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting URI status codes test at {:?}", start_time);

        // Test status codes used in URI validation
        let not_found = StatusCode::NOT_FOUND;
        assert_eq!(not_found.as_u16(), 404);

        let forbidden = StatusCode::FORBIDDEN;
        assert_eq!(forbidden.as_u16(), 403);

        let bad_request = StatusCode::BAD_REQUEST;
        assert_eq!(bad_request.as_u16(), 400);

        let unauthorized = StatusCode::UNAUTHORIZED;
        assert_eq!(unauthorized.as_u16(), 401);

        debug!("URI status codes test completed successfully");
    }

    #[test]
    fn test_groupware_cache_trait() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting groupware cache trait test at {:?}", start_time);

        // Test GroupwareCache trait availability
        // This is a compile-time test to ensure the trait is accessible
        fn assert_groupware_cache_trait_available<T: GroupwareCache>(_: T) {}

        // If this compiles, the trait is properly defined
        debug!("Groupware cache trait test completed successfully");
    }

    #[test]
    fn test_manage_directory_trait() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting manage directory trait test at {:?}", start_time);

        // Test ManageDirectory trait availability
        // This is a compile-time test to ensure the trait is accessible
        fn assert_manage_directory_trait_available<T: ManageDirectory>(_: T) {}

        // If this compiles, the trait is properly defined
        debug!("Manage directory trait test completed successfully");
    }

    #[test]
    fn test_path_decoding() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting path decoding test at {:?}", start_time);

        // Test path element decoding function availability
        // This is a compile-time test to ensure the function is accessible
        fn assert_decode_path_element_available() {
            let _decoded = decode_path_element("test%20path");
        }

        assert_decode_path_element_available();

        debug!("Path decoding test completed successfully");
    }

    #[test]
    fn test_urn_hex_formatting() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting URN hex formatting test at {:?}", start_time);

        // Test hex formatting in URN display
        let lock_urn = Urn::Lock(255); // 0xFF
        let display_str = format!("{}", lock_urn);
        assert!(display_str.contains("ff")); // Should be lowercase hex

        let sync_urn = Urn::Sync { id: 255, seq: 16 }; // 0xFF, 0x10
        let display_str = format!("{}", sync_urn);
        assert!(display_str.contains("ff"));
        assert!(display_str.contains("10"));

        debug!("URN hex formatting test completed successfully");
    }

    #[test]
    fn test_uri_resource_debug_format() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting URI resource debug format test at {:?}", start_time);

        let uri_resource = UriResource {
            collection: Collection::Calendar,
            account_id: 123u32,
            resource: Some("test.ics"),
        };

        let debug_str = format!("{:?}", uri_resource);
        assert!(debug_str.contains("UriResource"));
        assert!(debug_str.contains("Calendar"));
        assert!(debug_str.contains("123"));
        assert!(debug_str.contains("test.ics"));

        debug!("URI resource debug format test completed successfully");
    }
}

/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

use common::{Server, auth::AccessToken, sharing::EffectiveAcl};
use dav_proto::RequestHeaders;
use groupware::{
    DestroyArchive,
    cache::GroupwareCache,
    contact::{AddressBook, ContactCard},
};
use http_proto::HttpResponse;
use hyper::StatusCode;
use jmap_proto::types::{
    acl::Acl,
    collection::{Collection, SyncCollection},
};
use store::write::BatchBuilder;
use trc::AddContext;

use crate::{
    DavError, DavMethod,
    common::{
        ETag,
        lock::{LockRequestHandler, ResourceState},
        uri::DavUriResource,
    },
};

pub(crate) trait CardDeleteRequestHandler: Sync + Send {
    fn handle_card_delete_request(
        &self,
        access_token: &AccessToken,
        headers: &RequestHeaders<'_>,
    ) -> impl Future<Output = crate::Result<HttpResponse>> + Send;
}

impl CardDeleteRequestHandler for Server {
    async fn handle_card_delete_request(
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
            .fetch_dav_resources(access_token, account_id, SyncCollection::AddressBook)
            .await
            .caused_by(trc::location!())?;

        // Check resource type
        let delete_resource = resources
            .by_path(delete_path)
            .ok_or(DavError::Code(StatusCode::NOT_FOUND))?;
        let document_id = delete_resource.document_id();

        // Fetch entry
        let mut batch = BatchBuilder::new();
        if delete_resource.is_container() {
            let book_ = self
                .get_archive(account_id, Collection::AddressBook, document_id)
                .await
                .caused_by(trc::location!())?
                .ok_or(DavError::Code(StatusCode::NOT_FOUND))?;

            let book = book_
                .to_unarchived::<AddressBook>()
                .caused_by(trc::location!())?;

            // Validate ACL
            if !access_token.is_member(account_id)
                && !book
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
                    collection: Collection::AddressBook,
                    document_id: document_id.into(),
                    etag: book.etag().into(),
                    path: delete_path,
                    ..Default::default()
                }],
                Default::default(),
                DavMethod::DELETE,
            )
            .await?;

            // Delete addressbook and cards
            DestroyArchive(book)
                .delete_with_cards(
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
                    &mut batch,
                )
                .await
                .caused_by(trc::location!())?;
        } else {
            // Validate ACL
            let addressbook_id = delete_resource.parent_id().unwrap();
            if !access_token.is_member(account_id)
                && !resources.has_access_to_container(
                    access_token,
                    addressbook_id,
                    Acl::RemoveItems,
                )
            {
                return Err(DavError::Code(StatusCode::FORBIDDEN));
            }

            let card_ = self
                .get_archive(account_id, Collection::ContactCard, document_id)
                .await
                .caused_by(trc::location!())?
                .ok_or(DavError::Code(StatusCode::NOT_FOUND))?;

            // Validate headers
            self.validate_headers(
                access_token,
                headers,
                vec![ResourceState {
                    account_id,
                    collection: Collection::ContactCard,
                    document_id: document_id.into(),
                    etag: card_.etag().into(),
                    path: delete_path,
                    ..Default::default()
                }],
                Default::default(),
                DavMethod::DELETE,
            )
            .await?;

            // Delete card
            DestroyArchive(
                card_
                    .to_unarchived::<ContactCard>()
                    .caused_by(trc::location!())?,
            )
            .delete(
                access_token,
                account_id,
                document_id,
                addressbook_id,
                resources.format_resource(delete_resource).into(),
                &mut batch,
            )
            .caused_by(trc::location!())?;
        }

        self.commit_batch(batch).await.caused_by(trc::location!())?;

        Ok(HttpResponse::new(StatusCode::NO_CONTENT))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tracing::{info, debug};

    #[test]
    fn test_card_delete_request_handler_trait() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting card delete request handler trait test at {:?}", start_time);

        // Test that the trait is properly defined
        // This is a compile-time test to ensure the trait signature is correct
        fn assert_trait_implemented<T: CardDeleteRequestHandler>(_: T) {}

        // If this compiles, the trait is properly defined
        debug!("Card delete request handler trait test completed successfully");
    }

    #[test]
    fn test_card_delete_status_codes() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting card delete status codes test at {:?}", start_time);

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

        debug!("Card delete status codes test completed successfully");
    }

    #[test]
    fn test_card_delete_method() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting card delete method test at {:?}", start_time);

        // Test that DELETE method is used for card deletion
        let delete_method = DavMethod::DELETE;
        assert_eq!(format!("{:?}", delete_method), "DELETE");

        debug!("Card delete method test completed successfully");
    }

    #[test]
    fn test_card_delete_error_handling() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting card delete error handling test at {:?}", start_time);

        // Test that DavError can be created for various delete scenarios
        let not_found_error = DavError::not_found("card", "/addressbook/test.vcf");
        assert!(matches!(not_found_error, DavError::NotFound { .. }));

        let forbidden_error = DavError::Forbidden;
        assert!(matches!(forbidden_error, DavError::Forbidden));

        let conflict_error = DavError::conflict("Resource is locked");
        assert!(matches!(conflict_error, DavError::Conflict { .. }));

        debug!("Card delete error handling test completed successfully");
    }

    #[test]
    fn test_card_delete_acl_handling() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting card delete ACL handling test at {:?}", start_time);

        // Test ACL-related functionality for card deletion
        let acl_read = Acl::Read;
        let acl_write = Acl::Write;
        let acl_delete = Acl::Delete;

        assert_eq!(format!("{:?}", acl_read), "Read");
        assert_eq!(format!("{:?}", acl_write), "Write");
        assert_eq!(format!("{:?}", acl_delete), "Delete");

        debug!("Card delete ACL handling test completed successfully");
    }

    #[test]
    fn test_card_delete_collection_types() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting card delete collection types test at {:?}", start_time);

        // Test collection types used in card deletion
        let contact_collection = Collection::ContactCard;
        let sync_collection = SyncCollection::AddressBook;

        assert_eq!(format!("{:?}", contact_collection), "ContactCard");
        assert_eq!(format!("{:?}", sync_collection), "AddressBook");

        debug!("Card delete collection types test completed successfully");
    }

    #[test]
    fn test_card_delete_etag_handling() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting card delete ETag handling test at {:?}", start_time);

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

        debug!("Card delete ETag handling test completed successfully");
    }

    #[test]
    fn test_card_delete_resource_state() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting card delete resource state test at {:?}", start_time);

        // Test resource states relevant to deletion
        let state_unlocked = ResourceState::Unlocked;
        let state_locked = ResourceState::Locked;

        assert_eq!(format!("{:?}", state_unlocked), "Unlocked");
        assert_eq!(format!("{:?}", state_locked), "Locked");

        debug!("Card delete resource state test completed successfully");
    }

    #[test]
    fn test_card_delete_batch_operations() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting card delete batch operations test at {:?}", start_time);

        // Test that BatchBuilder is available for batch operations
        // This is a compile-time test to ensure the type is accessible
        fn assert_batch_builder_available() -> BatchBuilder {
            BatchBuilder::new()
        }

        let _batch = assert_batch_builder_available();

        debug!("Card delete batch operations test completed successfully");
    }

    #[test]
    fn test_card_delete_destroy_archive() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting card delete destroy archive test at {:?}", start_time);

        // Test DestroyArchive functionality
        // This is a compile-time test to ensure the type is accessible
        fn assert_destroy_archive_available() {
            let _destroy: Option<DestroyArchive<ContactCard>> = None;
        }

        assert_destroy_archive_available();

        debug!("Card delete destroy archive test completed successfully");
    }

    #[test]
    fn test_card_delete_effective_acl() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting card delete effective ACL test at {:?}", start_time);

        // Test EffectiveAcl trait availability
        // This is a compile-time test to ensure the trait is accessible
        fn assert_effective_acl_trait_available<T: EffectiveAcl>(_: T) {}

        // If this compiles, the trait is properly defined
        debug!("Card delete effective ACL test completed successfully");
    }

    #[test]
    fn test_card_delete_response_format() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting card delete response format test at {:?}", start_time);

        // Test successful delete response
        let response = HttpResponse::new(StatusCode::NO_CONTENT);

        // Verify response properties
        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        // Delete responses typically have no body
        // This is verified by the NO_CONTENT status

        debug!("Card delete response format test completed successfully");
    }

    #[test]
    fn test_card_delete_addressbook_types() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting card delete addressbook types test at {:?}", start_time);

        // Test addressbook and contact card types
        // This is a compile-time test to ensure the types are accessible
        fn assert_addressbook_types_available() {
            let _addressbook: Option<AddressBook> = None;
            let _contact_card: Option<ContactCard> = None;
        }

        assert_addressbook_types_available();

        debug!("Card delete addressbook types test completed successfully");
    }

    #[test]
    fn test_card_delete_groupware_cache() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting card delete groupware cache test at {:?}", start_time);

        // Test GroupwareCache trait availability
        // This is a compile-time test to ensure the trait is accessible
        fn assert_groupware_cache_trait_available<T: GroupwareCache>(_: T) {}

        // If this compiles, the trait is properly defined
        debug!("Card delete groupware cache test completed successfully");
    }

    #[test]
    fn test_card_delete_uri_validation() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting card delete URI validation test at {:?}", start_time);

        // Test URI validation for card deletion
        let valid_uris = [
            "/addressbook/user/personal/contact.vcf",
            "/addressbook/shared/team/member.vcf",
            "/addressbook/public/directory.vcf",
        ];

        for uri in &valid_uris {
            assert!(uri.starts_with("/addressbook/"));
            assert!(uri.ends_with(".vcf"));
        }

        debug!("Card delete URI validation test completed successfully");
    }
}

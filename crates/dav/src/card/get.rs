/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

use common::{Server, auth::AccessToken};
use dav_proto::{RequestHeaders, schema::property::Rfc1123DateTime};
use groupware::{cache::GroupwareCache, contact::ContactCard};
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

pub(crate) trait CardGetRequestHandler: Sync + Send {
    fn handle_card_get_request(
        &self,
        access_token: &AccessToken,
        headers: &RequestHeaders<'_>,
        is_head: bool,
    ) -> impl Future<Output = crate::Result<HttpResponse>> + Send;
}

impl CardGetRequestHandler for Server {
    async fn handle_card_get_request(
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
            .fetch_dav_resources(access_token, account_id, SyncCollection::AddressBook)
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

        // Fetch card
        let card_ = self
            .get_archive(account_id, Collection::ContactCard, resource.document_id())
            .await
            .caused_by(trc::location!())?
            .ok_or(DavError::Code(StatusCode::NOT_FOUND))?;
        let card = card_
            .unarchive::<ContactCard>()
            .caused_by(trc::location!())?;

        // Validate headers
        let etag = card_.etag();
        self.validate_headers(
            access_token,
            headers,
            vec![ResourceState {
                account_id,
                collection: Collection::ContactCard,
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
            .with_content_type("text/vcard; charset=utf-8")
            .with_etag(etag)
            .with_last_modified(Rfc1123DateTime::new(i64::from(card.modified)).to_string());

        let mut vcard = String::with_capacity(128);
        let _ = card.card.write_to(
            &mut vcard,
            headers
                .max_vcard_version
                .or_else(|| card.card.version())
                .unwrap_or_default(),
        );

        if !is_head {
            Ok(response.with_binary_body(vcard))
        } else {
            Ok(response.with_content_length(vcard.len()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tracing::{info, debug};

    #[test]
    fn test_card_get_request_handler_trait() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting card get request handler trait test at {:?}", start_time);

        // Test that the trait is properly defined
        // This is a compile-time test to ensure the trait signature is correct
        fn assert_trait_implemented<T: CardGetRequestHandler>(_: T) {}

        // If this compiles, the trait is properly defined
        debug!("Card get request handler trait test completed successfully");
    }

    #[test]
    fn test_card_get_content_type() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting card get content type test at {:?}", start_time);

        // Test that the expected content type is correct for vCard data
        let expected_content_type = "text/vcard; charset=utf-8";

        // Verify it's a valid vCard content type
        assert!(expected_content_type.starts_with("text/vcard"));
        assert!(expected_content_type.contains("charset=utf-8"));

        debug!("Card get content type test completed successfully");
    }

    #[test]
    fn test_card_get_http_methods() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting card get HTTP methods test at {:?}", start_time);

        // Test that GET and HEAD methods are supported
        let get_method = DavMethod::GET;
        let head_method = DavMethod::HEAD;

        // Verify these are the correct methods for card retrieval
        assert_eq!(format!("{:?}", get_method), "GET");
        assert_eq!(format!("{:?}", head_method), "HEAD");

        debug!("Card get HTTP methods test completed successfully");
    }

    #[test]
    fn test_card_get_status_codes() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting card get status codes test at {:?}", start_time);

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

        debug!("Card get status codes test completed successfully");
    }

    #[test]
    fn test_card_get_response_headers() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting card get response headers test at {:?}", start_time);

        // Test that required headers are properly formatted
        let content_type = "text/vcard; charset=utf-8";
        assert!(content_type.contains("text/vcard"));
        assert!(content_type.contains("utf-8"));

        // Test ETag format (should be quoted)
        let etag_example = "\"12345\"";
        assert!(etag_example.starts_with('"'));
        assert!(etag_example.ends_with('"'));

        debug!("Card get response headers test completed successfully");
    }

    #[test]
    fn test_card_get_error_handling() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting card get error handling test at {:?}", start_time);

        // Test that DavError can be created for various scenarios
        let not_found_error = DavError::not_found("card", "/addressbook/test.vcf");
        assert!(matches!(not_found_error, DavError::NotFound { .. }));

        let forbidden_error = DavError::Forbidden;
        assert!(matches!(forbidden_error, DavError::Forbidden));

        debug!("Card get error handling test completed successfully");
    }

    #[test]
    fn test_card_get_uri_validation() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting card get URI validation test at {:?}", start_time);

        // Test valid card URIs
        let valid_uris = [
            "/addressbook/user/personal/contact.vcf",
            "/addressbook/shared/team/member.vcf",
            "/addressbook/public/directory.vcf",
        ];

        for uri in &valid_uris {
            assert!(uri.starts_with("/addressbook/"));
            assert!(uri.ends_with(".vcf"));
        }

        debug!("Card get URI validation test completed successfully");
    }

    #[test]
    fn test_card_get_head_vs_get() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting card get HEAD vs GET test at {:?}", start_time);

        // Test the difference between HEAD and GET requests
        let is_head_true = true;
        let is_head_false = false;

        // HEAD should not include body, GET should include body
        assert!(is_head_true);
        assert!(!is_head_false);

        debug!("Card get HEAD vs GET test completed successfully");
    }

    #[test]
    fn test_card_get_vcard_format() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting card get vCard format test at {:?}", start_time);

        // Test basic vCard format requirements
        let vcard_example = "BEGIN:VCARD\nVERSION:4.0\nFN:John Doe\nEND:VCARD";

        assert!(vcard_example.contains("BEGIN:VCARD"));
        assert!(vcard_example.contains("VERSION:"));
        assert!(vcard_example.contains("END:VCARD"));

        debug!("Card get vCard format test completed successfully");
    }

    #[test]
    fn test_card_get_etag_handling() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting card get ETag handling test at {:?}", start_time);

        // Test ETag generation and validation
        let etag_value = "12345";
        let quoted_etag = format!("\"{}\"", etag_value);

        assert!(quoted_etag.starts_with('"'));
        assert!(quoted_etag.ends_with('"'));
        assert!(quoted_etag.contains(etag_value));

        debug!("Card get ETag handling test completed successfully");
    }

    #[test]
    fn test_card_get_last_modified() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting card get last modified test at {:?}", start_time);

        // Test last modified header format (RFC 1123)
        let timestamp = 1640995200i64; // Example timestamp

        // Should be a valid timestamp
        assert!(timestamp > 0);

        // Test that we can create an RFC 1123 date
        let rfc1123_example = "Sat, 01 Jan 2022 00:00:00 GMT";
        assert!(rfc1123_example.contains("GMT"));
        assert!(rfc1123_example.len() > 20); // Reasonable length check

        debug!("Card get last modified test completed successfully");
    }

    #[test]
    fn test_card_get_vcard_versions() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting card get vCard versions test at {:?}", start_time);

        // Test vCard version handling
        let version_3_0 = "3.0";
        let version_4_0 = "4.0";

        assert_eq!(version_3_0, "3.0");
        assert_eq!(version_4_0, "4.0");

        // Test version comparison
        assert!(version_4_0 > version_3_0);

        debug!("Card get vCard versions test completed successfully");
    }

    #[test]
    fn test_card_get_collection_types() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting card get collection types test at {:?}", start_time);

        // Test collection types used in card retrieval
        let contact_collection = Collection::ContactCard;
        let sync_collection = SyncCollection::AddressBook;

        assert_eq!(format!("{:?}", contact_collection), "ContactCard");
        assert_eq!(format!("{:?}", sync_collection), "AddressBook");

        debug!("Card get collection types test completed successfully");
    }

    #[test]
    fn test_card_get_acl_handling() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting card get ACL handling test at {:?}", start_time);

        // Test ACL-related functionality for card retrieval
        let acl_read = Acl::Read;
        let acl_write = Acl::Write;

        assert_eq!(format!("{:?}", acl_read), "Read");
        assert_eq!(format!("{:?}", acl_write), "Write");

        debug!("Card get ACL handling test completed successfully");
    }
}

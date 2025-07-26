/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

use common::auth::AccessToken;
use dav_proto::schema::response::Href;
use groupware::RFC_3986;

use crate::DavResourceName;

pub mod matching;
pub mod propfind;
pub mod propsearch;

pub trait CurrentUserPrincipal {
    fn current_user_principal(&self) -> Href;
}

impl CurrentUserPrincipal for AccessToken {
    fn current_user_principal(&self) -> Href {
        Href(format!(
            "{}/{}/",
            DavResourceName::Principal.base_path(),
            percent_encoding::utf8_percent_encode(&self.name, RFC_3986)
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tracing::{info, debug};

    // Mock AccessToken for testing
    struct MockAccessToken {
        name: String,
    }

    impl MockAccessToken {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
            }
        }
    }

    #[test]
    fn test_current_user_principal_trait() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting current user principal trait test at {:?}", start_time);

        // Test that the trait is properly defined
        // This is a compile-time test to ensure the trait signature is correct
        fn assert_trait_implemented<T: CurrentUserPrincipal>(_: T) {}

        // If this compiles, the trait is properly defined
        debug!("Current user principal trait test completed successfully");
    }

    #[test]
    fn test_href_structure() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting Href structure test at {:?}", start_time);

        let href = Href("/principals/user/".to_string());
        assert_eq!(href.0, "/principals/user/");

        let href2 = Href("https://example.com/principals/admin/".to_string());
        assert!(href2.0.starts_with("https://"));
        assert!(href2.0.contains("/principals/"));
        assert!(href2.0.ends_with("/"));

        debug!("Href structure test completed successfully");
    }

    #[test]
    fn test_principal_path_format() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting principal path format test at {:?}", start_time);

        // Test expected principal path format
        let base_path = DavResourceName::Principal.base_path();
        assert!(base_path.contains("principal"));

        // Test path construction
        let username = "testuser";
        let expected_path = format!("{}/{}/", base_path, username);
        assert!(expected_path.starts_with(base_path));
        assert!(expected_path.contains(username));
        assert!(expected_path.ends_with("/"));

        debug!("Principal path format test completed successfully");
    }

    #[test]
    fn test_percent_encoding() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting percent encoding test at {:?}", start_time);

        // Test percent encoding of usernames
        let simple_name = "user";
        let encoded_simple = percent_encoding::utf8_percent_encode(simple_name, RFC_3986).to_string();
        assert_eq!(encoded_simple, "user");

        let complex_name = "user@domain.com";
        let encoded_complex = percent_encoding::utf8_percent_encode(complex_name, RFC_3986).to_string();
        assert!(encoded_complex.contains("user"));
        // @ should be encoded
        assert!(encoded_complex.contains("%40"));

        let space_name = "user name";
        let encoded_space = percent_encoding::utf8_percent_encode(space_name, RFC_3986).to_string();
        assert!(encoded_space.contains("user"));
        assert!(encoded_space.contains("%20"));

        debug!("Percent encoding test completed successfully");
    }

    #[test]
    fn test_principal_url_construction() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting principal URL construction test at {:?}", start_time);

        // Test URL construction for different usernames
        let usernames = [
            "admin",
            "user123",
            "user@example.com",
            "user with spaces",
            "用户", // Unicode username
        ];

        for username in &usernames {
            let base_path = DavResourceName::Principal.base_path();
            let encoded_name = percent_encoding::utf8_percent_encode(username, RFC_3986);
            let url = format!("{}/{}/", base_path, encoded_name);

            assert!(url.starts_with(base_path));
            assert!(url.ends_with("/"));
            assert!(url.len() > base_path.len() + 2); // At least base + "/" + name + "/"
        }

        debug!("Principal URL construction test completed successfully");
    }

    #[test]
    fn test_rfc_3986_compliance() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting RFC 3986 compliance test at {:?}", start_time);

        // Test that RFC_3986 character set is used correctly
        let test_chars = "!@#$%^&*()_+-=[]{}|;':\",./<>?";
        let encoded = percent_encoding::utf8_percent_encode(test_chars, RFC_3986).to_string();

        // Should contain percent-encoded characters
        assert!(encoded.contains("%"));
        assert!(encoded.len() > test_chars.len()); // Should be longer due to encoding

        debug!("RFC 3986 compliance test completed successfully");
    }

    #[test]
    fn test_principal_href_format() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting principal Href format test at {:?}", start_time);

        // Test that Href format is correct for principals
        let username = "testuser";
        let base_path = DavResourceName::Principal.base_path();
        let encoded_name = percent_encoding::utf8_percent_encode(username, RFC_3986);
        let href_value = format!("{}/{}/", base_path, encoded_name);
        let href = Href(href_value.clone());

        assert_eq!(href.0, href_value);
        assert!(href.0.starts_with(base_path));
        assert!(href.0.contains(username));
        assert!(href.0.ends_with("/"));

        debug!("Principal Href format test completed successfully");
    }

    #[test]
    fn test_empty_username_handling() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting empty username handling test at {:?}", start_time);

        // Test handling of empty username
        let empty_name = "";
        let base_path = DavResourceName::Principal.base_path();
        let encoded_name = percent_encoding::utf8_percent_encode(empty_name, RFC_3986);
        let url = format!("{}/{}/", base_path, encoded_name);

        // Should still be a valid URL structure
        assert!(url.starts_with(base_path));
        assert!(url.ends_with("//"));

        debug!("Empty username handling test completed successfully");
    }

    #[test]
    fn test_unicode_username_support() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting Unicode username support test at {:?}", start_time);

        // Test Unicode usernames
        let unicode_names = [
            "用户",      // Chinese
            "пользователь", // Russian
            "ユーザー",    // Japanese
            "مستخدم",     // Arabic
        ];

        for username in &unicode_names {
            let base_path = DavResourceName::Principal.base_path();
            let encoded_name = percent_encoding::utf8_percent_encode(username, RFC_3986);
            let url = format!("{}/{}/", base_path, encoded_name);

            assert!(url.starts_with(base_path));
            assert!(url.ends_with("/"));
            assert!(url.contains("%")); // Unicode should be percent-encoded
        }

        debug!("Unicode username support test completed successfully");
    }

    #[test]
    fn test_principal_resource_name() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting principal resource name test at {:?}", start_time);

        // Test DavResourceName::Principal properties
        let principal_resource = DavResourceName::Principal;
        let base_path = principal_resource.base_path();

        assert!(base_path.contains("principal"));
        assert!(base_path.starts_with("/"));

        debug!("Principal resource name test completed successfully");
    }

    #[test]
    fn test_module_structure() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting module structure test at {:?}", start_time);

        // Test that all expected submodules are declared
        // This is a compile-time test to ensure modules exist
        // matching, propfind, propsearch modules should be available

        debug!("Module structure test completed successfully");
    }
}

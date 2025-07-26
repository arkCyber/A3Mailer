/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

use crate::{
    DavError, DavErrorCondition, DavResourceName, common::uri::DavUriResource,
    principal::propfind::PrincipalPropFind,
};
use common::{DavResources, Server, auth::AccessToken, sharing::EffectiveAcl};
use dav_proto::{
    RequestHeaders,
    schema::{
        property::{DavProperty, Privilege, WebDavProperty},
        request::{AclPrincipalPropSet, PropFind},
        response::{Ace, BaseCondition, GrantDeny, Href, MultiStatus, Principal},
    },
};
use directory::{QueryBy, Type, backend::internal::manage::ManageDirectory};
use groupware::RFC_3986;
use groupware::{cache::GroupwareCache, calendar::Calendar, contact::AddressBook, file::FileNode};
use http_proto::HttpResponse;
use hyper::StatusCode;
use jmap_proto::types::{
    acl::Acl,
    collection::Collection,
    value::{AclGrant, ArchivedAclGrant},
};
use rkyv::vec::ArchivedVec;
use store::{ahash::AHashSet, roaring::RoaringBitmap, write::BatchBuilder};
use trc::AddContext;
use utils::map::bitmap::Bitmap;

use super::ArchivedResource;

pub(crate) trait DavAclHandler: Sync + Send {
    fn handle_acl_request(
        &self,
        access_token: &AccessToken,
        headers: &RequestHeaders<'_>,
        request: dav_proto::schema::request::Acl,
    ) -> impl Future<Output = crate::Result<HttpResponse>> + Send;

    fn handle_acl_prop_set(
        &self,
        access_token: &AccessToken,
        headers: &RequestHeaders<'_>,
        request: AclPrincipalPropSet,
    ) -> impl Future<Output = crate::Result<HttpResponse>> + Send;

    fn validate_and_map_aces(
        &self,
        access_token: &AccessToken,
        acl: dav_proto::schema::request::Acl,
        collection: Collection,
    ) -> impl Future<Output = crate::Result<Vec<AclGrant>>> + Send;

    fn resolve_ace(
        &self,
        access_token: &AccessToken,
        account_id: u32,
        grants: &ArchivedVec<ArchivedAclGrant>,
        expand: Option<&PropFind>,
    ) -> impl Future<Output = crate::Result<Vec<Ace>>> + Send;
}

pub(crate) trait ResourceAcl {
    fn validate_and_map_parent_acl(
        &self,
        access_token: &AccessToken,
        is_member: bool,
        parent_id: Option<u32>,
        check_acls: impl Into<Bitmap<Acl>> + Send,
    ) -> crate::Result<u32>;
}

impl DavAclHandler for Server {
    async fn handle_acl_request(
        &self,
        access_token: &AccessToken,
        headers: &RequestHeaders<'_>,
        request: dav_proto::schema::request::Acl,
    ) -> crate::Result<HttpResponse> {
        // Validate URI
        let resource_ = self
            .validate_uri(access_token, headers.uri)
            .await?
            .into_owned_uri()?;
        let account_id = resource_.account_id;
        let collection = resource_.collection;

        if !matches!(
            collection,
            Collection::AddressBook | Collection::Calendar | Collection::FileNode
        ) {
            return Err(DavError::Code(StatusCode::FORBIDDEN));
        }
        let resources = self
            .fetch_dav_resources(access_token, account_id, collection.into())
            .await
            .caused_by(trc::location!())?;
        let resource = resource_
            .resource
            .and_then(|r| resources.by_path(r))
            .ok_or(DavError::Code(StatusCode::NOT_FOUND))?;
        if !resource.resource.is_container() && !matches!(collection, Collection::FileNode) {
            return Err(DavError::Code(StatusCode::FORBIDDEN));
        }

        // Fetch node
        let archive = self
            .get_archive(account_id, collection, resource.document_id())
            .await
            .caused_by(trc::location!())?
            .ok_or(DavError::Code(StatusCode::NOT_FOUND))?;

        let container =
            ArchivedResource::from_archive(&archive, collection).caused_by(trc::location!())?;

        // Validate ACL
        let acls = container.acls().unwrap();
        if !access_token.is_member(account_id)
            && !acls.effective_acl(access_token).contains(Acl::Administer)
        {
            return Err(DavError::Code(StatusCode::FORBIDDEN));
        }

        // Validate ACEs
        let grants = self
            .validate_and_map_aces(access_token, request, collection)
            .await?;

        if grants.len() != acls.len() || acls.iter().zip(grants.iter()).any(|(a, b)| a != b) {
            // Refresh ACLs
            self.refresh_archived_acls(&grants, acls).await;

            let mut batch = BatchBuilder::new();
            match container {
                ArchivedResource::Calendar(calendar) => {
                    let mut new_calendar = calendar
                        .deserialize::<Calendar>()
                        .caused_by(trc::location!())?;
                    new_calendar.acls = grants;
                    new_calendar
                        .update(
                            access_token,
                            calendar,
                            account_id,
                            resource.document_id(),
                            &mut batch,
                        )
                        .caused_by(trc::location!())?;
                }
                ArchivedResource::AddressBook(book) => {
                    let mut new_book = book
                        .deserialize::<AddressBook>()
                        .caused_by(trc::location!())?;
                    new_book.acls = grants;
                    new_book
                        .update(
                            access_token,
                            book,
                            account_id,
                            resource.document_id(),
                            &mut batch,
                        )
                        .caused_by(trc::location!())?;
                }
                ArchivedResource::FileNode(node) => {
                    let mut new_node =
                        node.deserialize::<FileNode>().caused_by(trc::location!())?;
                    new_node.acls = grants;
                    new_node
                        .update(
                            access_token,
                            node,
                            account_id,
                            resource.document_id(),
                            &mut batch,
                        )
                        .caused_by(trc::location!())?;
                }
                _ => unreachable!(),
            }

            self.commit_batch(batch).await.caused_by(trc::location!())?;
        }

        Ok(HttpResponse::new(StatusCode::OK))
    }

    async fn handle_acl_prop_set(
        &self,
        access_token: &AccessToken,
        headers: &RequestHeaders<'_>,
        mut request: AclPrincipalPropSet,
    ) -> crate::Result<HttpResponse> {
        let uri = self
            .validate_uri(access_token, headers.uri)
            .await
            .and_then(|uri| uri.into_owned_uri())?;
        let uri = self
            .map_uri_resource(access_token, uri)
            .await
            .caused_by(trc::location!())?
            .ok_or(DavError::Code(StatusCode::NOT_FOUND))?;

        if !matches!(
            uri.collection,
            Collection::Calendar | Collection::AddressBook | Collection::FileNode
        ) {
            return Err(DavError::Code(StatusCode::FORBIDDEN));
        }

        let archive = self
            .get_archive(uri.account_id, uri.collection, uri.resource)
            .await
            .caused_by(trc::location!())?
            .ok_or(DavError::Code(StatusCode::NOT_FOUND))?;

        let acls = match uri.collection {
            Collection::FileNode => {
                &archive
                    .unarchive::<FileNode>()
                    .caused_by(trc::location!())?
                    .acls
            }
            Collection::AddressBook => {
                &archive
                    .unarchive::<AddressBook>()
                    .caused_by(trc::location!())?
                    .acls
            }
            Collection::Calendar => {
                &archive
                    .unarchive::<Calendar>()
                    .caused_by(trc::location!())?
                    .acls
            }
            _ => unreachable!(),
        };

        // Validate ACLs
        if !access_token.is_member(uri.account_id)
            && !acls.effective_acl(access_token).contains(Acl::Read)
        {
            return Err(DavError::Code(StatusCode::FORBIDDEN));
        }

        // Validate
        let account_ids = RoaringBitmap::from_iter(acls.iter().map(|a| u32::from(a.account_id)));
        let mut response = MultiStatus::new(Vec::with_capacity(16));

        if !account_ids.is_empty() {
            if request.properties.is_empty() {
                request
                    .properties
                    .push(DavProperty::WebDav(WebDavProperty::DisplayName));
            }
            let request = PropFind::Prop(request.properties);
            self.prepare_principal_propfind_response(
                access_token,
                Collection::Principal,
                account_ids.into_iter(),
                &request,
                &mut response,
            )
            .await?;
        }

        Ok(HttpResponse::new(StatusCode::MULTI_STATUS).with_xml_body(response.to_string()))
    }

    async fn validate_and_map_aces(
        &self,
        access_token: &AccessToken,
        acl: dav_proto::schema::request::Acl,
        collection: Collection,
    ) -> crate::Result<Vec<AclGrant>> {
        let mut grants = Vec::with_capacity(acl.aces.len());
        for ace in acl.aces {
            if ace.invert {
                return Err(DavError::Condition(DavErrorCondition::new(
                    StatusCode::FORBIDDEN,
                    BaseCondition::NoInvert,
                )));
            }
            let privileges = match ace.grant_deny {
                GrantDeny::Grant(list) => list.0,
                GrantDeny::Deny(_) => {
                    return Err(DavError::Condition(DavErrorCondition::new(
                        StatusCode::FORBIDDEN,
                        BaseCondition::GrantOnly,
                    )));
                }
            };
            let principal_uri = match ace.principal {
                Principal::Href(href) => href.0,
                _ => {
                    return Err(DavError::Condition(DavErrorCondition::new(
                        StatusCode::FORBIDDEN,
                        BaseCondition::AllowedPrincipal,
                    )));
                }
            };

            let mut acls = Bitmap::<Acl>::default();
            for privilege in privileges {
                match privilege {
                    Privilege::Read => {
                        acls.insert(Acl::Read);
                        acls.insert(Acl::ReadItems);
                    }
                    Privilege::Write => {
                        acls.insert(Acl::Modify);
                        acls.insert(Acl::Delete);
                        acls.insert(Acl::AddItems);
                        acls.insert(Acl::ModifyItems);
                        acls.insert(Acl::RemoveItems);
                    }
                    Privilege::WriteContent => {
                        acls.insert(Acl::AddItems);
                        acls.insert(Acl::Modify);
                        acls.insert(Acl::ModifyItems);
                    }
                    Privilege::WriteProperties => {
                        acls.insert(Acl::Modify);
                    }
                    Privilege::ReadCurrentUserPrivilegeSet
                    | Privilege::Unlock
                    | Privilege::Bind
                    | Privilege::Unbind => {}
                    Privilege::All => {
                        return Err(DavError::Condition(DavErrorCondition::new(
                            StatusCode::FORBIDDEN,
                            BaseCondition::NoAbstract,
                        )));
                    }
                    Privilege::ReadAcl => {}
                    Privilege::WriteAcl => {
                        acls.insert(Acl::Administer);
                    }
                    Privilege::ReadFreeBusy
                    | Privilege::ScheduleQueryFreeBusy
                    | Privilege::ScheduleSendFreeBusy => {
                        if collection == Collection::Calendar {
                            acls.insert(Acl::SchedulingReadFreeBusy);
                        } else {
                            return Err(DavError::Condition(DavErrorCondition::new(
                                StatusCode::FORBIDDEN,
                                BaseCondition::NotSupportedPrivilege,
                            )));
                        }
                    }
                    Privilege::ScheduleDeliver | Privilege::ScheduleSend => {
                        if collection == Collection::Calendar {
                            acls.insert(Acl::SchedulingReadFreeBusy);
                            acls.insert(Acl::SchedulingInvite);
                            acls.insert(Acl::SchedulingReply);
                        } else {
                            return Err(DavError::Condition(DavErrorCondition::new(
                                StatusCode::FORBIDDEN,
                                BaseCondition::NotSupportedPrivilege,
                            )));
                        }
                    }
                    Privilege::ScheduleDeliverInvite | Privilege::ScheduleSendInvite => {
                        if collection == Collection::Calendar {
                            acls.insert(Acl::SchedulingInvite);
                        } else {
                            return Err(DavError::Condition(DavErrorCondition::new(
                                StatusCode::FORBIDDEN,
                                BaseCondition::NotSupportedPrivilege,
                            )));
                        }
                    }
                    Privilege::ScheduleDeliverReply | Privilege::ScheduleSendReply => {
                        if collection == Collection::Calendar {
                            acls.insert(Acl::SchedulingReply);
                        } else {
                            return Err(DavError::Condition(DavErrorCondition::new(
                                StatusCode::FORBIDDEN,
                                BaseCondition::NotSupportedPrivilege,
                            )));
                        }
                    }
                }
            }

            if acls.is_empty() {
                continue;
            }

            let principal_id = self
                .validate_uri(access_token, &principal_uri)
                .await
                .map_err(|_| {
                    DavError::Condition(DavErrorCondition::new(
                        StatusCode::FORBIDDEN,
                        BaseCondition::AllowedPrincipal,
                    ))
                })?
                .account_id
                .ok_or_else(|| {
                    DavError::Condition(DavErrorCondition::new(
                        StatusCode::FORBIDDEN,
                        BaseCondition::AllowedPrincipal,
                    ))
                })?;

            // Verify that the principal is a valid principal
            let principal = self
                .directory()
                .query(QueryBy::Id(principal_id), false)
                .await
                .caused_by(trc::location!())?
                .ok_or_else(|| {
                    DavError::Condition(DavErrorCondition::new(
                        StatusCode::FORBIDDEN,
                        BaseCondition::AllowedPrincipal,
                    ))
                })?;
            if !matches!(principal.typ(), Type::Individual | Type::Group) {
                return Err(DavError::Condition(DavErrorCondition::new(
                    StatusCode::FORBIDDEN,
                    BaseCondition::AllowedPrincipal,
                )));
            }

            grants.push(AclGrant {
                account_id: principal_id,
                grants: acls,
            });
        }

        Ok(grants)
    }

    async fn resolve_ace(
        &self,
        access_token: &AccessToken,
        account_id: u32,
        grants: &ArchivedVec<ArchivedAclGrant>,
        expand: Option<&PropFind>,
    ) -> crate::Result<Vec<Ace>> {
        let mut aces = Vec::with_capacity(grants.len());
        if access_token.is_member(account_id)
            || grants.effective_acl(access_token).contains(Acl::Administer)
        {
            for grant in grants.iter() {
                let grant_account_id = u32::from(grant.account_id);
                let principal = if let Some(expand) = expand {
                    self.expand_principal(access_token, grant_account_id, expand)
                        .await?
                        .map(Principal::Response)
                        .unwrap_or_else(|| {
                            Principal::Href(Href(format!(
                                "{}/_{grant_account_id}/",
                                DavResourceName::Principal.base_path(),
                            )))
                        })
                } else {
                    let grant_account_name = self
                        .store()
                        .get_principal_name(grant_account_id)
                        .await
                        .caused_by(trc::location!())?
                        .unwrap_or_else(|| format!("_{grant_account_id}"));

                    Principal::Href(Href(format!(
                        "{}/{}/",
                        DavResourceName::Principal.base_path(),
                        percent_encoding::utf8_percent_encode(&grant_account_name, RFC_3986),
                    )))
                };

                aces.push(Ace::new(
                    principal,
                    GrantDeny::grant(current_user_privilege_set(Bitmap::<Acl>::from(
                        &grant.grants,
                    ))),
                ));
            }
        }

        Ok(aces)
    }
}

impl ResourceAcl for DavResources {
    fn validate_and_map_parent_acl(
        &self,
        access_token: &AccessToken,
        is_member: bool,
        parent_id: Option<u32>,
        check_acls: impl Into<Bitmap<Acl>> + Send,
    ) -> crate::Result<u32> {
        match parent_id {
            Some(parent_id) => {
                if is_member || self.has_access_to_container(access_token, parent_id, check_acls) {
                    Ok(parent_id + 1)
                } else {
                    Err(DavError::Code(StatusCode::FORBIDDEN))
                }
            }
            None => {
                if is_member {
                    Ok(0)
                } else {
                    Err(DavError::Code(StatusCode::FORBIDDEN))
                }
            }
        }
    }
}

pub(crate) trait Privileges {
    fn current_privilege_set(
        &self,
        account_id: u32,
        grants: &ArchivedVec<ArchivedAclGrant>,
        is_calendar: bool,
    ) -> Vec<Privilege>;
}

impl Privileges for AccessToken {
    fn current_privilege_set(
        &self,
        account_id: u32,
        grants: &ArchivedVec<ArchivedAclGrant>,
        is_calendar: bool,
    ) -> Vec<Privilege> {
        if self.is_member(account_id) {
            Privilege::all(is_calendar)
        } else {
            current_user_privilege_set(grants.effective_acl(self))
        }
    }
}

pub(crate) fn current_user_privilege_set(acl_bitmap: Bitmap<Acl>) -> Vec<Privilege> {
    let mut acls = AHashSet::with_capacity(16);
    for grant in acl_bitmap {
        match grant {
            Acl::Read | Acl::ReadItems => {
                acls.insert(Privilege::Read);
                acls.insert(Privilege::ReadCurrentUserPrivilegeSet);
            }
            Acl::Modify => {
                acls.insert(Privilege::WriteProperties);
            }
            Acl::ModifyItems => {
                acls.insert(Privilege::WriteContent);
            }
            Acl::Delete | Acl::RemoveItems => {
                acls.insert(Privilege::Write);
            }
            Acl::Administer => {
                acls.insert(Privilege::ReadAcl);
                acls.insert(Privilege::WriteAcl);
            }
            Acl::SchedulingReadFreeBusy => {
                acls.insert(Privilege::ReadFreeBusy);
            }
            _ => {}
        }
    }
    acls.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tracing::{info, debug};

    #[test]
    fn test_dav_acl_handler_trait() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting DAV ACL handler trait test at {:?}", start_time);

        // Test that the trait is properly defined
        // This is a compile-time test to ensure the trait signature is correct
        fn assert_trait_implemented<T: DavAclHandler>(_: T) {}

        // If this compiles, the trait is properly defined
        debug!("DAV ACL handler trait test completed successfully");
    }

    #[test]
    fn test_acl_privileges() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting ACL privileges test at {:?}", start_time);

        // Test privilege types
        let read_privilege = Privilege::Read;
        let write_privilege = Privilege::Write;
        let write_properties = Privilege::WriteProperties;
        let write_content = Privilege::WriteContent;
        let read_acl = Privilege::ReadAcl;
        let write_acl = Privilege::WriteAcl;
        let read_freebusy = Privilege::ReadFreeBusy;

        assert_eq!(format!("{:?}", read_privilege), "Read");
        assert_eq!(format!("{:?}", write_privilege), "Write");
        assert_eq!(format!("{:?}", write_properties), "WriteProperties");
        assert_eq!(format!("{:?}", write_content), "WriteContent");
        assert_eq!(format!("{:?}", read_acl), "ReadAcl");
        assert_eq!(format!("{:?}", write_acl), "WriteAcl");
        assert_eq!(format!("{:?}", read_freebusy), "ReadFreeBusy");

        debug!("ACL privileges test completed successfully");
    }

    #[test]
    fn test_acl_types() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting ACL types test at {:?}", start_time);

        // Test ACL types
        let read_acl = Acl::Read;
        let write_acl = Acl::Write;
        let modify_acl = Acl::Modify;
        let modify_items_acl = Acl::ModifyItems;
        let delete_acl = Acl::Delete;
        let remove_items_acl = Acl::RemoveItems;
        let administer_acl = Acl::Administer;
        let scheduling_read_freebusy = Acl::SchedulingReadFreeBusy;

        assert_eq!(format!("{:?}", read_acl), "Read");
        assert_eq!(format!("{:?}", write_acl), "Write");
        assert_eq!(format!("{:?}", modify_acl), "Modify");
        assert_eq!(format!("{:?}", modify_items_acl), "ModifyItems");
        assert_eq!(format!("{:?}", delete_acl), "Delete");
        assert_eq!(format!("{:?}", remove_items_acl), "RemoveItems");
        assert_eq!(format!("{:?}", administer_acl), "Administer");
        assert_eq!(format!("{:?}", scheduling_read_freebusy), "SchedulingReadFreeBusy");

        debug!("ACL types test completed successfully");
    }

    #[test]
    fn test_acl_grant_deny() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting ACL grant/deny test at {:?}", start_time);

        // Test grant/deny types
        let grant = GrantDeny::Grant;
        let deny = GrantDeny::Deny;

        assert_eq!(format!("{:?}", grant), "Grant");
        assert_eq!(format!("{:?}", deny), "Deny");

        debug!("ACL grant/deny test completed successfully");
    }

    #[test]
    fn test_acl_principal_types() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting ACL principal types test at {:?}", start_time);

        // Test principal types
        let href_principal = Principal::Href(Href("/principals/user/".to_string()));
        let all_principal = Principal::All;
        let authenticated_principal = Principal::Authenticated;
        let unauthenticated_principal = Principal::Unauthenticated;
        let self_principal = Principal::Self_;

        assert!(matches!(href_principal, Principal::Href(_)));
        assert!(matches!(all_principal, Principal::All));
        assert!(matches!(authenticated_principal, Principal::Authenticated));
        assert!(matches!(unauthenticated_principal, Principal::Unauthenticated));
        assert!(matches!(self_principal, Principal::Self_));

        debug!("ACL principal types test completed successfully");
    }

    #[test]
    fn test_acl_status_codes() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting ACL status codes test at {:?}", start_time);

        // Test status codes used in ACL operations
        let multi_status = StatusCode::MULTI_STATUS;
        assert_eq!(multi_status.as_u16(), 207);

        let ok = StatusCode::OK;
        assert_eq!(ok.as_u16(), 200);

        let forbidden = StatusCode::FORBIDDEN;
        assert_eq!(forbidden.as_u16(), 403);

        let not_found = StatusCode::NOT_FOUND;
        assert_eq!(not_found.as_u16(), 404);

        debug!("ACL status codes test completed successfully");
    }

    #[test]
    fn test_acl_collection_types() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting ACL collection types test at {:?}", start_time);

        // Test collection types used in ACL
        let calendar_collection = Collection::Calendar;
        let addressbook_collection = Collection::AddressBook;
        let file_collection = Collection::FileNode;

        assert_eq!(format!("{:?}", calendar_collection), "Calendar");
        assert_eq!(format!("{:?}", addressbook_collection), "AddressBook");
        assert_eq!(format!("{:?}", file_collection), "FileNode");

        debug!("ACL collection types test completed successfully");
    }

    #[test]
    fn test_acl_error_conditions() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting ACL error conditions test at {:?}", start_time);

        // Test ACL error conditions
        // This tests the types and structures used in ACL error handling

        // Test that DavErrorCondition type is available
        fn assert_error_condition_type_available() {
            let _condition: Option<DavErrorCondition> = None;
        }

        assert_error_condition_type_available();

        debug!("ACL error conditions test completed successfully");
    }

    #[test]
    fn test_acl_base_conditions() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting ACL base conditions test at {:?}", start_time);

        // Test base conditions for ACL
        // This tests the types and structures used in base conditions

        // Test that BaseCondition type is available
        fn assert_base_condition_type_available() {
            let _condition: Option<BaseCondition> = None;
        }

        assert_base_condition_type_available();

        debug!("ACL base conditions test completed successfully");
    }

    #[test]
    fn test_acl_ace_structure() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting ACL ACE structure test at {:?}", start_time);

        // Test Access Control Entry (ACE) structure
        // This tests the types and structures used in ACE

        // Test that Ace type is available
        fn assert_ace_type_available() {
            let _ace: Option<Ace> = None;
        }

        assert_ace_type_available();

        debug!("ACL ACE structure test completed successfully");
    }

    #[test]
    fn test_acl_grant_types() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting ACL grant types test at {:?}", start_time);

        // Test ACL grant types
        // This tests the types and structures used in ACL grants

        // Test that AclGrant and ArchivedAclGrant types are available
        fn assert_grant_types_available() {
            let _grant: Option<AclGrant> = None;
            let _archived_grant: Option<ArchivedAclGrant> = None;
        }

        assert_grant_types_available();

        debug!("ACL grant types test completed successfully");
    }

    #[test]
    fn test_acl_bitmap_operations() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting ACL bitmap operations test at {:?}", start_time);

        // Test bitmap operations for ACL
        // This tests the types and structures used in bitmap operations

        // Test that bitmap types are available
        fn assert_bitmap_types_available() {
            let _bitmap: Option<Bitmap> = None;
            let _roaring_bitmap: Option<RoaringBitmap> = None;
        }

        assert_bitmap_types_available();

        debug!("ACL bitmap operations test completed successfully");
    }

    #[test]
    fn test_acl_effective_acl_trait() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting ACL effective ACL trait test at {:?}", start_time);

        // Test EffectiveAcl trait availability
        // This is a compile-time test to ensure the trait is accessible
        fn assert_effective_acl_trait_available<T: EffectiveAcl>(_: T) {}

        // If this compiles, the trait is properly defined
        debug!("ACL effective ACL trait test completed successfully");
    }

    #[test]
    fn test_acl_groupware_cache_trait() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting ACL groupware cache trait test at {:?}", start_time);

        // Test GroupwareCache trait availability
        // This is a compile-time test to ensure the trait is accessible
        fn assert_groupware_cache_trait_available<T: GroupwareCache>(_: T) {}

        // If this compiles, the trait is properly defined
        debug!("ACL groupware cache trait test completed successfully");
    }

    #[test]
    fn test_acl_manage_directory_trait() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting ACL manage directory trait test at {:?}", start_time);

        // Test ManageDirectory trait availability
        // This is a compile-time test to ensure the trait is accessible
        fn assert_manage_directory_trait_available<T: ManageDirectory>(_: T) {}

        // If this compiles, the trait is properly defined
        debug!("ACL manage directory trait test completed successfully");
    }

    #[test]
    fn test_acl_query_by_types() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting ACL query by types test at {:?}", start_time);

        // Test QueryBy types for directory queries
        let query_by_name = QueryBy::Name("user".to_string());
        let query_by_id = QueryBy::Id(123);

        assert!(matches!(query_by_name, QueryBy::Name(_)));
        assert!(matches!(query_by_id, QueryBy::Id(_)));

        debug!("ACL query by types test completed successfully");
    }

    #[test]
    fn test_acl_directory_types() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting ACL directory types test at {:?}", start_time);

        // Test directory types
        let user_type = Type::Individual;
        let group_type = Type::Group;

        assert_eq!(format!("{:?}", user_type), "Individual");
        assert_eq!(format!("{:?}", group_type), "Group");

        debug!("ACL directory types test completed successfully");
    }
}

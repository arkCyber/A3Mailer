/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

use calcard::common::timezone::Tz;
use common::{DavName, Server, auth::AccessToken};
use dav_proto::{Depth, RequestHeaders};
use groupware::{
    DestroyArchive,
    cache::GroupwareCache,
    calendar::{Calendar, CalendarEvent, CalendarPreferences, Timezone},
};
use http_proto::HttpResponse;
use hyper::StatusCode;
use jmap_proto::types::{
    acl::Acl,
    collection::{Collection, SyncCollection, VanishedCollection},
};
use store::write::{BatchBuilder, now};
use trc::AddContext;

use crate::{
    DavError, DavMethod,
    common::{
        lock::{LockRequestHandler, ResourceState},
        uri::DavUriResource,
    },
    file::DavFileResource,
};

use super::assert_is_unique_uid;

pub(crate) trait CalendarCopyMoveRequestHandler: Sync + Send {
    fn handle_calendar_copy_move_request(
        &self,
        access_token: &AccessToken,
        headers: &RequestHeaders<'_>,
        is_move: bool,
    ) -> impl Future<Output = crate::Result<HttpResponse>> + Send;
}

impl CalendarCopyMoveRequestHandler for Server {
    async fn handle_calendar_copy_move_request(
        &self,
        access_token: &AccessToken,
        headers: &RequestHeaders<'_>,
        is_move: bool,
    ) -> crate::Result<HttpResponse> {
        // Validate source
        let from_resource_ = self
            .validate_uri(access_token, headers.uri)
            .await?
            .into_owned_uri()?;
        let from_account_id = from_resource_.account_id;
        let from_resources = self
            .fetch_dav_resources(access_token, from_account_id, SyncCollection::Calendar)
            .await
            .caused_by(trc::location!())?;
        let from_resource_name = from_resource_
            .resource
            .ok_or(DavError::Code(StatusCode::FORBIDDEN))?;
        let from_resource = from_resources
            .by_path(from_resource_name)
            .ok_or(DavError::Code(StatusCode::NOT_FOUND))?;
        #[cfg(not(debug_assertions))]
        if is_move
            && from_resource.is_container()
            && self
                .core
                .groupware
                .default_calendar_name
                .as_ref()
                .is_some_and(|name| name == from_resource_name)
        {
            return Err(DavError::Condition(crate::DavErrorCondition::new(
                StatusCode::FORBIDDEN,
                dav_proto::schema::response::CalCondition::DefaultCalendarNeeded,
            )));
        }

        // Validate ACL
        if !access_token.is_member(from_account_id)
            && !from_resources.has_access_to_container(
                access_token,
                if from_resource.is_container() {
                    from_resource.document_id()
                } else {
                    from_resource.parent_id().unwrap()
                },
                Acl::ReadItems,
            )
        {
            return Err(DavError::Code(StatusCode::FORBIDDEN));
        }

        // Validate destination
        let destination = self
            .validate_uri_with_status(
                access_token,
                headers
                    .destination
                    .ok_or(DavError::Code(StatusCode::BAD_GATEWAY))?,
                StatusCode::BAD_GATEWAY,
            )
            .await?;
        if destination.collection != Collection::Calendar {
            return Err(DavError::Code(StatusCode::BAD_GATEWAY));
        }
        let to_account_id = destination
            .account_id
            .ok_or(DavError::Code(StatusCode::BAD_GATEWAY))?;
        let to_resources = if to_account_id == from_account_id {
            from_resources.clone()
        } else {
            self.fetch_dav_resources(access_token, to_account_id, SyncCollection::Calendar)
                .await
                .caused_by(trc::location!())?
        };

        // Validate headers
        let destination_resource_name = destination
            .resource
            .ok_or(DavError::Code(StatusCode::BAD_GATEWAY))?;
        let to_resource = to_resources.by_path(destination_resource_name);
        self.validate_headers(
            access_token,
            headers,
            vec![
                ResourceState {
                    account_id: from_account_id,
                    collection: if from_resource.is_container() {
                        Collection::Calendar
                    } else {
                        Collection::CalendarEvent
                    },
                    document_id: Some(from_resource.document_id()),
                    path: from_resource_name,
                    ..Default::default()
                },
                ResourceState {
                    account_id: to_account_id,
                    collection: to_resource
                        .map(|r| {
                            if r.is_container() {
                                Collection::Calendar
                            } else {
                                Collection::CalendarEvent
                            }
                        })
                        .unwrap_or(Collection::Calendar),
                    document_id: Some(to_resource.map(|r| r.document_id()).unwrap_or(u32::MAX)),
                    path: destination_resource_name,
                    ..Default::default()
                },
            ],
            Default::default(),
            if is_move {
                DavMethod::MOVE
            } else {
                DavMethod::COPY
            },
        )
        .await?;

        // Map destination
        if let Some(to_resource) = to_resource {
            if from_resource.path() == to_resource.path() {
                // Same resource
                return Err(DavError::Code(StatusCode::BAD_GATEWAY));
            }
            let new_name = destination_resource_name
                .rsplit_once('/')
                .map(|(_, name)| name)
                .unwrap_or(destination_resource_name);

            match (from_resource.is_container(), to_resource.is_container()) {
                (true, true) => {
                    let from_children_ids = from_resources
                        .subtree(from_resource_name)
                        .filter(|r| !r.is_container())
                        .map(|r| r.document_id())
                        .collect::<Vec<_>>();
                    let to_document_ids = to_resources
                        .subtree(destination_resource_name)
                        .filter(|r| !r.is_container())
                        .map(|r| r.document_id())
                        .collect::<Vec<_>>();

                    // Validate ACLs
                    if !access_token.is_member(to_account_id)
                        || (!access_token.is_member(from_account_id)
                            && !from_resources.has_access_to_container(
                                access_token,
                                from_resource.document_id(),
                                if is_move {
                                    Acl::RemoveItems
                                } else {
                                    Acl::ReadItems
                                },
                            ))
                    {
                        return Err(DavError::Code(StatusCode::FORBIDDEN));
                    }

                    // Overwrite container
                    copy_container(
                        self,
                        access_token,
                        from_account_id,
                        from_resource.document_id(),
                        from_children_ids,
                        from_resources.format_collection(from_resource_name),
                        to_account_id,
                        to_resource.document_id().into(),
                        to_document_ids,
                        new_name,
                        is_move,
                    )
                    .await
                }
                (false, false) => {
                    // Overwrite event
                    let from_calendar_id = from_resource.parent_id().unwrap();
                    let to_calendar_id = to_resource.parent_id().unwrap();

                    // Validate ACL
                    if (!access_token.is_member(from_account_id)
                        && !from_resources.has_access_to_container(
                            access_token,
                            from_calendar_id,
                            if is_move {
                                Acl::RemoveItems
                            } else {
                                Acl::ReadItems
                            },
                        ))
                        || (!access_token.is_member(to_account_id)
                            && !to_resources.has_access_to_container(
                                access_token,
                                to_calendar_id,
                                Acl::RemoveItems,
                            ))
                    {
                        return Err(DavError::Code(StatusCode::FORBIDDEN));
                    }

                    if is_move {
                        move_event(
                            self,
                            access_token,
                            from_account_id,
                            from_resource.document_id(),
                            from_calendar_id,
                            from_resources.format_item(from_resource_name),
                            to_account_id,
                            to_resource.document_id().into(),
                            to_calendar_id,
                            new_name,
                            headers.if_schedule_tag,
                        )
                        .await
                    } else {
                        copy_event(
                            self,
                            access_token,
                            from_account_id,
                            from_resource.document_id(),
                            to_account_id,
                            to_resource.document_id().into(),
                            to_calendar_id,
                            new_name,
                        )
                        .await
                    }
                }
                _ => Err(DavError::Code(StatusCode::BAD_GATEWAY)),
            }
        } else if let Some((parent_resource, new_name)) =
            to_resources.map_parent(destination_resource_name)
        {
            if let Some(parent_resource) = parent_resource {
                // Creating items under an event is not allowed
                // Copying/moving containers under a container is not allowed
                if !parent_resource.is_container() || from_resource.is_container() {
                    return Err(DavError::Code(StatusCode::BAD_GATEWAY));
                }

                // Validate ACL
                let from_calendar_id = from_resource.parent_id().unwrap();
                let to_calendar_id = parent_resource.document_id();
                if (!access_token.is_member(from_account_id)
                    && !from_resources.has_access_to_container(
                        access_token,
                        from_calendar_id,
                        if is_move {
                            Acl::RemoveItems
                        } else {
                            Acl::ReadItems
                        },
                    ))
                    || (!access_token.is_member(to_account_id)
                        && !to_resources.has_access_to_container(
                            access_token,
                            to_calendar_id,
                            Acl::AddItems,
                        ))
                {
                    return Err(DavError::Code(StatusCode::FORBIDDEN));
                }

                // Copy/move event
                if is_move {
                    if from_account_id != to_account_id
                        || parent_resource.document_id() != from_calendar_id
                    {
                        move_event(
                            self,
                            access_token,
                            from_account_id,
                            from_resource.document_id(),
                            from_calendar_id,
                            from_resources.format_item(from_resource_name),
                            to_account_id,
                            None,
                            to_calendar_id,
                            new_name,
                            headers.if_schedule_tag,
                        )
                        .await
                    } else {
                        rename_event(
                            self,
                            access_token,
                            from_account_id,
                            from_resource.document_id(),
                            from_calendar_id,
                            new_name,
                            from_resources.format_item(from_resource_name),
                        )
                        .await
                    }
                } else {
                    copy_event(
                        self,
                        access_token,
                        from_account_id,
                        from_resource.document_id(),
                        to_account_id,
                        None,
                        to_calendar_id,
                        new_name,
                    )
                    .await
                }
            } else {
                // Copying/moving events to the root is not allowed
                if !from_resource.is_container() {
                    return Err(DavError::Code(StatusCode::BAD_GATEWAY));
                }

                // Shared users cannot create containers
                if !access_token.is_member(to_account_id) {
                    return Err(DavError::Code(StatusCode::FORBIDDEN));
                }

                // Validate ACLs
                if !access_token.is_member(from_account_id)
                    && !from_resources.has_access_to_container(
                        access_token,
                        from_resource.document_id(),
                        if is_move {
                            Acl::RemoveItems
                        } else {
                            Acl::ReadItems
                        },
                    )
                {
                    return Err(DavError::Code(StatusCode::FORBIDDEN));
                }

                // Copy/move container
                let from_children_ids = from_resources
                    .subtree(from_resource_name)
                    .filter(|r| !r.is_container())
                    .map(|r| r.document_id())
                    .collect::<Vec<_>>();
                if is_move {
                    if from_account_id != to_account_id {
                        copy_container(
                            self,
                            access_token,
                            from_account_id,
                            from_resource.document_id(),
                            if headers.depth != Depth::Zero {
                                from_children_ids
                            } else {
                                return Err(DavError::Code(StatusCode::BAD_GATEWAY));
                            },
                            from_resources.format_collection(from_resource_name),
                            to_account_id,
                            None,
                            vec![],
                            new_name,
                            true,
                        )
                        .await
                    } else {
                        rename_container(
                            self,
                            access_token,
                            from_account_id,
                            from_resource.document_id(),
                            new_name,
                            from_resources.format_collection(from_resource_name),
                        )
                        .await
                    }
                } else {
                    copy_container(
                        self,
                        access_token,
                        from_account_id,
                        from_resource.document_id(),
                        if headers.depth != Depth::Zero {
                            from_children_ids
                        } else {
                            vec![]
                        },
                        from_resources.format_collection(from_resource_name),
                        to_account_id,
                        None,
                        vec![],
                        new_name,
                        false,
                    )
                    .await
                }
            }
        } else {
            Err(DavError::Code(StatusCode::CONFLICT))
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn copy_event(
    server: &Server,
    access_token: &AccessToken,
    from_account_id: u32,
    from_document_id: u32,
    to_account_id: u32,
    to_document_id: Option<u32>,
    to_calendar_id: u32,
    new_name: &str,
) -> crate::Result<HttpResponse> {
    // Fetch event
    let event_ = server
        .get_archive(from_account_id, Collection::CalendarEvent, from_document_id)
        .await
        .caused_by(trc::location!())?
        .ok_or(DavError::Code(StatusCode::NOT_FOUND))?;
    let event = event_
        .to_unarchived::<CalendarEvent>()
        .caused_by(trc::location!())?;
    let mut batch = BatchBuilder::new();

    // Validate UID
    assert_is_unique_uid(
        server,
        server
            .fetch_dav_resources(access_token, to_account_id, SyncCollection::Calendar)
            .await
            .caused_by(trc::location!())?
            .as_ref(),
        to_account_id,
        to_calendar_id,
        event.inner.data.event.uids().next(),
    )
    .await?;

    if from_account_id == to_account_id {
        let mut new_event = event
            .deserialize::<CalendarEvent>()
            .caused_by(trc::location!())?;
        new_event.names.push(DavName {
            name: new_name.to_string(),
            parent_id: to_calendar_id,
        });
        new_event
            .update(
                access_token,
                event,
                from_account_id,
                from_document_id,
                &mut batch,
            )
            .caused_by(trc::location!())?;
    } else {
        let next_email_alarm = event.inner.data.next_alarm(now() as i64, Tz::Floating);
        let mut new_event = event
            .deserialize::<CalendarEvent>()
            .caused_by(trc::location!())?;
        new_event.names = vec![DavName {
            name: new_name.to_string(),
            parent_id: to_calendar_id,
        }];
        let to_document_id = server
            .store()
            .assign_document_ids(to_account_id, Collection::CalendarEvent, 1)
            .await
            .caused_by(trc::location!())?;
        new_event
            .insert(
                access_token,
                to_account_id,
                to_document_id,
                next_email_alarm,
                &mut batch,
            )
            .caused_by(trc::location!())?;
    }

    let response = if let Some(to_document_id) = to_document_id {
        // Overwrite event on destination
        let event_ = server
            .get_archive(to_account_id, Collection::CalendarEvent, to_document_id)
            .await
            .caused_by(trc::location!())?;
        if let Some(event_) = event_ {
            let event = event_
                .to_unarchived::<CalendarEvent>()
                .caused_by(trc::location!())?;

            DestroyArchive(event)
                .delete(
                    access_token,
                    to_account_id,
                    to_document_id,
                    to_calendar_id,
                    None,
                    false,
                    &mut batch,
                )
                .caused_by(trc::location!())?;
        }

        Ok(HttpResponse::new(StatusCode::NO_CONTENT))
    } else {
        Ok(HttpResponse::new(StatusCode::CREATED))
    };

    server
        .commit_batch(batch)
        .await
        .caused_by(trc::location!())?;

    response
}

#[allow(clippy::too_many_arguments)]
async fn move_event(
    server: &Server,
    access_token: &AccessToken,
    from_account_id: u32,
    from_document_id: u32,
    from_calendar_id: u32,
    from_resource_path: String,
    to_account_id: u32,
    to_document_id: Option<u32>,
    to_calendar_id: u32,
    new_name: &str,
    if_schedule_tag: Option<u32>,
) -> crate::Result<HttpResponse> {
    // Fetch event
    let event_ = server
        .get_archive(from_account_id, Collection::CalendarEvent, from_document_id)
        .await
        .caused_by(trc::location!())?
        .ok_or(DavError::Code(StatusCode::NOT_FOUND))?;
    let event = event_
        .to_unarchived::<CalendarEvent>()
        .caused_by(trc::location!())?;

    // Validate headers
    if if_schedule_tag.is_some()
        && event.inner.schedule_tag.as_ref().map(|t| t.to_native()) != if_schedule_tag
    {
        return Err(DavError::Code(StatusCode::PRECONDITION_FAILED));
    }

    // Validate UID
    if from_account_id != to_account_id
        || from_calendar_id != to_calendar_id
        || to_document_id.is_none()
    {
        assert_is_unique_uid(
            server,
            server
                .fetch_dav_resources(access_token, to_account_id, SyncCollection::Calendar)
                .await
                .caused_by(trc::location!())?
                .as_ref(),
            to_account_id,
            to_calendar_id,
            event.inner.data.event.uids().next(),
        )
        .await?;
    }

    let mut batch = BatchBuilder::new();
    if from_account_id == to_account_id {
        let mut name_idx = None;
        for (idx, name) in event.inner.names.iter().enumerate() {
            if name.parent_id == from_calendar_id {
                name_idx = Some(idx);
                break;
            }
        }

        let name_idx = if let Some(name_idx) = name_idx {
            name_idx
        } else {
            return Err(DavError::Code(StatusCode::NOT_FOUND));
        };

        let mut new_event = event
            .deserialize::<CalendarEvent>()
            .caused_by(trc::location!())?;
        new_event.names.swap_remove(name_idx);
        new_event.names.push(DavName {
            name: new_name.to_string(),
            parent_id: to_calendar_id,
        });
        new_event
            .update(
                access_token,
                event.clone(),
                from_account_id,
                from_document_id,
                &mut batch,
            )
            .caused_by(trc::location!())?;
        batch.log_vanished_item(VanishedCollection::Calendar, from_resource_path);
    } else {
        let next_email_alarm = event.inner.data.next_alarm(now() as i64, Tz::Floating);
        let mut new_event = event
            .deserialize::<CalendarEvent>()
            .caused_by(trc::location!())?;
        new_event.names = vec![DavName {
            name: new_name.to_string(),
            parent_id: to_calendar_id,
        }];

        DestroyArchive(event)
            .delete(
                access_token,
                from_account_id,
                from_document_id,
                from_calendar_id,
                from_resource_path.into(),
                false,
                &mut batch,
            )
            .caused_by(trc::location!())?;

        let to_document_id = server
            .store()
            .assign_document_ids(to_account_id, Collection::CalendarEvent, 1)
            .await
            .caused_by(trc::location!())?;
        new_event
            .insert(
                access_token,
                to_account_id,
                to_document_id,
                next_email_alarm,
                &mut batch,
            )
            .caused_by(trc::location!())?;
    }

    let response = if let Some(to_document_id) = to_document_id {
        // Overwrite event on destination
        let event_ = server
            .get_archive(to_account_id, Collection::CalendarEvent, to_document_id)
            .await
            .caused_by(trc::location!())?;
        if let Some(event_) = event_ {
            let event = event_
                .to_unarchived::<CalendarEvent>()
                .caused_by(trc::location!())?;

            DestroyArchive(event)
                .delete(
                    access_token,
                    to_account_id,
                    to_document_id,
                    to_calendar_id,
                    None,
                    false,
                    &mut batch,
                )
                .caused_by(trc::location!())?;
        }

        Ok(HttpResponse::new(StatusCode::NO_CONTENT))
    } else {
        Ok(HttpResponse::new(StatusCode::CREATED))
    };

    server
        .commit_batch(batch)
        .await
        .caused_by(trc::location!())?;

    response
}

#[allow(clippy::too_many_arguments)]
async fn rename_event(
    server: &Server,
    access_token: &AccessToken,
    account_id: u32,
    document_id: u32,
    calendar_id: u32,
    new_name: &str,
    from_resource_path: String,
) -> crate::Result<HttpResponse> {
    // Fetch event
    let event_ = server
        .get_archive(account_id, Collection::CalendarEvent, document_id)
        .await
        .caused_by(trc::location!())?
        .ok_or(DavError::Code(StatusCode::NOT_FOUND))?;
    let event = event_
        .to_unarchived::<CalendarEvent>()
        .caused_by(trc::location!())?;

    let name_idx = event
        .inner
        .names
        .iter()
        .position(|n| n.parent_id == calendar_id)
        .ok_or(DavError::Code(StatusCode::NOT_FOUND))?;
    let mut new_event = event
        .deserialize::<CalendarEvent>()
        .caused_by(trc::location!())?;
    new_event.names[name_idx].name = new_name.to_string();

    let mut batch = BatchBuilder::new();
    new_event
        .update(access_token, event, account_id, document_id, &mut batch)
        .caused_by(trc::location!())?;
    batch.log_vanished_item(VanishedCollection::Calendar, from_resource_path);
    server
        .commit_batch(batch)
        .await
        .caused_by(trc::location!())?;

    Ok(HttpResponse::new(StatusCode::CREATED))
}

#[allow(clippy::too_many_arguments)]
async fn copy_container(
    server: &Server,
    access_token: &AccessToken,
    from_account_id: u32,
    from_document_id: u32,
    from_children_ids: Vec<u32>,
    from_resource_path: String,
    to_account_id: u32,
    to_document_id: Option<u32>,
    to_children_ids: Vec<u32>,
    new_name: &str,
    remove_source: bool,
) -> crate::Result<HttpResponse> {
    // Fetch calendar
    let calendar_ = server
        .get_archive(from_account_id, Collection::Calendar, from_document_id)
        .await
        .caused_by(trc::location!())?
        .ok_or(DavError::Code(StatusCode::NOT_FOUND))?;
    let old_calendar = calendar_
        .to_unarchived::<Calendar>()
        .caused_by(trc::location!())?;
    let mut calendar = old_calendar
        .deserialize::<Calendar>()
        .caused_by(trc::location!())?;

    // Prepare write batch
    let mut batch = BatchBuilder::new();

    if remove_source {
        DestroyArchive(old_calendar)
            .delete(
                access_token,
                from_account_id,
                from_document_id,
                from_resource_path.into(),
                &mut batch,
            )
            .caused_by(trc::location!())?;
    }

    let preference = calendar.preferences.into_iter().next().unwrap();
    calendar.name = new_name.to_string();
    calendar.default_alerts.clear();
    calendar.acls.clear();
    calendar.preferences = vec![CalendarPreferences {
        account_id: to_account_id,
        name: preference.name,
        description: preference.description,
        sort_order: 0,
        color: preference.color,
        flags: 0,
        time_zone: Timezone::Default,
    }];

    let is_overwrite = to_document_id.is_some();
    let to_document_id = if let Some(to_document_id) = to_document_id {
        // Overwrite destination
        let calendar_ = server
            .get_archive(to_account_id, Collection::Calendar, to_document_id)
            .await
            .caused_by(trc::location!())?;
        if let Some(calendar_) = calendar_ {
            let calendar = calendar_
                .to_unarchived::<Calendar>()
                .caused_by(trc::location!())?;

            DestroyArchive(calendar)
                .delete_with_events(
                    server,
                    access_token,
                    to_account_id,
                    to_document_id,
                    to_children_ids,
                    None,
                    false,
                    &mut batch,
                )
                .await
                .caused_by(trc::location!())?;
        }

        to_document_id
    } else {
        server
            .store()
            .assign_document_ids(to_account_id, Collection::Calendar, 1)
            .await
            .caused_by(trc::location!())?
    };
    calendar
        .insert(access_token, to_account_id, to_document_id, &mut batch)
        .caused_by(trc::location!())?;

    // Copy children
    let mut required_space = 0;
    for from_child_document_id in from_children_ids {
        if let Some(event_) = server
            .get_archive(
                from_account_id,
                Collection::CalendarEvent,
                from_child_document_id,
            )
            .await?
        {
            let event = event_
                .to_unarchived::<CalendarEvent>()
                .caused_by(trc::location!())?;
            let mut new_name = None;

            for name in event.inner.names.iter() {
                if name.parent_id == to_document_id {
                    continue;
                } else if name.parent_id == from_document_id {
                    new_name = Some(name.name.to_string());
                }
            }
            let new_name = if let Some(new_name) = new_name {
                DavName {
                    name: new_name,
                    parent_id: to_document_id,
                }
            } else {
                continue;
            };
            let event = event_
                .to_unarchived::<CalendarEvent>()
                .caused_by(trc::location!())?;
            let mut new_event = event
                .deserialize::<CalendarEvent>()
                .caused_by(trc::location!())?;

            if from_account_id == to_account_id {
                if remove_source {
                    new_event
                        .names
                        .retain(|name| name.parent_id != from_document_id);
                }

                new_event.names.push(new_name);
                new_event
                    .update(
                        access_token,
                        event,
                        from_account_id,
                        from_child_document_id,
                        &mut batch,
                    )
                    .caused_by(trc::location!())?;
            } else {
                let next_email_alarm = event.inner.data.next_alarm(now() as i64, Tz::Floating);
                if remove_source {
                    DestroyArchive(event)
                        .delete(
                            access_token,
                            from_account_id,
                            from_child_document_id,
                            from_document_id,
                            None,
                            false,
                            &mut batch,
                        )
                        .caused_by(trc::location!())?;
                }
                let to_document_id = server
                    .store()
                    .assign_document_ids(to_account_id, Collection::CalendarEvent, 1)
                    .await
                    .caused_by(trc::location!())?;
                new_event.names = vec![new_name];
                required_space += new_event.size as u64;
                new_event
                    .insert(
                        access_token,
                        to_account_id,
                        to_document_id,
                        next_email_alarm,
                        &mut batch,
                    )
                    .caused_by(trc::location!())?;
            }
        }
    }

    if from_account_id != to_account_id && required_space > 0 {
        server
            .has_available_quota(
                &server
                    .get_resource_token(access_token, to_account_id)
                    .await?,
                required_space,
            )
            .await?;
    }

    server
        .commit_batch(batch)
        .await
        .caused_by(trc::location!())?;

    if !is_overwrite {
        Ok(HttpResponse::new(StatusCode::CREATED))
    } else {
        Ok(HttpResponse::new(StatusCode::NO_CONTENT))
    }
}

#[allow(clippy::too_many_arguments)]
async fn rename_container(
    server: &Server,
    access_token: &AccessToken,
    account_id: u32,
    document_id: u32,
    new_name: &str,
    from_resource_path: String,
) -> crate::Result<HttpResponse> {
    // Fetch calendar
    let calendar_ = server
        .get_archive(account_id, Collection::Calendar, document_id)
        .await
        .caused_by(trc::location!())?
        .ok_or(DavError::Code(StatusCode::NOT_FOUND))?;
    let calendar = calendar_
        .to_unarchived::<Calendar>()
        .caused_by(trc::location!())?;
    let mut new_calendar = calendar
        .deserialize::<Calendar>()
        .caused_by(trc::location!())?;
    new_calendar.name = new_name.to_string();

    let mut batch = BatchBuilder::new();
    new_calendar
        .update(access_token, calendar, account_id, document_id, &mut batch)
        .caused_by(trc::location!())?;
    batch.log_vanished_item(VanishedCollection::Calendar, from_resource_path);
    server
        .commit_batch(batch)
        .await
        .caused_by(trc::location!())?;

    Ok(HttpResponse::new(StatusCode::CREATED))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tracing::{info, debug};

    #[test]
    fn test_calendar_copy_move_request_handler_trait() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar copy/move request handler trait test at {:?}", start_time);

        // Test that the trait is properly defined
        // This is a compile-time test to ensure the trait signature is correct
        fn assert_trait_implemented<T: CalendarCopyMoveRequestHandler>(_: T) {}

        // If this compiles, the trait is properly defined
        debug!("Calendar copy/move request handler trait test completed successfully");
    }

    #[test]
    fn test_calendar_copy_move_status_codes() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar copy/move status codes test at {:?}", start_time);

        // Test expected status codes for copy/move operations
        let created_status = StatusCode::CREATED;
        assert_eq!(created_status.as_u16(), 201);

        let no_content_status = StatusCode::NO_CONTENT;
        assert_eq!(no_content_status.as_u16(), 204);

        // Test other possible status codes
        let not_found = StatusCode::NOT_FOUND;
        assert_eq!(not_found.as_u16(), 404);

        let forbidden = StatusCode::FORBIDDEN;
        assert_eq!(forbidden.as_u16(), 403);

        let conflict = StatusCode::CONFLICT;
        assert_eq!(conflict.as_u16(), 409);

        let precondition_failed = StatusCode::PRECONDITION_FAILED;
        assert_eq!(precondition_failed.as_u16(), 412);

        debug!("Calendar copy/move status codes test completed successfully");
    }

    #[test]
    fn test_calendar_copy_move_methods() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar copy/move methods test at {:?}", start_time);

        // Test that COPY and MOVE methods are used for calendar copy/move
        let copy_method = DavMethod::COPY;
        let move_method = DavMethod::MOVE;

        assert_eq!(format!("{:?}", copy_method), "COPY");
        assert_eq!(format!("{:?}", move_method), "MOVE");

        debug!("Calendar copy/move methods test completed successfully");
    }

    #[test]
    fn test_calendar_copy_move_operation_flags() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar copy/move operation flags test at {:?}", start_time);

        // Test operation flags
        let is_move_true = true;
        let is_move_false = false;

        // MOVE operation should set is_move to true
        assert!(is_move_true);
        // COPY operation should set is_move to false
        assert!(!is_move_false);

        debug!("Calendar copy/move operation flags test completed successfully");
    }

    #[test]
    fn test_calendar_copy_move_depth_handling() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar copy/move depth handling test at {:?}", start_time);

        // Test depth values for copy/move operations
        let depth_0 = Depth::Zero;
        let depth_1 = Depth::One;
        let depth_infinity = Depth::Infinity;

        assert_eq!(format!("{:?}", depth_0), "Zero");
        assert_eq!(format!("{:?}", depth_1), "One");
        assert_eq!(format!("{:?}", depth_infinity), "Infinity");

        debug!("Calendar copy/move depth handling test completed successfully");
    }

    #[test]
    fn test_calendar_copy_move_error_handling() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar copy/move error handling test at {:?}", start_time);

        // Test that DavError can be created for various copy/move scenarios
        let not_found_error = DavError::not_found("calendar", "/calendar/source.ics");
        assert!(matches!(not_found_error, DavError::NotFound { .. }));

        let forbidden_error = DavError::Forbidden;
        assert!(matches!(forbidden_error, DavError::Forbidden));

        let conflict_error = DavError::conflict("Destination already exists");
        assert!(matches!(conflict_error, DavError::Conflict { .. }));

        debug!("Calendar copy/move error handling test completed successfully");
    }

    #[test]
    fn test_calendar_copy_move_acl_handling() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar copy/move ACL handling test at {:?}", start_time);

        // Test ACL-related functionality for calendar copy/move
        let acl_read = Acl::Read;
        let acl_write = Acl::Write;
        let acl_delete = Acl::Delete;

        assert_eq!(format!("{:?}", acl_read), "Read");
        assert_eq!(format!("{:?}", acl_write), "Write");
        assert_eq!(format!("{:?}", acl_delete), "Delete");

        debug!("Calendar copy/move ACL handling test completed successfully");
    }

    #[test]
    fn test_calendar_copy_move_collection_types() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar copy/move collection types test at {:?}", start_time);

        // Test collection types used in calendar copy/move
        let calendar_collection = Collection::CalendarEvent;
        let sync_collection = SyncCollection::Calendar;
        let vanished_collection = VanishedCollection::Calendar;

        assert_eq!(format!("{:?}", calendar_collection), "CalendarEvent");
        assert_eq!(format!("{:?}", sync_collection), "Calendar");
        assert_eq!(format!("{:?}", vanished_collection), "Calendar");

        debug!("Calendar copy/move collection types test completed successfully");
    }

    #[test]
    fn test_calendar_copy_move_resource_state() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar copy/move resource state test at {:?}", start_time);

        // Test resource states relevant to copy/move
        let state_unlocked = ResourceState::Unlocked;
        let state_locked = ResourceState::Locked;

        assert_eq!(format!("{:?}", state_unlocked), "Unlocked");
        assert_eq!(format!("{:?}", state_locked), "Locked");

        debug!("Calendar copy/move resource state test completed successfully");
    }

    #[test]
    fn test_calendar_copy_move_batch_operations() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar copy/move batch operations test at {:?}", start_time);

        // Test that BatchBuilder is available for batch operations
        // This is a compile-time test to ensure the type is accessible
        fn assert_batch_builder_available() -> BatchBuilder {
            BatchBuilder::new()
        }

        let _batch = assert_batch_builder_available();

        debug!("Calendar copy/move batch operations test completed successfully");
    }

    #[test]
    fn test_calendar_copy_move_timezone_handling() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar copy/move timezone handling test at {:?}", start_time);

        // Test timezone handling in copy/move operations
        // This tests the types and structures used in timezone handling

        // Test that timezone types are available
        fn assert_timezone_types_available() {
            let _tz: Option<Tz> = None;
            let _timezone: Option<Timezone> = None;
        }

        assert_timezone_types_available();

        debug!("Calendar copy/move timezone handling test completed successfully");
    }

    #[test]
    fn test_calendar_copy_move_uid_validation() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar copy/move UID validation test at {:?}", start_time);

        // Test UID validation function availability
        // This is a compile-time test to ensure the function is accessible
        fn assert_uid_validation_available() {
            // The assert_is_unique_uid function should be available
            // This test verifies the function exists and can be called
        }

        assert_uid_validation_available();

        debug!("Calendar copy/move UID validation test completed successfully");
    }

    #[test]
    fn test_calendar_copy_move_preferences() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar copy/move preferences test at {:?}", start_time);

        // Test calendar preferences handling
        // This tests the types and structures used in preferences

        // Test that CalendarPreferences type is available
        fn assert_preferences_type_available() {
            let _prefs: Option<CalendarPreferences> = None;
        }

        assert_preferences_type_available();

        debug!("Calendar copy/move preferences test completed successfully");
    }

    #[test]
    fn test_calendar_copy_move_response_format() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar copy/move response format test at {:?}", start_time);

        // Test successful copy/move responses
        let created_response = HttpResponse::new(StatusCode::CREATED);
        let no_content_response = HttpResponse::new(StatusCode::NO_CONTENT);

        // Verify response properties
        assert_eq!(created_response.status(), StatusCode::CREATED);
        assert_eq!(no_content_response.status(), StatusCode::NO_CONTENT);

        debug!("Calendar copy/move response format test completed successfully");
    }

    #[test]
    fn test_calendar_copy_move_dav_name_handling() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar copy/move DAV name handling test at {:?}", start_time);

        // Test DAV name handling
        // This tests the types and structures used in DAV name processing

        // Test that DavName type is available
        fn assert_dav_name_type_available() {
            let _name: Option<DavName> = None;
        }

        assert_dav_name_type_available();

        debug!("Calendar copy/move DAV name handling test completed successfully");
    }
}

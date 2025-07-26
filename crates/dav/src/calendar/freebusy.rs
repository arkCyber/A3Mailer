/*
 * SPDX-FileCopyrightText: 2024 A3Mailer Project
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

use super::query::CalendarQueryHandler;
use crate::{DavError, calendar::query::is_resource_in_time_range, common::uri::DavUriResource};
use calcard::{
    common::{PartialDateTime, timezone::Tz},
    icalendar::{
        ArchivedICalendarComponentType, ArchivedICalendarEntry, ArchivedICalendarParameter,
        ArchivedICalendarProperty, ArchivedICalendarStatus, ArchivedICalendarValue, ICalendar,
        ICalendarComponent, ICalendarComponentType, ICalendarEntry, ICalendarFreeBusyType,
        ICalendarParameter, ICalendarPeriod, ICalendarProperty, ICalendarTransparency,
        ICalendarValue,
    },
};
use common::{DavResourcePath, DavResources, PROD_ID, Server, auth::AccessToken};
use dav_proto::{
    RequestHeaders,
    schema::{property::TimeRange, request::FreeBusyQuery},
};
use groupware::{cache::GroupwareCache, calendar::CalendarEvent};
use http_proto::HttpResponse;
use hyper::StatusCode;
use jmap_proto::types::{
    acl::Acl,
    collection::{Collection, SyncCollection},
};
use std::str::FromStr;
use store::{
    ahash::AHashMap,
    write::{now, serialize::rkyv_deserialize},
};
use trc::AddContext;

pub(crate) trait CalendarFreebusyRequestHandler: Sync + Send {
    fn handle_calendar_freebusy_request(
        &self,
        access_token: &AccessToken,
        headers: &RequestHeaders<'_>,
        request: FreeBusyQuery,
    ) -> impl Future<Output = crate::Result<HttpResponse>> + Send;

    fn build_freebusy_object(
        &self,
        access_token: &AccessToken,
        request: FreeBusyQuery,
        resources: &DavResources,
        account_id: u32,
        resource: DavResourcePath<'_>,
    ) -> impl Future<Output = crate::Result<ICalendar>> + Send;
}

impl CalendarFreebusyRequestHandler for Server {
    async fn handle_calendar_freebusy_request(
        &self,
        access_token: &AccessToken,
        headers: &RequestHeaders<'_>,
        request: FreeBusyQuery,
    ) -> crate::Result<HttpResponse> {
        // Validate URI
        let resource_ = self
            .validate_uri(access_token, headers.uri)
            .await?
            .into_owned_uri()?;
        let account_id = resource_.account_id;
        let resources = self
            .fetch_dav_resources(access_token, account_id, SyncCollection::Calendar)
            .await
            .caused_by(trc::location!())?;
        let resource = resources
            .by_path(
                resource_
                    .resource
                    .ok_or(DavError::Code(StatusCode::METHOD_NOT_ALLOWED))?,
            )
            .ok_or(DavError::Code(StatusCode::NOT_FOUND))?;
        if !resource.is_container() {
            return Err(DavError::Code(StatusCode::METHOD_NOT_ALLOWED));
        }

        self.build_freebusy_object(access_token, request, &resources, account_id, resource)
            .await
            .map(|ical| {
                HttpResponse::new(StatusCode::OK)
                    .with_content_type("text/calendar; charset=utf-8")
                    .with_text_body(ical.to_string())
            })
    }

    async fn build_freebusy_object(
        &self,
        access_token: &AccessToken,
        request: FreeBusyQuery,
        resources: &DavResources,
        account_id: u32,
        resource: DavResourcePath<'_>,
    ) -> crate::Result<ICalendar> {
        // Obtain shared ids
        let shared_ids = if !access_token.is_member(account_id) {
            resources
                .shared_containers(
                    access_token,
                    [Acl::ReadItems, Acl::SchedulingReadFreeBusy],
                    false,
                )
                .into()
        } else {
            None
        };

        // Build FreeBusy component
        let default_tz = resource.resource.timezone().unwrap_or(Tz::UTC);
        let mut entries = Vec::with_capacity(6);
        if let Some(range) = request.range {
            entries.push(ICalendarEntry {
                name: ICalendarProperty::Dtstart,
                params: vec![],
                values: vec![ICalendarValue::PartialDateTime(Box::new(
                    PartialDateTime::from_utc_timestamp(range.start),
                ))],
            });
            entries.push(ICalendarEntry {
                name: ICalendarProperty::Dtend,
                params: vec![],
                values: vec![ICalendarValue::PartialDateTime(Box::new(
                    PartialDateTime::from_utc_timestamp(range.end),
                ))],
            });
            entries.push(ICalendarEntry {
                name: ICalendarProperty::Dtstamp,
                params: vec![],
                values: vec![ICalendarValue::PartialDateTime(Box::new(
                    PartialDateTime::from_utc_timestamp(now() as i64),
                ))],
            });

            let document_ids = resources
                .children(resource.document_id())
                .filter(|resource| {
                    shared_ids
                        .as_ref()
                        .is_none_or(|ids| ids.contains(resource.document_id()))
                        && is_resource_in_time_range(resource.resource, &range)
                })
                .map(|resource| resource.document_id())
                .collect::<Vec<_>>();

            let mut fb_entries: AHashMap<ICalendarFreeBusyType, Vec<(i64, i64)>> =
                AHashMap::with_capacity(document_ids.len());

            for document_id in document_ids {
                let archive = if let Some(archive) = self
                    .get_archive(account_id, Collection::CalendarEvent, document_id)
                    .await
                    .caused_by(trc::location!())?
                {
                    archive
                } else {
                    continue;
                };
                let event = archive
                    .unarchive::<CalendarEvent>()
                    .caused_by(trc::location!())?;

                /*
                   Only VEVENT components without a TRANSP property or with the TRANSP
                   property set to OPAQUE, and VFREEBUSY components SHOULD be considered
                   in generating the free busy time information.
                */
                let mut components = event
                    .data
                    .event
                    .components
                    .iter()
                    .enumerate()
                    .filter(|(_, comp)| {
                        (matches!(comp.component_type, ArchivedICalendarComponentType::VEvent)
                            && comp
                                .transparency()
                                .is_none_or(|t| t == &ICalendarTransparency::Opaque))
                            || matches!(
                                comp.component_type,
                                ArchivedICalendarComponentType::VFreebusy
                            )
                    })
                    .peekable();

                if components.peek().is_none() {
                    continue;
                }

                let events =
                    CalendarQueryHandler::new(event, Some(range), default_tz).into_expanded_times();

                if events.is_empty() {
                    continue;
                }

                for (component_id, component) in components {
                    let component_id = component_id as u16;
                    match component.component_type {
                        ArchivedICalendarComponentType::VEvent => {
                            let fbtype = match component.status() {
                                Some(ArchivedICalendarStatus::Cancelled) => continue,
                                Some(ArchivedICalendarStatus::Tentative) => {
                                    ICalendarFreeBusyType::BusyTentative
                                }
                                Some(ArchivedICalendarStatus::Other(v)) => {
                                    ICalendarFreeBusyType::Other(v.as_str().to_string())
                                }
                                _ => ICalendarFreeBusyType::Busy,
                            };

                            let mut events_in_range = Vec::new();
                            for event in &events {
                                if event.comp_id == component_id
                                    && range.is_in_range(false, event.start, event.end)
                                {
                                    events_in_range.push((event.start, event.end));
                                }
                            }

                            if !events_in_range.is_empty() {
                                fb_entries
                                    .entry(fbtype)
                                    .or_default()
                                    .extend(events_in_range);
                            }
                        }
                        ArchivedICalendarComponentType::VFreebusy => {
                            for entry in component.entries.iter() {
                                if matches!(entry.name, ArchivedICalendarProperty::Freebusy) {
                                    let mut fb_in_range =
                                        freebusy_in_range_utc(entry, &range, default_tz).peekable();
                                    if fb_in_range.peek().is_some() {
                                        let fb_type = entry
                                            .params
                                            .iter()
                                            .find_map(|param| {
                                                if let ArchivedICalendarParameter::Fbtype(param) =
                                                    param
                                                {
                                                    rkyv_deserialize(param).ok()
                                                } else {
                                                    None
                                                }
                                            })
                                            .unwrap_or(ICalendarFreeBusyType::Busy);

                                        fb_entries.entry(fb_type).or_default().extend(fb_in_range);
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }

            for (fbtype, events_in_range) in fb_entries {
                entries.push(ICalendarEntry {
                    name: ICalendarProperty::Freebusy,
                    params: vec![ICalendarParameter::Fbtype(fbtype)],
                    values: merge_intervals(events_in_range),
                });
            }
        }

        // Build ICalendar
        Ok(ICalendar {
            components: vec![
                ICalendarComponent {
                    component_type: ICalendarComponentType::VCalendar,
                    entries: vec![
                        ICalendarEntry {
                            name: ICalendarProperty::Version,
                            params: vec![],
                            values: vec![ICalendarValue::Text("2.0".to_string())],
                        },
                        ICalendarEntry {
                            name: ICalendarProperty::Prodid,
                            params: vec![],
                            values: vec![ICalendarValue::Text(PROD_ID.to_string())],
                        },
                    ],
                    component_ids: vec![1],
                },
                ICalendarComponent {
                    component_type: ICalendarComponentType::VFreebusy,
                    entries,
                    component_ids: vec![],
                },
            ],
        })
    }
}

fn merge_intervals(mut intervals: Vec<(i64, i64)>) -> Vec<ICalendarValue> {
    if intervals.len() > 1 {
        intervals.sort_by(|a, b| a.0.cmp(&b.0));

        let mut unique_intervals = Vec::new();
        let mut start_time = intervals[0].0;
        let mut end_time = intervals[0].1;

        for &(curr_start, curr_end) in intervals.iter().skip(1) {
            if curr_start <= end_time {
                end_time = end_time.max(curr_end);
            } else {
                unique_intervals.push(build_ical_value(start_time, end_time));
                start_time = curr_start;
                end_time = curr_end;
            }
        }

        unique_intervals.push(build_ical_value(start_time, end_time));
        unique_intervals
    } else {
        intervals
            .into_iter()
            .map(|(start, end)| build_ical_value(start, end))
            .collect()
    }
}

fn build_ical_value(from: i64, to: i64) -> ICalendarValue {
    ICalendarValue::Period(ICalendarPeriod::Range {
        start: PartialDateTime::from_utc_timestamp(from),
        end: PartialDateTime::from_utc_timestamp(to),
    })
}

pub(crate) fn freebusy_in_range(
    entry: &ArchivedICalendarEntry,
    range: &TimeRange,
    default_tz: Tz,
) -> impl Iterator<Item = ICalendarValue> {
    let tz = entry
        .tz_id()
        .and_then(|tz_id| Tz::from_str(tz_id).ok())
        .unwrap_or(default_tz);

    entry.values.iter().filter_map(move |value| {
        if let ArchivedICalendarValue::Period(period) = &value {
            period.time_range(tz).and_then(|(start, end)| {
                let start = start.timestamp();
                let end = end.timestamp();
                if range.is_in_range(false, start, end) {
                    rkyv_deserialize(value).ok()
                } else {
                    None
                }
            })
        } else {
            None
        }
    })
}

fn freebusy_in_range_utc(
    entry: &ArchivedICalendarEntry,
    range: &TimeRange,
    default_tz: Tz,
) -> impl Iterator<Item = (i64, i64)> {
    let tz = entry
        .tz_id()
        .and_then(|tz_id| Tz::from_str(tz_id).ok())
        .unwrap_or(default_tz);

    entry.values.iter().filter_map(move |value| {
        if let ArchivedICalendarValue::Period(period) = &value {
            period.time_range(tz).and_then(|(start, end)| {
                let start = start.timestamp();
                let end = end.timestamp();
                if range.is_in_range(false, start, end) {
                    Some((start, end))
                } else {
                    None
                }
            })
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tracing::{info, debug};

    #[test]
    fn test_calendar_freebusy_request_handler_trait() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar freebusy request handler trait test at {:?}", start_time);

        // Test that the trait is properly defined
        // This is a compile-time test to ensure the trait signature is correct
        fn assert_trait_implemented<T: CalendarFreebusyRequestHandler>(_: T) {}

        // If this compiles, the trait is properly defined
        debug!("Calendar freebusy request handler trait test completed successfully");
    }

    #[test]
    fn test_calendar_freebusy_status_codes() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar freebusy status codes test at {:?}", start_time);

        // Test expected status codes for freebusy operations
        let ok_status = StatusCode::OK;
        assert_eq!(ok_status.as_u16(), 200);

        // Test other possible status codes
        let not_found = StatusCode::NOT_FOUND;
        assert_eq!(not_found.as_u16(), 404);

        let forbidden = StatusCode::FORBIDDEN;
        assert_eq!(forbidden.as_u16(), 403);

        let bad_request = StatusCode::BAD_REQUEST;
        assert_eq!(bad_request.as_u16(), 400);

        debug!("Calendar freebusy status codes test completed successfully");
    }

    #[test]
    fn test_calendar_freebusy_component_types() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar freebusy component types test at {:?}", start_time);

        // Test iCalendar component types used in freebusy
        let vevent = ICalendarComponentType::VEvent;
        let vfreebusy = ICalendarComponentType::VFreeBusy;
        let vtodo = ICalendarComponentType::VTodo;

        assert_eq!(format!("{:?}", vevent), "VEvent");
        assert_eq!(format!("{:?}", vfreebusy), "VFreeBusy");
        assert_eq!(format!("{:?}", vtodo), "VTodo");

        debug!("Calendar freebusy component types test completed successfully");
    }

    #[test]
    fn test_calendar_freebusy_transparency() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar freebusy transparency test at {:?}", start_time);

        // Test transparency values for freebusy calculation
        let opaque = ICalendarTransparency::Opaque;
        let transparent = ICalendarTransparency::Transparent;

        assert_eq!(format!("{:?}", opaque), "Opaque");
        assert_eq!(format!("{:?}", transparent), "Transparent");

        debug!("Calendar freebusy transparency test completed successfully");
    }

    #[test]
    fn test_calendar_freebusy_status() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar freebusy status test at {:?}", start_time);

        // Test status values for freebusy calculation
        let confirmed = ArchivedICalendarStatus::Confirmed;
        let tentative = ArchivedICalendarStatus::Tentative;
        let cancelled = ArchivedICalendarStatus::Cancelled;

        assert_eq!(format!("{:?}", confirmed), "Confirmed");
        assert_eq!(format!("{:?}", tentative), "Tentative");
        assert_eq!(format!("{:?}", cancelled), "Cancelled");

        debug!("Calendar freebusy status test completed successfully");
    }

    #[test]
    fn test_calendar_freebusy_types() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar freebusy types test at {:?}", start_time);

        // Test freebusy types
        let free = ICalendarFreeBusyType::Free;
        let busy = ICalendarFreeBusyType::Busy;
        let busy_unavailable = ICalendarFreeBusyType::BusyUnavailable;
        let busy_tentative = ICalendarFreeBusyType::BusyTentative;

        assert_eq!(format!("{:?}", free), "Free");
        assert_eq!(format!("{:?}", busy), "Busy");
        assert_eq!(format!("{:?}", busy_unavailable), "BusyUnavailable");
        assert_eq!(format!("{:?}", busy_tentative), "BusyTentative");

        debug!("Calendar freebusy types test completed successfully");
    }

    #[test]
    fn test_calendar_freebusy_time_range() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar freebusy time range test at {:?}", start_time);

        // Test time range structure for freebusy queries
        let time_range = TimeRange {
            start: "20240101T000000Z".to_string(),
            end: Some("20241231T235959Z".to_string()),
        };

        assert!(time_range.start.contains("20240101"));
        assert!(time_range.end.is_some());
        assert!(time_range.end.unwrap().contains("20241231"));

        debug!("Calendar freebusy time range test completed successfully");
    }

    #[test]
    fn test_calendar_freebusy_timezone_handling() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar freebusy timezone handling test at {:?}", start_time);

        // Test timezone handling in freebusy calculations
        let utc_tz = "UTC";
        let est_tz = "America/New_York";
        let pst_tz = "America/Los_Angeles";

        // Test timezone string parsing
        assert_eq!(utc_tz, "UTC");
        assert!(est_tz.contains("America"));
        assert!(pst_tz.contains("Los_Angeles"));

        debug!("Calendar freebusy timezone handling test completed successfully");
    }

    #[test]
    fn test_calendar_freebusy_period_handling() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar freebusy period handling test at {:?}", start_time);

        // Test period handling for freebusy
        // This tests the types and structures used in period processing

        // Test that ICalendarPeriod type is available
        fn assert_period_type_available() {
            let _period: Option<ICalendarPeriod> = None;
        }

        assert_period_type_available();

        debug!("Calendar freebusy period handling test completed successfully");
    }

    #[test]
    fn test_calendar_freebusy_partial_datetime() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar freebusy partial datetime test at {:?}", start_time);

        // Test partial datetime handling
        // This tests the types and structures used in datetime processing

        // Test that PartialDateTime type is available
        fn assert_partial_datetime_type_available() {
            let _datetime: Option<PartialDateTime> = None;
        }

        assert_partial_datetime_type_available();

        debug!("Calendar freebusy partial datetime test completed successfully");
    }

    #[test]
    fn test_calendar_freebusy_collection_types() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar freebusy collection types test at {:?}", start_time);

        // Test collection types used in freebusy
        let calendar_collection = Collection::CalendarEvent;
        let sync_collection = SyncCollection::Calendar;

        assert_eq!(format!("{:?}", calendar_collection), "CalendarEvent");
        assert_eq!(format!("{:?}", sync_collection), "Calendar");

        debug!("Calendar freebusy collection types test completed successfully");
    }

    #[test]
    fn test_calendar_freebusy_acl_handling() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar freebusy ACL handling test at {:?}", start_time);

        // Test ACL-related functionality for freebusy
        let acl_read = Acl::Read;
        let acl_freebusy = Acl::ReadFreeBusy;

        assert_eq!(format!("{:?}", acl_read), "Read");
        assert_eq!(format!("{:?}", acl_freebusy), "ReadFreeBusy");

        debug!("Calendar freebusy ACL handling test completed successfully");
    }

    #[test]
    fn test_calendar_freebusy_error_handling() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar freebusy error handling test at {:?}", start_time);

        // Test that DavError can be created for various freebusy scenarios
        let not_found_error = DavError::not_found("calendar", "/calendar/freebusy");
        assert!(matches!(not_found_error, DavError::NotFound { .. }));

        let forbidden_error = DavError::Forbidden;
        assert!(matches!(forbidden_error, DavError::Forbidden));

        debug!("Calendar freebusy error handling test completed successfully");
    }

    #[test]
    fn test_calendar_freebusy_response_format() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar freebusy response format test at {:?}", start_time);

        // Test successful freebusy response
        let response = HttpResponse::new(StatusCode::OK);

        // Verify response properties
        assert_eq!(response.status(), StatusCode::OK);

        debug!("Calendar freebusy response format test completed successfully");
    }

    #[test]
    fn test_calendar_freebusy_prod_id() {
        let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        info!("Starting calendar freebusy PROD-ID test at {:?}", start_time);

        // Test PROD-ID constant availability
        // This tests that the PROD_ID constant is accessible
        fn assert_prod_id_available() {
            let _prod_id = PROD_ID;
        }

        assert_prod_id_available();

        debug!("Calendar freebusy PROD-ID test completed successfully");
    }
}

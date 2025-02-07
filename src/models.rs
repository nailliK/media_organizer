use crate::schema::track_files;
use diesel::prelude::*;

#[derive(Debug, Queryable, Selectable, Clone, PartialEq)]
#[diesel(table_name = track_files)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct TrackFile {
    pub id: i32,
    pub barcode: Option<String>,
    pub path: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub title: Option<String>,
    pub track_number: Option<i32>,
    pub disc_number: Option<i32>,
    pub disc_total: Option<i32>,
    pub year: Option<i32>,
    pub processed: bool,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = track_files)]
pub struct NewTrackFile<'a> {
    pub barcode: Option<&'a str>,
    pub path: &'a str,
    pub artist: Option<&'a str>,
    pub album: Option<&'a str>,
    pub title: Option<&'a str>,
    pub track_number: Option<i32>,
    pub disc_number: Option<i32>,
    pub disc_total: Option<i32>,
    pub year: Option<i32>,
    pub processed: bool,
}

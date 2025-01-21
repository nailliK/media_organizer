use crate::models::*;
use crate::schema::track_files;
use crate::schema::*;
use chrono::Datelike;
use chrono::NaiveDate;
use diesel::dsl::max;
use diesel::prelude::*;
use diesel::row::NamedRow;
use dotenvy::dotenv;
use lofty::file::TaggedFileExt;
use lofty::read_from_path;
use lofty::tag::{Accessor, ItemKey};
use musicbrainz_rs::entity::release::{Release, ReleaseSearchQuery};
use musicbrainz_rs::prelude::*;
use reqwest::header::{ACCEPT, ACCEPT_LANGUAGE, COOKIE, DNT};
use sanitize_filename::sanitize;
use std::{env, fs};
use strsim::levenshtein;
pub struct MediaOrganizer {
    connection: SqliteConnection,
}

impl MediaOrganizer {
    pub fn new() -> Self {
        MediaOrganizer {
            connection: MediaOrganizer::establish_connection(),
        }
    }

    pub fn establish_connection() -> SqliteConnection {
        dotenv().ok();

        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        SqliteConnection::establish(&database_url)
            .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
    }

    fn is_fuzzy_match(s1: &str, s2: &str, threshold: usize) -> bool {
        levenshtein(s1, s2) <= threshold
    }

    fn delete_file(file_path: &str) {
        match fs::remove_file(file_path) {
            Ok(_) => println!("File deleted successfully."),
            Err(e) => println!("Error deleting file: {}", e),
        }
    }

    pub fn parse_file(&mut self, file_path: &str) {
        println!("Parsing file: {}", file_path);

        let valid_extensions = [".mp3", ".wav", ".flac", ".aiff"];
        if !valid_extensions.iter().any(|ext| file_path.ends_with(ext)) {
            println!("File is not an audio file. Deleting.");

            Self::delete_file(file_path);
            return;
        }

        let existing_track_file = track_files::table
            .filter(track_files::path.eq(file_path))
            .first::<TrackFile>(&mut self.connection)
            .optional()
            .unwrap();

        if existing_track_file.is_some() {
            println!("Track already exists in the database.");
            return;
        }

        if existing_track_file.is_none() {
            println!("Track does not exist in the database. Inserting.");

            let tagged_file = match read_from_path(file_path) {
                Ok(file) => file,
                Err(e) => {
                    eprintln!("Error reading file: {}", e);
                    return;
                }
            };

            let tag = tagged_file.first_tag();

            if let Some(tag) = tag {
                for item in tag.items() {
                    println!("Key: {:?}, Value: {:?}", item.key(), item.value());
                }

                let barcode = tag
                    .items()
                    .find(|item| *item.key() == ItemKey::Barcode)
                    .and_then(|item| item.value().text());
                let artist = tag
                    .items()
                    .find(|item| *item.key() == ItemKey::AlbumArtist)
                    .and_then(|item| item.value().text())
                    .or_else(|| {
                        tag.items()
                            .find(|item| *item.key() == ItemKey::TrackArtist)
                            .and_then(|item| item.value().text())
                    });
                let album = tag
                    .items()
                    .find(|item| *item.key() == ItemKey::AlbumTitle)
                    .and_then(|item| item.value().text());
                let title = tag
                    .items()
                    .find(|item| *item.key() == ItemKey::TrackTitle)
                    .and_then(|item| item.value().text());

                let new_track_file = NewTrackFile {
                    path: file_path,
                    barcode: barcode.as_deref(),
                    artist: artist.as_deref(),
                    album: album.as_deref(),
                    title: title.as_deref(),
                    year: tag.year().map(|y| y as i32),
                    track_number: tag.track().map(|n| n as i32),
                    disc_number: tag.disk().map(|n| n as i32),
                    disc_total: tag.disk_total().map(|n| n as i32),
                    processed: false,
                };

                println!("Inserting track: {:?}", new_track_file);

                diesel::insert_into(track_files::table)
                    .values(&new_track_file)
                    .returning(TrackFile::as_returning())
                    .get_result(&mut self.connection)
                    .expect("Error saving new track");
            } else {
                eprintln!("No tag found for file: {}", file_path);
            }
        }
    }

    pub fn parse_media_directory(&mut self, dir_name: &str) {
        println!("Parsing directory: {}", dir_name);

        let target_dir = fs::read_dir(dir_name).unwrap();

        for entry in target_dir {
            let entry = entry.unwrap();

            if entry.path().is_dir() {
                self.parse_media_directory(entry.path().to_str().unwrap());
            } else {
                self.parse_file(entry.path().to_str().unwrap());
            }
        }
    }

    pub async fn update_disc_totals(&mut self) {
        use diesel::dsl::max;
        use diesel::prelude::*;

        let connection = &mut self.connection;

        let artist_albums = track_files::table
            .select((track_files::artist, track_files::album))
            .distinct()
            .load::<(Option<String>, Option<String>)>(connection)
            .expect("Error loading artist albums");

        for (artist, album) in artist_albums {
            if let (Some(artist), Some(album)) = (artist, album) {
                let max_disc_number = track_files::table
                    .filter(track_files::artist.eq(&artist))
                    .filter(track_files::album.eq(&album))
                    .select(max(track_files::disc_number))
                    .first::<Option<i32>>(connection)
                    .expect("Error finding max disc number");

                if let Some(disc_total) = max_disc_number {
                    if (disc_total > 1) {
                        println!(
                            "Updating disc total for {} - {} to {}",
                            artist, album, disc_total
                        );
                    }
                    diesel::update(
                        track_files::table
                            .filter(track_files::artist.eq(&artist))
                            .filter(track_files::album.eq(&album)),
                    )
                    .set(track_files::disc_total.eq(disc_total))
                    .execute(connection)
                    .expect("Error updating disc total");
                }
            }
        }
    }

    pub async fn deezer_object_api_call(
        &mut self,
        base_url: String,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let response = client
            .get(&base_url)
            .header(ACCEPT, "*/*")
            .header(ACCEPT_LANGUAGE, "en-US,en;q=0.9")
            .header(COOKIE, "arl=; sid=; refresh-token=;")
            .header(DNT, "1")
            .send()
            .await?;

        let response_json = response.json::<serde_json::Value>().await?;
        Ok(response_json)
    }

    pub async fn recursive_deezer_api_call(
        &mut self,
        base_url: String,
    ) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let mut results = Vec::new();
        let mut next_url = Some(base_url);

        let arl = env::var("DEEZER_ARL")?;
        let sid = env::var("DEEZER_SID")?;
        let refresh_token = env::var("DEEZER_REFRESH_TOKEN")?;

        while let Some(url) = next_url {
            let response = client
                .get(&url)
                .header(ACCEPT, "*/*")
                .header(ACCEPT_LANGUAGE, "en-US,en;q=0.9")
                .header(
                    COOKIE,
                    format!("arl={}; sid={}; refresh-token={};", arl, sid, refresh_token),
                )
                .header(DNT, "1")
                .send()
                .await?;

            let response_json = response.json::<serde_json::Value>().await?;
            if let Some(data) = response_json.get("data") {
                if let Some(array) = data.as_array() {
                    results.extend(array.clone());
                }
            }

            next_url = response_json
                .get("next")
                .and_then(|n| n.as_str())
                .map(String::from);
        }

        Ok(results)
    }
    pub fn move_media_by_metadata(&mut self, base_dir: &str) {
        let tracks = track_files::table
            .filter(track_files::processed.eq(false))
            // .limit(10000)
            .load::<TrackFile>(&mut self.connection)
            .expect("Error loading tracks");

        for track in tracks {
            let extension = track.path.split('.').last().unwrap();
            let artist = sanitize(track.artist.unwrap());
            let album = sanitize(track.album.unwrap());
            let title = sanitize(track.title.unwrap());
            let year = track.year.unwrap();
            let track_number = track.track_number.unwrap();
            let disc_number = track.disc_number.unwrap_or_else(|| 1);
            let disc_total = track.disc_total.unwrap_or_else(|| 1);

            let mut target_path = format!("{}/{}", base_dir, artist);
            if !std::path::Path::new(&target_path).exists() {
                println!("Creating directory: {}", target_path);
                fs::create_dir_all(&target_path).expect("Error creating artist directory");
            }

            target_path = format!("{}/{} ({:04})", target_path, album, year);
            if !std::path::Path::new(&target_path).exists() {
                println!("Creating directory: {}", target_path);
                fs::create_dir_all(&target_path).expect("Error creating album directory");
            }

            if disc_total > 1 {
                target_path = format!("{}/CD {:02}", target_path, disc_number);
                if !std::path::Path::new(&target_path).exists() {
                    println!("Creating directory: {}", target_path);
                    fs::create_dir_all(&target_path).expect("Error creating disc directory");
                }
            }

            target_path = format!(
                "{}/{:02} - {}.{}",
                target_path, track_number, title, extension
            );

            println!("Moving file from {} to {}", track.path, target_path);

            match fs::rename(&track.path, &target_path) {
                Ok(_) => {
                    println!("Moved file from {} to {}", track.path, target_path);
                    diesel::update(track_files::table.find(track.id))
                        .set(track_files::processed.eq(true))
                        .execute(&mut self.connection)
                        .expect("Error updating track processed");
                }
                Err(e) => {
                    eprintln!("Error moving file: {}", e);
                }
            }
        }
    }
}

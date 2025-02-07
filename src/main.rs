use media_organizer::media_organizer::MediaOrganizer;

#[tokio::main]
async fn main() {
    let mut media_organizer = MediaOrganizer::new();
    let dir =
        "/Users/killian/Library/CloudStorage/GoogleDrive-killianlgrant@gmail.com/My Drive/Music";

    media_organizer.parse_media_directory(dir);
    media_organizer.find_and_remove_duplicates();
    // media_organizer.update_disc_totals().await;
    // media_organizer.move_media_by_metadata("/home/killian/Drive/Music");

    // media_organizer.find_missing_metadata().await;
    // media_organizer.move_media_by_metadata("/Volumes/DRIVE/Music");
}

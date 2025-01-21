use media_organizer::media_organizer::MediaOrganizer;

#[tokio::main]
async fn main() {
    let mut media_organizer = MediaOrganizer::new();
    // media_organizer.parse_media_directory("/home/killian/Drive/Music");
    // media_organizer.update_disc_totals().await;
    media_organizer.move_media_by_metadata("/home/killian/Drive/Music");

    // media_organizer.find_missing_metadata().await;
    // media_organizer.move_media_by_metadata("/Volumes/DRIVE/Music");
}

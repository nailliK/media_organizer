// @generated automatically by Diesel CLI.

diesel::table! {
    track_files (id) {
        id -> Integer,
        barcode -> Nullable<Text>,
        path -> Text,
        artist -> Nullable<Text>,
        album -> Nullable<Text>,
        title -> Nullable<Text>,
        year -> Nullable<Integer>,
        track_number -> Nullable<Integer>,
        disc_number -> Nullable<Integer>,
        disc_total -> Nullable<Integer>,
        processed -> Bool,
    }
}

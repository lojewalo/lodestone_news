table! {
    news_items (id) {
        id -> Integer,
        title -> Text,
        url -> Text,
        description -> Nullable<Text>,
        fields -> Nullable<Text>,
        image -> Nullable<Text>,
        lodestone_id -> Text,
        kind -> SmallInt,
        created -> Timestamp,
        tag -> Nullable<Text>,
    }
}

table! {
    send_records (server_id, news_id) {
        server_id -> Integer,
        news_id -> Integer,
    }
}

table! {
    servers (id) {
        id -> Integer,
        title -> Text,
        url -> Text,
        created -> Timestamp,
    }
}

joinable!(send_records -> news_items (news_id));
joinable!(send_records -> servers (server_id));

allow_tables_to_appear_in_same_query!(
    news_items,
    send_records,
    servers,
);

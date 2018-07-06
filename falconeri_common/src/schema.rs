table! {
    use diesel::sql_types::*;
    use models::sql_types::Status;

    data (id) {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        status -> Status,
        job_id -> Uuid,
        source_uri -> Text,
        error_message -> Nullable<Text>,
    }
}

table! {
    use diesel::sql_types::*;
    use models::sql_types::Status;

    jobs (id) {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        status -> Status,
        pipeline_spec -> Jsonb,
        destination_uri -> Text,
    }
}

joinable!(data -> jobs (job_id));

allow_tables_to_appear_in_same_query!(
    data,
    jobs,
);

table! {
    use diesel::sql_types::*;
    use crate::models::sql_types::Status;

    datums (id) {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        status -> Status,
        job_id -> Uuid,
        error_message -> Nullable<Text>,
        node_name -> Nullable<Text>,
        pod_name -> Nullable<Text>,
        backtrace -> Nullable<Text>,
        output -> Nullable<Text>,
        attempted_run_count -> Int4,
        maximum_allowed_run_count -> Int4,
    }
}

table! {
    use diesel::sql_types::*;

    input_files (id) {
        id -> Uuid,
        created_at -> Timestamp,
        datum_id -> Uuid,
        uri -> Text,
        local_path -> Text,
        job_id -> Uuid,
    }
}

table! {
    use diesel::sql_types::*;
    use crate::models::sql_types::Status;

    jobs (id) {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        status -> Status,
        pipeline_spec -> Jsonb,
        job_name -> Text,
        command -> Array<Text>,
        egress_uri -> Text,
    }
}

table! {
    use diesel::sql_types::*;
    use crate::models::sql_types::Status;

    output_files (id) {
        id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        status -> Status,
        job_id -> Uuid,
        datum_id -> Uuid,
        uri -> Text,
    }
}

joinable!(datums -> jobs (job_id));
joinable!(input_files -> datums (datum_id));
joinable!(output_files -> datums (datum_id));
joinable!(output_files -> jobs (job_id));

allow_tables_to_appear_in_same_query!(datums, input_files, jobs, output_files,);

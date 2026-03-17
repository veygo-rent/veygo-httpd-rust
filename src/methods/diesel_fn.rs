use diesel::define_sql_function;
use diesel::sql_types::{Date, Nullable, Text, Timestamptz, Timestamp};

define_sql_function! {
    fn coalesce(x: Nullable<Timestamptz>, y: Timestamptz) -> Timestamptz;
}

define_sql_function! {
    fn greatest(x: Timestamptz, y: Timestamptz) -> Timestamptz;
}

define_sql_function! {
    #[sql_name = "to_char"]
    fn to_char_tstz(ts: Timestamptz, fmt: Text) -> Text;
}

define_sql_function! {
    #[sql_name = "extract"]
    fn extract_date(text: Text, date: Date) -> Numeric;
}

define_sql_function! {
    #[sql_name = "extract"]
    fn extract_ts(text: Text, ts: Timestamp) -> Numeric;
}

define_sql_function! {
    fn now() -> Timestamptz;
}

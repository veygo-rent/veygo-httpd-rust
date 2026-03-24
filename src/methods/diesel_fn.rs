use diesel::define_sql_function;
use diesel::sql_types::{Nullable, Text, Timestamptz};

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
    fn now() -> Timestamptz;
}

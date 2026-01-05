use diesel::define_sql_function;
use diesel::sql_types::{Nullable, Text, Timestamptz};

define_sql_function! { 
    fn coalesce(x: Nullable<Timestamptz>, y: Timestamptz) -> Timestamptz; 
}

define_sql_function! {
    fn greatest(x: Timestamptz, y: Timestamptz) -> Timestamptz;
}

define_sql_function! {
    fn to_char(ts: Timestamptz, fmt: Text) -> Text;
}

define_sql_function! {
    fn date_trunc(precision: Text, ts: Timestamptz) -> Timestamptz;
}
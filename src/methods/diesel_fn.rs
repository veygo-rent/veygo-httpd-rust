use diesel::define_sql_function;
use diesel::sql_types::{Nullable, Timestamptz};

define_sql_function! { 
    fn coalesce(x: Nullable<Timestamptz>, y: Timestamptz) -> Timestamptz; 
}
define_sql_function! {
    fn greatest(x: Timestamptz, y: Timestamptz) -> Timestamptz;
}
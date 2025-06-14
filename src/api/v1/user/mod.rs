mod change_plan;
mod create;
mod get_files;
mod login;
mod retrieve;
mod update_apartment;
mod update_phone;
mod upload_file;
mod rm_token;

use warp::Filter;

pub fn api_v1_user() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone
{
    warp::path("user")
        .and(
            login::main()
                .or(create::main())
                .or(update_apartment::main())
                .or(update_phone::main())
                .or(upload_file::main())
                .or(get_files::main())
                .or(change_plan::main())
                .or(retrieve::main())
                .or(rm_token::main()),
        )
        .and(warp::path::end())
}

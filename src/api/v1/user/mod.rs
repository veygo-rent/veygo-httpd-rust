mod change_plan;
mod create;
mod get_files;
mod get_users;
mod login;
mod retrieve;
mod rm_token;
mod update_apartment;
mod update_apns;
mod update_phone;
mod upload_file;

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
                .or(rm_token::main())
                .or(update_apns::main())
                .or(get_users::main()),
        )
        .and(warp::path::end())
}

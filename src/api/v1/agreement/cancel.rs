use reqwest::Method;
use warp::{Filter, Rejection, Reply};
use crate::{methods};

pub fn main() -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::path!(String)
        .and(warp::path::end())
        .and(warp::method())
        .and(warp::header::<String>("auth"))
        .and(warp::header::<String>("user-agent"))
        .and_then(async move |_agreement_conf: String, method: Method, _auth: String, _user_agent: String| {

            // Checking method is POST
            if method != Method::DELETE {
                return methods::standard_replies::method_not_allowed_response_405();
            }

            methods::standard_replies::not_implemented_response()
        })
}
use crate::api::{self, ApiResult, AuthResult, PagedQuery};
use warp::{Filter, Rejection, Reply};

pub fn routes() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let list_tags = warp::get()
        .and(warp::path!("tags"))
        .and(api::auth())
        .and(warp::query())
        .map(list_tags)
        .map(api::Reply::from);

    list_tags
}

fn list_tags(_auth: AuthResult, _query_info: PagedQuery) -> ApiResult<()> {
    unimplemented!()
}

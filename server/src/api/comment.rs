use crate::api::{ApiResult, AuthResult, DeleteRequest, PagedQuery, PagedResponse, RatingRequest, ResourceQuery};
use crate::model::comment::{NewComment, NewCommentScore};
use crate::model::enums::{ResourceType, Score};
use crate::resource::comment::{CommentInfo, FieldTable};
use crate::schema::{comment, comment_score, database_statistics};
use crate::time::DateTime;
use crate::{api, config, db, resource, search};
use diesel::dsl::exists;
use diesel::prelude::*;
use serde::Deserialize;
use warp::{Filter, Rejection, Reply};

pub fn routes() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let list_comments = warp::get()
        .and(api::auth())
        .and(warp::path!("comments"))
        .and(warp::query())
        .map(list_comments)
        .map(api::Reply::from);
    let get_comment = warp::get()
        .and(api::auth())
        .and(warp::path!("comment" / i64))
        .and(api::resource_query())
        .map(get_comment)
        .map(api::Reply::from);
    let create_comment = warp::post()
        .and(api::auth())
        .and(warp::path!("comments"))
        .and(api::resource_query())
        .and(warp::body::json())
        .map(create_comment)
        .map(api::Reply::from);
    let update_comment = warp::put()
        .and(api::auth())
        .and(warp::path!("comment" / i64))
        .and(api::resource_query())
        .and(warp::body::json())
        .map(update_comment)
        .map(api::Reply::from);
    let rate_comment = warp::put()
        .and(api::auth())
        .and(warp::path!("comment" / i64 / "score"))
        .and(api::resource_query())
        .and(warp::body::json())
        .map(rate_comment)
        .map(api::Reply::from);
    let delete_comment = warp::delete()
        .and(api::auth())
        .and(warp::path!("comment" / i64))
        .and(warp::body::json())
        .map(delete_comment)
        .map(api::Reply::from);

    list_comments
        .or(get_comment)
        .or(create_comment)
        .or(update_comment)
        .or(rate_comment)
        .or(delete_comment)
}

const MAX_COMMENTS_PER_PAGE: i64 = 50;

fn create_field_table(fields: Option<&str>) -> Result<FieldTable<bool>, Box<dyn std::error::Error>> {
    fields
        .map(resource::comment::Field::create_table)
        .transpose()
        .map(|opt_table| opt_table.unwrap_or(FieldTable::filled(true)))
        .map_err(Box::from)
}

fn list_comments(auth: AuthResult, query: PagedQuery) -> ApiResult<PagedResponse<CommentInfo>> {
    let client = auth?;
    api::verify_privilege(client, config::privileges().comment_list)?;

    let client_id = client.map(|user| user.id);
    let offset = query.offset.unwrap_or(0);
    let limit = std::cmp::min(query.limit.get(), MAX_COMMENTS_PER_PAGE);
    let fields = create_field_table(query.fields())?;

    db::get_connection()?.transaction(|conn| {
        let mut search_criteria = search::comment::parse_search_criteria(query.criteria())?;
        search_criteria.add_offset_and_limit(offset, limit);
        let sql_query = search::comment::build_query(&search_criteria)?;

        let total = if search_criteria.has_filter() {
            let count_query = search::comment::build_query(&search_criteria)?;
            count_query.count().first(conn)?
        } else {
            database_statistics::table
                .select(database_statistics::comment_count)
                .first(conn)?
        };

        let selected_comments: Vec<i64> = search::comment::get_ordered_ids(conn, sql_query, &search_criteria)?;
        Ok(PagedResponse {
            query: query.query.query,
            offset,
            limit,
            total,
            results: CommentInfo::new_batch_from_ids(conn, client_id, selected_comments, &fields)?,
        })
    })
}

fn get_comment(auth: AuthResult, comment_id: i64, query: ResourceQuery) -> ApiResult<CommentInfo> {
    let client = auth?;
    api::verify_privilege(client, config::privileges().comment_view)?;

    let client_id = client.map(|user| user.id);
    let fields = create_field_table(query.fields())?;
    db::get_connection()?.transaction(|conn| {
        let comment_exists: bool = diesel::select(exists(comment::table.find(comment_id))).get_result(conn)?;
        if !comment_exists {
            return Err(api::Error::NotFound(ResourceType::Comment));
        }
        CommentInfo::new_from_id(conn, client_id, comment_id, &fields).map_err(api::Error::from)
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
struct NewCommentInfo {
    post_id: i64,
    text: String,
}

fn create_comment(auth: AuthResult, query: ResourceQuery, comment_info: NewCommentInfo) -> ApiResult<CommentInfo> {
    let client = auth?;
    api::verify_privilege(client, config::privileges().comment_create)?;

    let user_id = client.ok_or(api::Error::NotLoggedIn).map(|user| user.id)?;
    let fields = create_field_table(query.fields())?;
    let new_comment = NewComment {
        user_id: Some(user_id),
        post_id: comment_info.post_id,
        text: &comment_info.text,
        creation_time: DateTime::now(),
    };

    let mut conn = db::get_connection()?;
    let comment_id: i64 = diesel::insert_into(comment::table)
        .values(new_comment)
        .returning(comment::id)
        .get_result(&mut conn)?;
    conn.transaction(|conn| {
        CommentInfo::new_from_id(conn, Some(user_id), comment_id, &fields).map_err(api::Error::from)
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CommentUpdate {
    version: DateTime,
    text: String,
}

fn update_comment(
    auth: AuthResult,
    comment_id: i64,
    query: ResourceQuery,
    update: CommentUpdate,
) -> ApiResult<CommentInfo> {
    let client = auth?;
    let client_id = client.map(|user| user.id);
    let fields = create_field_table(query.fields())?;

    let mut conn = db::get_connection()?;
    conn.transaction(|conn| {
        let (comment_owner, comment_version): (Option<i64>, DateTime) = comment::table
            .find(comment_id)
            .select((comment::user_id, comment::last_edit_time))
            .first(conn)?;
        api::verify_version(comment_version, update.version)?;

        let required_rank = match client_id.is_some() && client_id == comment_owner {
            true => config::privileges().comment_edit_own,
            false => config::privileges().comment_edit_any,
        };
        api::verify_privilege(client, required_rank)?;

        diesel::update(comment::table.find(comment_id))
            .set((comment::text.eq(update.text), comment::last_edit_time.eq(DateTime::now())))
            .execute(conn)
            .map_err(api::Error::from)
    })?;
    conn.transaction(|conn| CommentInfo::new_from_id(conn, client_id, comment_id, &fields).map_err(api::Error::from))
}

fn rate_comment(
    auth: AuthResult,
    comment_id: i64,
    query: ResourceQuery,
    rating: RatingRequest,
) -> ApiResult<CommentInfo> {
    let client = auth?;
    api::verify_privilege(client, config::privileges().comment_score)?;

    let user_id = client.ok_or(api::Error::NotLoggedIn).map(|user| user.id)?;
    let fields = create_field_table(query.fields())?;

    let mut conn = db::get_connection()?;
    conn.transaction(|conn| {
        diesel::delete(comment_score::table.find((comment_id, user_id))).execute(conn)?;

        if let Ok(score) = Score::try_from(*rating) {
            let new_comment_score = NewCommentScore {
                comment_id,
                user_id,
                score,
            };
            diesel::insert_into(comment_score::table)
                .values(new_comment_score)
                .execute(conn)?;
        }
        Ok::<_, api::Error>(())
    })?;
    conn.transaction(|conn| {
        CommentInfo::new_from_id(conn, Some(user_id), comment_id, &fields).map_err(api::Error::from)
    })
}

fn delete_comment(auth: AuthResult, comment_id: i64, client_version: DeleteRequest) -> ApiResult<()> {
    let client = auth?;
    let client_id = client.map(|user| user.id);

    db::get_connection()?.transaction(|conn| {
        let (comment_owner, comment_version): (Option<i64>, DateTime) = comment::table
            .find(comment_id)
            .select((comment::user_id, comment::last_edit_time))
            .first(conn)?;
        api::verify_version(comment_version, *client_version)?;

        let required_rank = match client_id.is_some() && client_id == comment_owner {
            true => config::privileges().comment_delete_own,
            false => config::privileges().comment_delete_any,
        };
        api::verify_privilege(client, required_rank)?;

        diesel::delete(comment::table.find(comment_id)).execute(conn)?;
        Ok(())
    })
}

#[cfg(test)]
mod test {
    use crate::api::ApiResult;
    use crate::model::comment::Comment;
    use crate::schema::{comment, comment_statistics, database_statistics};
    use crate::test::*;
    use crate::time::DateTime;
    use diesel::dsl::exists;
    use diesel::prelude::*;
    use serial_test::{parallel, serial};

    // Exclude fields that involve creation_time or last_edit_time
    const FIELDS: &str = "&fields=id,postId,text,user,score,ownScore";

    #[tokio::test]
    #[parallel]
    async fn list() -> ApiResult<()> {
        const QUERY: &str = "GET /comments/?query";
        const SORT: &str = "-sort:id&limit=40";
        verify_query(&format!("{QUERY}={SORT}{FIELDS}"), "comment/list.json").await?;
        verify_query(&format!("{QUERY}=sort:score&limit=1{FIELDS}"), "comment/list_highest_score.json").await?;
        verify_query(&format!("{QUERY}=user:regular_user {SORT}{FIELDS}"), "comment/list_regular_user.json").await?;
        verify_query(&format!("{QUERY}=text:*this* {SORT}{FIELDS}"), "comment/list_text_filter.json").await
    }

    #[tokio::test]
    #[parallel]
    async fn get() -> ApiResult<()> {
        const COMMENT_ID: i64 = 3;
        let get_last_edit_time = |conn: &mut PgConnection| -> QueryResult<DateTime> {
            comment::table
                .select(comment::last_edit_time)
                .filter(comment::id.eq(COMMENT_ID))
                .first(conn)
        };

        let mut conn = get_connection()?;
        let last_edit_time = get_last_edit_time(&mut conn)?;

        verify_query(&format!("GET /comment/{COMMENT_ID}/?{FIELDS}"), "comment/get.json").await?;

        let new_last_edit_time = get_last_edit_time(&mut conn)?;
        assert_eq!(new_last_edit_time, last_edit_time);
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn create() -> ApiResult<()> {
        let get_comment_count = |conn: &mut PgConnection| -> QueryResult<i64> {
            database_statistics::table
                .select(database_statistics::comment_count)
                .first(conn)
        };

        let mut conn = get_connection()?;
        let comment_count = get_comment_count(&mut conn)?;

        verify_query(&format!("POST /comments/?{FIELDS}"), "comment/create.json").await?;

        let comment_id: i64 = comment::table
            .select(comment::id)
            .order_by(comment::id.desc())
            .first(&mut conn)?;

        let new_comment_count = get_comment_count(&mut conn)?;
        let comment_score: i64 = comment_statistics::table
            .select(comment_statistics::score)
            .filter(comment_statistics::comment_id.eq(comment_id))
            .first(&mut conn)?;
        assert_eq!(new_comment_count, comment_count + 1);
        assert_eq!(comment_score, 0);

        verify_query(&format!("DELETE /comment/{comment_id}"), "delete.json").await?;

        let new_comment_count = get_comment_count(&mut conn)?;
        let has_comment: bool = diesel::select(exists(comment::table.find(comment_id))).get_result(&mut conn)?;
        assert_eq!(new_comment_count, comment_count);
        assert!(!has_comment);
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn update() -> ApiResult<()> {
        const COMMENT_ID: i64 = 4;
        let get_comment_info = |conn: &mut PgConnection| -> QueryResult<(Comment, i64)> {
            comment::table
                .inner_join(comment_statistics::table)
                .select((Comment::as_select(), comment_statistics::score))
                .filter(comment::id.eq(COMMENT_ID))
                .first(conn)
        };

        let mut conn = get_connection()?;
        let (comment, score) = get_comment_info(&mut conn)?;

        verify_query(&format!("PUT /comment/{COMMENT_ID}/?{FIELDS}"), "comment/update.json").await?;

        let (new_comment, new_score) = get_comment_info(&mut conn)?;
        assert_ne!(new_comment.text, comment.text);
        assert_eq!(new_comment.creation_time, comment.creation_time);
        assert!(new_comment.last_edit_time > comment.last_edit_time);
        assert_eq!(new_score, score);

        verify_query(&format!("PUT /comment/{COMMENT_ID}/?{FIELDS}"), "comment/update_restore.json").await?;

        let (new_comment, new_score) = get_comment_info(&mut conn)?;
        assert_eq!(new_comment.text, comment.text);
        assert_eq!(new_comment.creation_time, comment.creation_time);
        assert!(new_comment.last_edit_time > comment.last_edit_time);
        assert_eq!(new_score, score);
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn rate() -> ApiResult<()> {
        const COMMENT_ID: i64 = 2;
        let get_comment_info = |conn: &mut PgConnection| -> QueryResult<(i64, DateTime)> {
            comment::table
                .inner_join(comment_statistics::table)
                .select((comment_statistics::score, comment::last_edit_time))
                .filter(comment::id.eq(COMMENT_ID))
                .first(conn)
        };

        let mut conn = get_connection()?;
        let (score, last_edit_time) = get_comment_info(&mut conn)?;

        verify_query(&format!("PUT /comment/{COMMENT_ID}/score/?{FIELDS}"), "comment/like.json").await?;

        let (new_score, new_last_edit_time) = get_comment_info(&mut conn)?;
        assert_eq!(new_score, score + 1);
        assert_eq!(new_last_edit_time, last_edit_time);

        verify_query(&format!("PUT /comment/{COMMENT_ID}/score/?{FIELDS}"), "comment/dislike.json").await?;

        let (new_score, new_last_edit_time) = get_comment_info(&mut conn)?;
        assert_eq!(new_score, score - 1);
        assert_eq!(new_last_edit_time, last_edit_time);

        verify_query(&format!("PUT /comment/{COMMENT_ID}/score/?{FIELDS}"), "comment/remove_score.json").await?;

        let (new_score, new_last_edit_time) = get_comment_info(&mut conn)?;
        assert_eq!(new_score, score);
        assert_eq!(new_last_edit_time, last_edit_time);
        Ok(())
    }
}

use crate::api::comment::CommentInfo;
use crate::api::micro::{MicroPool, MicroPost, MicroTag, MicroUser};
use crate::auth::content;
use crate::model::comment::Comment;
use crate::model::enums::{AvatarStyle, MimeType, PostSafety, PostType};
use crate::model::pool::{Pool, PoolPost};
use crate::model::post::{Post, PostFavorite, PostFeature, PostId, PostNote, PostRelation, PostScore, PostTag};
use crate::model::tag::{TagId, TagName};
use crate::model::user::User;
use crate::schema::{
    comment, pool, pool_post, post, post_favorite, post_feature, post_note, post_relation, post_score, post_tag, tag,
    tag_category, tag_name, user,
};
use crate::util::DateTime;
use diesel::dsl;
use diesel::prelude::*;
use serde::Serialize;
use serde_with::skip_serializing_none;
use std::collections::HashMap;
use std::str::FromStr;
use strum::{EnumString, EnumTable};

#[derive(Clone, Copy, EnumString, EnumTable)]
#[strum(serialize_all = "camelCase")]
pub enum Field {
    Version,
    Id,
    User,
    FileSize,
    CanvasWidth,
    CanvasHeight,
    Safety,
    Type,
    MimeType,
    Checksum,
    ChecksumMd5,
    Flags,
    Source,
    CreationTime,
    LastEditTime,
    ContentUrl,
    ThumbnailUrl,
    Tags,
    Comments,
    Relations,
    Pools,
    Notes,
    Score,
    OwnScore,
    OwnFavorite,
    TagCount,
    CommentCount,
    RelationCount,
    NoteCount,
    FavoriteCount,
    FeatureCount,
    LastFeatureTime,
    FavoritedBy,
    HasCustomThumbnail,
}

impl Field {
    pub fn create_table(fields_str: &str) -> Result<FieldTable<bool>, <Self as FromStr>::Err> {
        let mut table = FieldTable::filled(false);
        let fields = fields_str
            .split(',')
            .into_iter()
            .map(Self::from_str)
            .collect::<Result<Vec<_>, _>>()?;
        for field in fields.into_iter() {
            table[field] = true;
        }
        Ok(table)
    }
}

#[skip_serializing_none]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PostInfo {
    version: Option<DateTime>,
    id: Option<i32>,
    user: Option<Option<MicroUser>>,
    file_size: Option<i64>,
    canvas_width: Option<i32>,
    canvas_height: Option<i32>,
    safety: Option<PostSafety>,
    type_: Option<PostType>,
    mime_type: Option<MimeType>,
    checksum: Option<String>,
    #[serde(rename = "checksumMD5")]
    checksum_md5: Option<Option<String>>,
    flags: Option<Option<String>>,
    source: Option<Option<String>>,
    creation_time: Option<DateTime>,
    last_edit_time: Option<DateTime>,
    content_url: Option<String>,
    thumbnail_url: Option<String>,
    tags: Option<Vec<MicroTag>>,
    comments: Option<Vec<CommentInfo>>,
    relations: Option<Vec<MicroPost>>,
    pools: Option<Vec<MicroPool>>,
    notes: Option<Vec<PostNoteInfo>>,
    score: Option<i64>,
    own_score: Option<i32>,
    own_favorite: Option<bool>,
    tag_count: Option<i64>,
    comment_count: Option<i64>,
    relation_count: Option<i64>,
    note_count: Option<i64>,
    favorite_count: Option<i64>,
    feature_count: Option<i64>,
    last_feature_time: Option<Option<DateTime>>,
    favorited_by: Option<Vec<MicroUser>>,
    has_custom_thumbnail: Option<bool>,
}

impl PostInfo {
    pub fn new(
        conn: &mut PgConnection,
        client: Option<i32>,
        post: Post,
        fields: &FieldTable<bool>,
    ) -> QueryResult<Self> {
        let mut post_info = Self::new_batch(conn, client, vec![post], fields)?;
        debug_assert_eq!(post_info.len(), 1);
        Ok(post_info.pop().unwrap())
    }

    pub fn new_batch(
        conn: &mut PgConnection,
        client: Option<i32>,
        mut posts: Vec<Post>,
        fields: &FieldTable<bool>,
    ) -> QueryResult<Vec<Self>> {
        let mut owners = fields[Field::User]
            .then_some(get_post_owners(conn, &posts))
            .transpose()?
            .unwrap_or_default();
        let mut content_urls = fields[Field::ContentUrl]
            .then_some(get_content_urls(&posts))
            .unwrap_or_default();
        let mut thumbnail_urls = fields[Field::ThumbnailUrl]
            .then_some(get_thumbnail_urls(&posts))
            .unwrap_or_default();
        let mut tags = fields[Field::Tags]
            .then_some(get_tags(conn, &posts))
            .transpose()?
            .unwrap_or_default();
        let mut comments = fields[Field::Comments]
            .then_some(get_comments(conn, client, &posts))
            .transpose()?
            .unwrap_or_default();
        let mut relations = fields[Field::Relations]
            .then_some(get_relations(conn, &posts))
            .transpose()?
            .unwrap_or_default();
        let mut pools = fields[Field::Pools]
            .then_some(get_pools(conn, &posts))
            .transpose()?
            .unwrap_or_default();
        let mut notes = fields[Field::Notes]
            .then_some(get_notes(conn, &posts))
            .transpose()?
            .unwrap_or_default();
        let mut scores = fields[Field::Score]
            .then_some(get_scores(conn, &posts))
            .transpose()?
            .unwrap_or_default();
        let mut client_scores = fields[Field::OwnScore]
            .then_some(get_client_scores(conn, client, &posts))
            .transpose()?
            .unwrap_or_default();
        let mut client_favorites = fields[Field::OwnFavorite]
            .then_some(get_client_favorites(conn, client, &posts))
            .transpose()?
            .unwrap_or_default();
        let mut tag_counts = fields[Field::TagCount]
            .then_some(get_tag_counts(conn, &posts))
            .transpose()?
            .unwrap_or_default();
        let mut comment_counts = fields[Field::CommentCount]
            .then_some(get_comment_counts(conn, &posts))
            .transpose()?
            .unwrap_or_default();
        let mut relation_counts = fields[Field::RelationCount]
            .then_some(get_relation_counts(conn, &posts))
            .transpose()?
            .unwrap_or_default();
        let mut note_counts = fields[Field::NoteCount]
            .then_some(get_note_counts(conn, &posts))
            .transpose()?
            .unwrap_or_default();
        let mut favorite_counts = fields[Field::FavoriteCount]
            .then_some(get_favorite_counts(conn, &posts))
            .transpose()?
            .unwrap_or_default();
        let mut feature_counts = fields[Field::FeatureCount]
            .then_some(get_feature_counts(conn, &posts))
            .transpose()?
            .unwrap_or_default();
        let mut last_feature_times = fields[Field::LastFeatureTime]
            .then_some(get_last_feature_times(conn, &posts))
            .transpose()?
            .unwrap_or_default();
        let mut users_who_favorited = fields[Field::FavoritedBy]
            .then_some(get_users_who_favorited(conn, &posts))
            .transpose()?
            .unwrap_or_default();

        let mut results: Vec<Self> = Vec::new();
        while let Some(post) = posts.pop() {
            results.push(Self {
                version: fields[Field::Version].then_some(post.last_edit_time),
                id: fields[Field::Id].then_some(post.id),
                user: owners.pop(),
                file_size: fields[Field::FileSize].then_some(post.file_size),
                canvas_width: fields[Field::CanvasWidth].then_some(post.width),
                canvas_height: fields[Field::CanvasHeight].then_some(post.height),
                safety: fields[Field::Safety].then_some(post.safety),
                type_: fields[Field::Type].then_some(post.type_),
                mime_type: fields[Field::MimeType].then_some(post.mime_type),
                checksum: fields[Field::Checksum].then_some(post.checksum),
                checksum_md5: fields[Field::ChecksumMd5].then_some(post.checksum_md5),
                flags: fields[Field::Flags].then_some(post.flags),
                source: fields[Field::Source].then_some(post.source),
                creation_time: fields[Field::CreationTime].then_some(post.creation_time),
                last_edit_time: fields[Field::LastEditTime].then_some(post.last_edit_time),
                content_url: content_urls.pop(),
                thumbnail_url: thumbnail_urls.pop(),
                tags: tags.pop(),
                relations: relations.pop(),
                notes: notes.pop(),
                score: scores.pop(),
                own_score: client_scores.pop(),
                own_favorite: client_favorites.pop(),
                tag_count: tag_counts.pop(),
                favorite_count: favorite_counts.pop(),
                comment_count: comment_counts.pop(),
                note_count: note_counts.pop(),
                feature_count: feature_counts.pop(),
                relation_count: relation_counts.pop(),
                last_feature_time: last_feature_times.pop(),
                favorited_by: users_who_favorited.pop(),
                has_custom_thumbnail: Some(false), // TODO
                comments: comments.pop(),
                pools: pools.pop(),
            });
        }
        Ok(results)
    }
}

#[derive(Serialize)]
struct PostNoteInfo {
    polygon: Vec<u8>, // Probably not correct type, TODO
    text: String,
}

impl PostNoteInfo {
    pub fn new(note: PostNote) -> Self {
        PostNoteInfo {
            polygon: note.polygon,
            text: note.text,
        }
    }
}

fn get_post_owners(conn: &mut PgConnection, posts: &[Post]) -> QueryResult<Vec<Option<MicroUser>>> {
    let post_ids = posts.iter().map(|post| post.id).collect::<Vec<_>>();
    Ok(post::table
        .filter(post::id.eq_any(&post_ids))
        .inner_join(user::table)
        .select((post::id, user::name, user::avatar_style))
        .load::<(PostId, String, AvatarStyle)>(conn)?
        .grouped_by(&posts)
        .into_iter()
        .map(|mut post_owners| {
            post_owners
                .pop()
                .map(|(_, username, avatar_style)| MicroUser::new2(username, avatar_style))
        })
        .collect())
}

fn get_content_urls(posts: &[Post]) -> Vec<String> {
    posts
        .iter()
        .map(|post| content::post_content_url(post.id, post.mime_type))
        .collect()
}

fn get_thumbnail_urls(posts: &[Post]) -> Vec<String> {
    posts
        .iter()
        .map(|post| post.id)
        .map(content::post_thumbnail_url)
        .collect()
}

fn get_tags(conn: &mut PgConnection, posts: &[Post]) -> QueryResult<Vec<Vec<MicroTag>>> {
    let tags: Vec<TagId> = PostTag::belonging_to(posts)
        .select(post_tag::tag_id)
        .distinct()
        .load(conn)?;
    let usages: HashMap<i32, i64> = post_tag::table
        .group_by(post_tag::tag_id)
        .select((post_tag::tag_id, dsl::count(post_tag::tag_id)))
        .filter(post_tag::tag_id.eq_any(&tags))
        .load(conn)?
        .into_iter()
        .collect();
    let category_names: HashMap<i32, String> = tag_category::table
        .select((tag_category::id, tag_category::name))
        .load(conn)?
        .into_iter()
        .collect();

    let post_tags = PostTag::belonging_to(posts)
        .inner_join(tag::table.inner_join(tag_name::table))
        .select((PostTag::as_select(), tag::category_id, TagName::as_select()))
        .load(conn)?;
    let process_tag = |tag_info: Vec<(PostTag, i32, TagName)>| -> Option<MicroTag> {
        let usages_and_category = tag_info.first().map(|(post_tag, category_id, _)| {
            (usages.get(&post_tag.tag_id).map(|x| *x).unwrap_or(0), category_names[category_id].clone())
        });
        usages_and_category.map(|(usages, category)| {
            let mut names: Vec<TagName> = tag_info.into_iter().map(|(_, _, tag_name)| tag_name).collect();
            names.sort();
            MicroTag {
                names,
                category,
                usages,
            }
        })
    };
    Ok(post_tags
        .grouped_by(posts)
        .into_iter()
        .map(|tags_on_post| {
            tags_on_post
                .grouped_by(&tags)
                .into_iter()
                .filter_map(process_tag)
                .collect()
        })
        .collect())
}

fn get_comments(conn: &mut PgConnection, client: Option<i32>, posts: &[Post]) -> QueryResult<Vec<Vec<CommentInfo>>> {
    Comment::belonging_to(posts)
        .load(conn)?
        .grouped_by(posts)
        .into_iter()
        .map(|post_comments| {
            post_comments
                .into_iter()
                .map(|comment| CommentInfo::new(conn, comment, client))
                .collect()
        })
        .collect()
}

fn get_relations(conn: &mut PgConnection, posts: &[Post]) -> QueryResult<Vec<Vec<MicroPost>>> {
    let related_posts: Vec<(PostId, Post)> = PostRelation::belonging_to(posts)
        .inner_join(post::table.on(post::id.eq(post_relation::child_id)))
        .select((post_relation::parent_id, Post::as_select()))
        .load(conn)?;
    Ok(related_posts
        .grouped_by(posts)
        .into_iter()
        .map(|post_relations| {
            post_relations
                .into_iter()
                .map(|(_, relation)| MicroPost::new(&relation))
                .collect()
        })
        .collect())
}

fn get_pools(conn: &mut PgConnection, posts: &[Post]) -> QueryResult<Vec<Vec<MicroPool>>> {
    let pools: Vec<(PostId, Pool)> = PoolPost::belonging_to(posts)
        .inner_join(pool::table)
        .select((pool_post::post_id, Pool::as_select()))
        .load(conn)?;
    pools
        .grouped_by(posts)
        .into_iter()
        .map(|pools| pools.into_iter().map(|(_, pool)| MicroPool::new(conn, pool)).collect())
        .collect()
}

fn get_notes(conn: &mut PgConnection, posts: &[Post]) -> QueryResult<Vec<Vec<PostNoteInfo>>> {
    Ok(PostNote::belonging_to(posts)
        .select(PostNote::as_select())
        .load(conn)?
        .grouped_by(posts)
        .into_iter()
        .map(|post_notes| post_notes.into_iter().map(PostNoteInfo::new).collect())
        .collect())
}

fn get_scores(conn: &mut PgConnection, posts: &[Post]) -> QueryResult<Vec<i64>> {
    let post_scores: Vec<(PostId, Option<i64>)> = PostScore::belonging_to(posts)
        .group_by(post_score::post_id)
        .select((post_score::post_id, dsl::sum(post_score::score)))
        .load(conn)?;
    Ok(post_scores
        .grouped_by(posts)
        .into_iter()
        .map(|post_scores| post_scores.first().map(|(_, score)| *score).flatten().unwrap_or(0))
        .collect())
}

fn get_client_scores(conn: &mut PgConnection, client: Option<i32>, posts: &[Post]) -> QueryResult<Vec<i32>> {
    Ok(client
        .map(|id| {
            PostScore::belonging_to(posts)
                .filter(post_score::user_id.eq(id))
                .load::<PostScore>(conn)
        })
        .transpose()?
        .map(|results| {
            results
                .grouped_by(posts)
                .into_iter()
                .map(|scores| scores.first().map(|post_score| post_score.score).unwrap_or(0))
                .collect()
        })
        .unwrap_or(vec![0; posts.len()]))
}

fn get_client_favorites(conn: &mut PgConnection, client: Option<i32>, posts: &[Post]) -> QueryResult<Vec<bool>> {
    Ok(client
        .map(|id| {
            PostFavorite::belonging_to(posts)
                .filter(post_favorite::user_id.eq(id))
                .load::<PostFavorite>(conn)
        })
        .transpose()?
        .map(|results| {
            results
                .grouped_by(posts)
                .into_iter()
                .map(|fav| fav.first().is_some())
                .collect()
        })
        .unwrap_or(vec![false; posts.len()]))
}

fn get_tag_counts(conn: &mut PgConnection, posts: &[Post]) -> QueryResult<Vec<i64>> {
    let tag_counts: Vec<(PostId, i64)> = PostTag::belonging_to(posts)
        .group_by(post_tag::post_id)
        .select((post_tag::post_id, dsl::count(post_tag::tag_id)))
        .load(conn)?;
    Ok(tag_counts
        .grouped_by(posts)
        .into_iter()
        .map(|counts| counts.first().map(|(_, count)| *count).unwrap_or(0))
        .collect())
}

fn get_comment_counts(conn: &mut PgConnection, posts: &[Post]) -> QueryResult<Vec<i64>> {
    let comment_counts: Vec<(PostId, i64)> = Comment::belonging_to(posts)
        .group_by(comment::post_id)
        .select((comment::post_id, dsl::count(comment::post_id)))
        .load(conn)?;
    Ok(comment_counts
        .grouped_by(posts)
        .into_iter()
        .map(|counts| counts.first().map(|(_, count)| *count).unwrap_or(0))
        .collect())
}

fn get_relation_counts(conn: &mut PgConnection, posts: &[Post]) -> QueryResult<Vec<i64>> {
    let relation_counts: Vec<(PostId, i64)> = PostRelation::belonging_to(posts)
        .group_by(post_relation::parent_id)
        .select((post_relation::parent_id, dsl::count(post_relation::child_id)))
        .load(conn)?;
    Ok(relation_counts
        .grouped_by(posts)
        .into_iter()
        .map(|counts| counts.first().map(|(_, count)| *count).unwrap_or(0))
        .collect())
}

fn get_note_counts(conn: &mut PgConnection, posts: &[Post]) -> QueryResult<Vec<i64>> {
    let note_counts: Vec<(PostId, i64)> = PostNote::belonging_to(posts)
        .group_by(post_note::post_id)
        .select((post_note::post_id, dsl::count(post_note::id)))
        .load(conn)?;
    Ok(note_counts
        .grouped_by(posts)
        .into_iter()
        .map(|counts| counts.first().map(|(_, count)| *count).unwrap_or(0))
        .collect())
}

fn get_favorite_counts(conn: &mut PgConnection, posts: &[Post]) -> QueryResult<Vec<i64>> {
    let favorite_counts: Vec<(PostId, i64)> = PostFavorite::belonging_to(posts)
        .group_by(post_favorite::post_id)
        .select((post_favorite::post_id, dsl::count(post_favorite::user_id)))
        .load(conn)?;
    Ok(favorite_counts
        .grouped_by(posts)
        .into_iter()
        .map(|counts| counts.first().map(|(_, count)| *count).unwrap_or(0))
        .collect())
}

fn get_feature_counts(conn: &mut PgConnection, posts: &[Post]) -> QueryResult<Vec<i64>> {
    let feature_counts: Vec<(PostId, i64)> = PostFeature::belonging_to(posts)
        .group_by(post_feature::post_id)
        .select((post_feature::post_id, dsl::count(post_feature::id)))
        .load(conn)?;
    Ok(feature_counts
        .grouped_by(posts)
        .into_iter()
        .map(|counts| counts.first().map(|(_, count)| *count).unwrap_or(0))
        .collect())
}

fn get_last_feature_times(conn: &mut PgConnection, posts: &[Post]) -> QueryResult<Vec<Option<DateTime>>> {
    let last_feature_times: Vec<(PostId, Option<DateTime>)> = PostFeature::belonging_to(posts)
        .group_by(post_feature::post_id)
        .select((post_feature::post_id, dsl::max(post_feature::time)))
        .load(conn)?;
    Ok(last_feature_times
        .grouped_by(posts)
        .into_iter()
        .map(|feature_times| feature_times.first().map(|(_, time)| *time).flatten())
        .collect())
}

fn get_users_who_favorited(conn: &mut PgConnection, posts: &[Post]) -> QueryResult<Vec<Vec<MicroUser>>> {
    let users_who_favorited: Vec<(PostId, User)> = PostFavorite::belonging_to(posts)
        .inner_join(user::table)
        .select((post_favorite::post_id, User::as_select()))
        .load(conn)?;
    Ok(users_who_favorited
        .grouped_by(posts)
        .into_iter()
        .map(|users| users.into_iter().map(|(_, user)| MicroUser::new(user)).collect())
        .collect())
}

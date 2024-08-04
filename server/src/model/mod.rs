pub mod comment;
pub mod enums;
pub mod pool;
pub mod post;
pub mod tag;
pub mod user;

pub trait IntegerIdentifiable {
    fn id(&self) -> i32;
}

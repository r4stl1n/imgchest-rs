use time::OffsetDateTime;

/// The user
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct User {
    /// The user name
    pub name: Box<str>,

    /// The number of posts
    pub posts: u64,

    /// The number of comments
    pub comments: u64,

    /// The time this user was created
    #[serde(with = "time::serde::iso8601")]
    pub created: OffsetDateTime,
    //#[serde(flatten)]
    //extra: std::collections::HashMap<Box<str>, serde_json::Value>,
}

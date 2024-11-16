mod post;
mod scraped_post;
mod user;

pub use self::post::File as PostFile;
pub use self::post::Post;
pub use self::post::Privacy as PostPrivacy;
pub use self::scraped_post::File as ScrapedPostFile;
pub use self::scraped_post::FromHtmlError as InvalidScrapedPostError;
pub use self::scraped_post::ScrapedPost;
pub use self::user::User;

/// A request for updating files in bulk.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct ApiUpdateFilesBulkRequest {
    /// The payload
    pub data: Vec<FileUpdate>,
}

/// A file update as part of a bulk file update.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct FileUpdate {
    /// The file id
    pub id: String,

    /// The file description.
    ///
    /// Though the API docs seem to say that this field is nullable,
    /// it is not.
    pub description: String,
}

/// The response to an api request
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct ApiResponse<T> {
    /// The data payload
    pub data: T,
}

/// The response for when the api completed something
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct ApiCompletedResponse {
    /// Whether the operation was successful.
    #[serde(with = "from_str_to_str")]
    pub success: bool,

    /// The operation message response.
    pub message: Option<Box<str>>,
}

mod from_str_to_str {
    use serde::de::Error;
    use std::borrow::Cow;

    pub(crate) fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where
        D: serde::Deserializer<'de>,
        T: std::str::FromStr,
        <T as std::str::FromStr>::Err: std::fmt::Display,
    {
        let value: Cow<str> = serde::Deserialize::deserialize(deserializer)?;
        let value: T = value.parse().map_err(D::Error::custom)?;
        Ok(value)
    }

    pub(crate) fn serialize<S, T>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
        T: std::string::ToString,
    {
        let value = value.to_string();
        serializer.serialize_str(&value)
    }
}

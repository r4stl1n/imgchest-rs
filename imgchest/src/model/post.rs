use std::num::NonZeroU32;
use time::OffsetDateTime;

/// An API post object
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Post {
    /// The post id
    pub id: Box<str>,

    /// The post title
    pub title: Option<Box<str>>,

    /// The post author's username
    pub username: Box<str>,

    /// The privacy of the post
    pub privacy: Privacy,

    /// ?
    pub report_status: i32,

    /// The number of views
    pub views: u64,

    /// Whether the post is nsfw
    #[serde(with = "u8_to_bool")]
    pub nsfw: bool,

    /// The number of images
    pub image_count: u64,

    /// The time this was created
    #[serde(with = "time::serde::iso8601")]
    pub created: OffsetDateTime,

    /// The files of this post
    pub images: Box<[File]>,

    /// The url to delete this post
    ///
    /// Only present if the current user owns this post.
    pub delete_url: Option<Box<str>>,
    // #[serde(flatten)]
    // extra: std::collections::HashMap<Box<str>, serde_json::Value>,
}

/// An API file of a post
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct File {
    /// The id of the image
    pub id: Box<str>,

    /// The file description
    pub description: Option<Box<str>>,

    /// The link to the image file
    pub link: Box<str>,

    /// The position of the image in the post.
    ///
    /// Starts at 1.
    pub position: NonZeroU32,

    /// The time this image was created.
    #[serde(with = "time::serde::iso8601")]
    pub created: OffsetDateTime,

    /// The original name of the image.
    ///
    /// Only present if the current user owns this image.
    pub original_name: Option<Box<str>>,
    // #[serde(flatten)]
    // extra: std::collections::HashMap<Box<str>, serde_json::Value>,
}

/// The post privacy
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Privacy {
    #[serde(rename = "public")]
    Public,

    #[serde(rename = "hidden")]
    Hidden,

    #[serde(rename = "secret")]
    Secret,
}

impl Privacy {
    /// Get this as a str.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Hidden => "hidden",
            Self::Secret => "secret",
        }
    }
}

mod u8_to_bool {
    use serde::de::Error;
    use serde::de::Unexpected;

    pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<bool, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value: u8 = serde::Deserialize::deserialize(deserializer)?;
        match value {
            0 => Ok(false),
            1 => Ok(true),
            n => Err(D::Error::invalid_type(
                Unexpected::Unsigned(n.into()),
                &"an integer that is either 0 or 1",
            )),
        }
    }

    pub(super) fn serialize<S>(value: &bool, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u8(u8::from(*value))
    }
}

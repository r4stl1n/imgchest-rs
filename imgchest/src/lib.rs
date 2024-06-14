mod client;
mod model;

pub use self::client::Client;
pub use crate::client::CreatePostBuilder;
pub use crate::client::UpdateFileBuilder;
pub use crate::client::UpdatePostBuilder;
pub use crate::client::UploadPostFile;
use crate::model::ApiCompletedResponse;
use crate::model::ApiResponse;
use crate::model::ApiUpdateFilesBulkRequest;
pub use crate::model::FileUpdate;
pub use crate::model::InvalidScrapedPostError;
pub use crate::model::InvalidScrapedPostFileError;
pub use crate::model::Post;
pub use crate::model::PostFile;
pub use crate::model::PostPrivacy;
pub use crate::model::ScrapedPost;
pub use crate::model::ScrapedPostFile;
pub use crate::model::User;
pub use reqwest::Body;

/// The error
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Reqwest error
    #[error("reqwest http error")]
    Reqwest(#[from] reqwest::Error),

    /// Failed to join tokio task
    #[error("failed to join tokio task")]
    TokioJoin(#[from] tokio::task::JoinError),

    /// Failed to parse post
    #[error("invalid scraped post")]
    InvalidScrapedPost(#[from] InvalidScrapedPostError),

    /// Failed to parse a scraped post file
    #[error("invalid scraped post file")]
    InvalidScrapedPostFile(#[from] InvalidScrapedPostFileError),

    /// Missing a token
    #[error("missing token")]
    MissingToken,

    /// Missing images
    #[error("missing images")]
    MissingImages,

    /// An api operation was not successful
    #[error("api operation was not successful")]
    ApiOperationFailed,

    /// An api response is missing a message
    #[error("api response missing messsage")]
    ApiResponseMissingMessage,

    /// An api response had un unknown message
    #[error("api response had unknown message \"{message}\"")]
    ApiResponseUnknownMessage {
        /// The unknown message
        message: Box<str>,
    },

    /// Missing description
    #[error("missing description")]
    MissingDescription,

    /// The title is too short.
    #[error("title too short, must be at least 3 characters")]
    TitleTooShort,
}

#[cfg(test)]
mod test {
    use super::*;
    use std::sync::OnceLock;
    use time::format_description::well_known::Iso8601;
    use time::OffsetDateTime;

    const POST_URL: &str = "https://imgchest.com/p/3qe4gdvj4j2";
    const VIDEO_POST_URL: &str = "https://imgchest.com/p/pwl7lgepyx2";

    fn get_token() -> &'static str {
        static TOKEN: OnceLock<String> = OnceLock::new();
        TOKEN.get_or_init(|| {
            let token_env = std::env::var_os("IMGCHEST_TOKEN").map(|token| {
                token
                    .into_string()
                    .expect("\"IMGCHEST_TOKEN\" env var value is not valid unicode")
            });

            if let Some(token) = token_env {
                return token;
            }

            let token_file = match std::fs::read_to_string("token.txt") {
                Ok(token) => Some(token),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                Err(error) => panic!("failed to read token from file: {error}"),
            };

            if let Some(token) = token_file {
                return token;
            }

            panic!("missing token");
        })
    }

    #[tokio::test]
    async fn get_scraped_post() {
        let client = Client::new();
        let post = client
            .get_scraped_post(POST_URL)
            .await
            .expect("failed to get scraped post");
        assert!(&*post.id == "3qe4gdvj4j2");
        assert!(&*post.title == "Donkey Kong - Video Game From The Mid 80's");
        assert!(&*post.username == "LunarLandr");
        // assert!(post.privacy == "public");
        // assert!(post.report_status == 1);
        assert!(post.views >= 198);
        // assert!(post.nsfw == 0);
        assert!(post.image_count == 4);
        // assert!(post.created == "2019-11-03T00:36:00.000000Z");

        assert!(&*post.images[0].id == "nw7w6cmlvye");
        assert!(post.images[0]
            .description
            .as_ref()
            .expect("missing description")
            .starts_with("Released in the arcades in 1981, Donkey Kong"));
        assert!(&*post.images[0].link == "https://cdn.imgchest.com/files/nw7w6cmlvye.png");
        assert!(post.images[0].video_link.is_none());

        assert!(&*post.images[1].id == "kwye3cpag4b");
        assert!(post.images[1].description.as_deref() == Some("amstrad - apple ii - atari - colecovision - c64 - msx\nnes - pc - vic-20 - spectrum - tI-99 4A - arcade"));
        assert!(&*post.images[1].link == "https://cdn.imgchest.com/files/kwye3cpag4b.png");
        assert!(post.images[1].video_link.is_none());

        assert!(&*post.images[2].id == "5g4z9c8ok72");
        assert!(post.images[2].description.is_none());
        assert!(&*post.images[2].link == "https://cdn.imgchest.com/files/5g4z9c8ok72.png");
        assert!(post.images[2].video_link.is_none());

        assert!(&*post.images[3].id == "we4gdcv5j4r");
        assert!(post.images[3].description.is_none());
        assert!(&*post.images[3].link == "https://cdn.imgchest.com/files/we4gdcv5j4r.jpg");
        assert!(post.images[3].video_link.is_none());

        dbg!(&post);
    }

    #[tokio::test]
    async fn get_scraped_video_post() {
        let client = Client::new();
        let post = client
            .get_scraped_post(VIDEO_POST_URL)
            .await
            .expect("failed to get post");

        assert!(&*post.id == "pwl7lgepyx2");
        assert!(&*post.title == "PDN AGIF Issue #1");
        assert!(&*post.username == "Jacob");
        assert!(post.views >= 2537);
        assert!(post.image_count == 1);

        assert!(&*post.images[0].id == "6yxkcz5ml7w");
        assert!(post.images[0].description.as_deref() == Some("Notice how inserting an AGIF is now supported, but does not want to be moved from its initial position."));
        assert!(&*post.images[0].link == "https://cdn.imgchest.com/files/6yxkcz5ml7w.gif");
        assert!(
            post.images[0].video_link.as_deref()
                == Some("https://cdn.imgchest.com/files/6yxkcz5ml7w.mp4")
        );

        dbg!(&post);
    }

    #[tokio::test]
    async fn get_post_no_token() {
        let client = Client::new();

        let err = client
            .get_post("3qe4gdvj4j2")
            .await
            .expect_err("succeeded getting post with no token");
        assert!(matches!(err, Error::MissingToken));
    }

    #[tokio::test]
    async fn get_post() {
        let client = Client::new();
        client.set_token(get_token());

        let post = client
            .get_post("3qe4gdvj4j2")
            .await
            .expect("failed to get post");

        assert!(&*post.id == "3qe4gdvj4j2");
        assert!(post.title.as_deref() == Some("Donkey Kong - Video Game From The Mid 80's"));
        assert!(&*post.username == "LunarLandr");
        assert!(post.privacy == PostPrivacy::Public);
        assert!(post.report_status == 1);
        assert!(post.views >= 198);
        assert!(post.nsfw == 0);
        assert!(post.image_count == 4);
        assert!(
            post.created
                == OffsetDateTime::parse("2019-11-03T00:36:00.000000Z", &Iso8601::DEFAULT).unwrap()
        );
        assert!(post.delete_url.is_none());

        assert!(&*post.images[0].id == "nw7w6cmlvye");
        assert!(post.images[0]
            .description
            .as_ref()
            .expect("missing description")
            .starts_with("**Description**  \nReleased in the arcades in 1981, Donkey Kong"));
        assert!(&*post.images[0].link == "https://cdn.imgchest.com/files/nw7w6cmlvye.png");
        assert!(post.images[0].position.get() == 1);
        assert!(
            post.images[0].created
                == OffsetDateTime::parse("2019-11-03T00:36:00.000000Z", &Iso8601::DEFAULT).unwrap()
        );
        assert!(post.images[0].original_name.is_none());

        assert!(&*post.images[1].id == "kwye3cpag4b");
        assert!(post.images[1].description.as_deref() == Some("amstrad - apple ii - atari - colecovision - c64 - msx\nnes - pc - vic-20 - spectrum - tI-99 4A - arcade"));
        assert!(&*post.images[1].link == "https://cdn.imgchest.com/files/kwye3cpag4b.png");
        assert!(post.images[1].position.get() == 2);
        assert!(
            post.images[1].created
                == OffsetDateTime::parse("2019-11-03T00:36:00.000000Z", &Iso8601::DEFAULT).unwrap()
        );
        assert!(post.images[1].original_name.is_none());

        assert!(&*post.images[2].id == "5g4z9c8ok72");
        assert!(post.images[2].description.as_deref() == Some(""));
        assert!(&*post.images[2].link == "https://cdn.imgchest.com/files/5g4z9c8ok72.png");
        assert!(post.images[2].position.get() == 3);
        assert!(
            post.images[2].created
                == OffsetDateTime::parse("2019-11-03T00:36:00.000000Z", &Iso8601::DEFAULT).unwrap()
        );
        assert!(post.images[2].original_name.is_none());

        assert!(&*post.images[3].id == "we4gdcv5j4r");
        assert!(post.images[3].description.as_deref() == Some(""));
        assert!(&*post.images[3].link == "https://cdn.imgchest.com/files/we4gdcv5j4r.jpg");
        assert!(post.images[3].position.get() == 4);
        assert!(
            post.images[3].created
                == OffsetDateTime::parse("2019-11-03T00:36:00.000000Z", &Iso8601::DEFAULT).unwrap()
        );
        assert!(post.images[3].original_name.is_none());

        dbg!(&post);
    }

    #[tokio::test]
    async fn get_user() {
        let client = Client::new();
        client.set_token(get_token());

        let user = client
            .get_user("LunarLandr")
            .await
            .expect("failed to get user");

        assert!(&*user.name == "LunarLandr");
        assert!(
            user.created
                == OffsetDateTime::parse("2019-09-25T01:00:45.000000Z", &Iso8601::DEFAULT).unwrap()
        );

        dbg!(&user);
    }

    // Endpoint appears disabled
    /*
    #[tokio::test]
    async fn get_file() {
        let client = Client::new();
        client.set_token(get_token());

        let file = client
            .get_file("nw7w6cmlvye")
            .await
            .expect("failed to get file");

        assert!(&*file.id == "nw7w6cmlvye");
        assert!(file
            .description
            .as_ref()
            .expect("missing description")
            .starts_with("**Description**  \nReleased in the arcades in 1981, Donkey Kong"));
        assert!(&*file.link == "https://cdn.imgchest.com/files/nw7w6cmlvye.png");
        assert!(file.position.get() == 1);
        assert!(
            file.created
                == OffsetDateTime::parse("2019-11-03T00:36:00.000000Z", &Iso8601::DEFAULT).unwrap()
        );
        assert!(file.original_name.is_none());

        dbg!(&file);
    }
    */

    #[tokio::test]
    async fn create_post_too_short_title() {
        let client = Client::new();
        client.set_token(get_token());

        let mut builder = CreatePostBuilder::new();
        builder.title("");

        let err = client
            .create_post(builder)
            .await
            .expect_err("title should have been too short");

        assert!(matches!(err, Error::TitleTooShort));
    }
}

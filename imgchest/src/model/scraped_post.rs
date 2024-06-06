use once_cell::sync::Lazy;
use scraper::ElementRef;
use scraper::Html;
use scraper::Selector;

/// An error that may occur while parsing a post
#[derive(Debug, thiserror::Error)]
pub enum FromHtmlError {
    #[error("missing id")]
    MissingId,

    #[error("missing title")]
    MissingTitle,

    #[error("missing username")]
    MissingUsername,

    #[error("missing views")]
    MissingViews,

    #[error("invalid views")]
    InvalidViews(#[source] std::num::ParseIntError),

    #[error("invalid image")]
    InvalidImage(#[from] FromElementError),

    #[error("missing token")]
    MissingToken,
}

/// A Post
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScrapedPost {
    /// The id of the post
    pub id: Box<str>,

    /// The title of the post
    pub title: Box<str>,

    /// The author of the post
    pub username: Box<str>,

    // /// The post privacy
    // pub privacy: String,

    // /// ?
    // pub report_status: u32,
    /// The number of views
    pub views: u64,

    // /// ?
    // pub nsfw: u32,
    /// The number of images
    pub image_count: u32,

    // /// The timestamp of post creation
    // pub created: String,
    /// Post images
    pub images: Box<[File]>,

    /// The number of extra images.
    ///
    /// This is not part of the official api object.
    pub extra_image_count: Option<u32>,

    /// The csrf token for the page
    ///
    /// Only useful for loading more image data.
    /// This is not part of the official api object.
    pub token: Box<str>,
}

impl ScrapedPost {
    /// Parse this from html
    pub(crate) fn from_html(html: &Html) -> Result<Self, FromHtmlError> {
        static ID_SELECTOR: Lazy<Selector> =
            Lazy::new(|| Selector::parse("meta[property=\"og:url\"]").unwrap());
        static TITLE_SELECTOR: Lazy<Selector> =
            Lazy::new(|| Selector::parse("meta[property=\"og:title\"]").unwrap());
        static USER_SELECTOR: Lazy<Selector> =
            Lazy::new(|| Selector::parse("a[href^=\"https://imgchest.com/u/\"]").unwrap());
        static VIEWS_SELECTOR: Lazy<Selector> =
            Lazy::new(|| Selector::parse("meta[name=\"twitter:description\"]").unwrap());
        static POST_FILE_SELECTOR: Lazy<Selector> =
            Lazy::new(|| Selector::parse("#post-images > div[id^=\"image\"]").unwrap());
        static LOAD_MORE_SELECTOR: Lazy<Selector> =
            Lazy::new(|| Selector::parse("#post-images .load-all").unwrap());
        static TOKEN_SELECTOR: Lazy<Selector> =
            Lazy::new(|| Selector::parse("meta[name=\"csrf-token\"]").unwrap());

        let id = html
            .select(&ID_SELECTOR)
            .next()
            .and_then(|meta| {
                Some(
                    meta.value()
                        .attr("content")?
                        .strip_prefix("https://imgchest.com/p/")?
                        .into(),
                )
            })
            .ok_or(FromHtmlError::MissingId)?;

        let title = html
            .select(&TITLE_SELECTOR)
            .next()
            .and_then(|meta| Some(meta.value().attr("content")?.trim().into()))
            .ok_or(FromHtmlError::MissingTitle)?;

        let username = html
            .select(&USER_SELECTOR)
            .next()
            .and_then(|a| {
                Some(
                    a.value()
                        .attr("href")?
                        .strip_prefix("https://imgchest.com/u/")?
                        .into(),
                )
            })
            .ok_or(FromHtmlError::MissingUsername)?;

        let views = html
            .select(&VIEWS_SELECTOR)
            .next()
            .and_then(|meta| {
                Some(
                    meta.value()
                        .attr("content")?
                        .split(' ')
                        .next()?
                        .parse::<u64>()
                        .map_err(FromHtmlError::InvalidViews),
                )
            })
            .ok_or(FromHtmlError::MissingViews)??;

        let images = html
            .select(&POST_FILE_SELECTOR)
            .map(File::from_element)
            .collect::<Result<Vec<_>, _>>()?;

        let extra_image_count: Option<u32> =
            html.select(&LOAD_MORE_SELECTOR).next().and_then(|button| {
                button
                    .text()
                    .map(|text| text.trim())
                    .find(|s| !s.is_empty())?
                    .split(' ')
                    .nth(1)?
                    .parse()
                    .ok()
            });

        // The unwrap here will fail if the number of images cannot fit into a u32.
        // This is incredibly unlikely and probably impossible.
        let image_count = u32::try_from(images.len()).unwrap() + extra_image_count.unwrap_or(0);

        let token = html
            .select(&TOKEN_SELECTOR)
            .next()
            .and_then(|meta| Some(meta.value().attr("content")?.into()))
            .ok_or(FromHtmlError::MissingToken)?;

        Ok(Self {
            id,
            title,
            username,
            views,
            image_count,
            images: images.into(),

            extra_image_count,
            token,
        })
    }
}

/// Returned if a post image could not be extracted from an element
#[allow(clippy::enum_variant_names)]
#[derive(Debug, thiserror::Error)]
pub enum FromElementError {
    #[error("missing id")]
    MissingId,

    #[error("missing link")]
    MissingLink,
}

/// A post file
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct File {
    /// The file id
    pub id: Box<str>,

    /// The file description
    pub description: Option<Box<str>>,

    /// The file link
    pub link: Box<str>,
    // /// The file position
    // pub position: u32,

    // /// The file creation time
    // pub created: u32,
    /// The link of the video, if it exists
    ///
    /// Note that this field is not present on the real api.
    pub video_link: Option<Box<str>>,
}

impl File {
    /// Try to parse this from an element
    pub(crate) fn from_element(element: ElementRef<'_>) -> Result<Self, FromElementError> {
        static DESCRIPTION_SELECTOR: Lazy<Selector> =
            Lazy::new(|| Selector::parse(".description-wrapper").unwrap());
        static LINK_SELECTOR: Lazy<Selector> =
            Lazy::new(|| Selector::parse("a[data-url]").unwrap());
        static VIDEO_LINK_SELECTOR: Lazy<Selector> =
            Lazy::new(|| Selector::parse("video source").unwrap());

        let id = element
            .value()
            .attr("id")
            .and_then(|id| id.split('-').nth(1))
            .ok_or(FromElementError::MissingId)?;

        let mut description = String::with_capacity(256);
        for text in element
            .select(&DESCRIPTION_SELECTOR)
            .take(1)
            .flat_map(|element| {
                element
                    .text()
                    .map(|text| text.trim())
                    .filter(|text| *text != "Description")
            })
        {
            description.push_str(text);
        }
        let description = match description.is_empty() {
            true => None,
            false => Some(description.into()),
        };

        let link = element
            .select(&LINK_SELECTOR)
            .next()
            .and_then(|a| Some(a.value().attr("data-url")?.into()))
            .ok_or(FromElementError::MissingLink)?;

        let video_link = element
            .select(&VIDEO_LINK_SELECTOR)
            .next()
            .and_then(|src| Some(src.value().attr("src")?.into()));

        Ok(Self {
            id: id.into(),
            description,
            link,
            video_link,
        })
    }
}

use once_cell::sync::Lazy;
use scraper::Html;
use scraper::Selector;

static APP_SELECTOR: Lazy<Selector> = Lazy::new(|| Selector::parse("#app").unwrap());

/// An error that may occur while parsing a post
#[derive(Debug, thiserror::Error)]
pub enum FromHtmlError {
    #[error("missing {0}")]
    MissingElement(&'static str),

    #[error("missing attribute {0}")]
    MissingAttribute(&'static str),

    #[error("invalid data page")]
    InvalidDataPage(serde_json::Error),
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

    /// Whether this is nsfw
    pub nsfw: bool,

    /// The number of images
    pub image_count: u64,

    // /// The timestamp of post creation
    // pub created: String,
    /// Post images
    pub images: Box<[File]>,
}

impl ScrapedPost {
    /// Parse this from html
    pub(crate) fn from_html(html: &Html) -> Result<Self, FromHtmlError> {
        // Implement:
        // JSON.parse(document.getElementById('app').getAttribute('data-page'))
        let app_element = html
            .select(&APP_SELECTOR)
            .next()
            .ok_or(FromHtmlError::MissingElement("app div"))?;
        let data_page_attr = app_element
            .attr("data-page")
            .ok_or(FromHtmlError::MissingAttribute("data-page"))?;
        let page_data: PageData =
            serde_json::from_str(data_page_attr).map_err(FromHtmlError::InvalidDataPage)?;

        // Overflowing a u64 with image entries is impossible.
        let image_count = u64::try_from(page_data.props.post.files.len()).unwrap();
        let images: Vec<_> = page_data
            .props
            .post
            .files
            .into_iter()
            .map(|file| File {
                id: file.id,
                description: file.description,
                link: file.link,
                position: file.position,
            })
            .collect();
        Ok(Self {
            id: page_data.props.post.slug,
            title: page_data.props.post.title,
            username: page_data.props.post.user.username,
            views: page_data.props.post.views,
            nsfw: page_data.props.post.nsfw != 0,
            image_count,
            images: images.into(),
        })
    }
}

#[derive(Debug, serde::Deserialize)]
struct PageData {
    props: PageDataProps,
}

#[derive(Debug, serde::Deserialize)]
struct PageDataProps {
    post: PageDataPost,
}

#[derive(Debug, serde::Deserialize)]
struct PageDataPost {
    files: Vec<PageDataFile>,
    nsfw: u8,
    slug: Box<str>,
    title: Box<str>,
    user: PageDataUser,
    views: u64,
}

#[derive(Debug, serde::Deserialize)]
struct PageDataUser {
    username: Box<str>,
}

#[derive(Debug, serde::Deserialize)]
struct PageDataFile {
    id: Box<str>,
    description: Option<Box<str>>,
    link: Box<str>,
    position: u32,
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

    /// The position of the image in the post.
    ///
    /// Starts at 1.
    pub position: u32,
    // /// The file creation time
    // pub created: u32,
}

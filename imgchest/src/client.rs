use crate::ApiCompletedResponse;
use crate::ApiResponse;
use crate::ApiUpdateFilesBulkRequest;
use crate::Error;
use crate::FileUpdate;
use crate::Post;
use crate::PostFile;
use crate::PostPrivacy;
use crate::ScrapedPost;
use crate::User;
use reqwest::header::AUTHORIZATION;
use reqwest::multipart::Form;
use scraper::Html;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
use tokio_util::codec::BytesCodec;
use tokio_util::codec::FramedRead;

const REQUESTS_PER_MINUTE: u8 = 60;
const ONE_MINUTE: Duration = Duration::from_secs(60);
const API_BASE: &str = "https://api.imgchest.com";

/// A builder for creating a post.
///
/// This builder is for the low-level function.
#[derive(Debug)]
pub struct CreatePostBuilder {
    /// The title of the post.
    ///
    /// If specified, it must be at least 3 characters long.
    pub title: Option<String>,

    /// The post privacy.
    ///
    /// Defaults to hidden.
    pub privacy: Option<PostPrivacy>,

    /// Whether the post should be tied to the user.
    pub anonymous: Option<bool>,

    /// Whether this post is nsfw.
    pub nsfw: Option<bool>,

    /// The images of the post
    pub images: Vec<UploadPostFile>,
}

impl CreatePostBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            title: None,
            privacy: None,
            anonymous: None,
            nsfw: None,
            images: Vec::new(),
        }
    }

    /// Set the title.
    ///
    /// It must be at least 3 characters long.
    pub fn title(&mut self, title: impl Into<String>) -> &mut Self {
        self.title = Some(title.into());
        self
    }

    /// Set the post privacy.
    ///
    /// Defaults to hidden.
    pub fn privacy(&mut self, privacy: PostPrivacy) -> &mut Self {
        self.privacy = Some(privacy);
        self
    }

    /// Set whether this post should be anonymous.
    pub fn anonymous(&mut self, anonymous: bool) -> &mut Self {
        self.anonymous = Some(anonymous);
        self
    }

    /// Set whether this post is nsfw.
    pub fn nsfw(&mut self, nsfw: bool) -> &mut Self {
        self.nsfw = Some(nsfw);
        self
    }

    /// Add a new image to this post.
    pub fn image(&mut self, file: UploadPostFile) -> &mut Self {
        self.images.push(file);
        self
    }
}

impl Default for CreatePostBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// A post file that is meant for uploading.
#[derive(Debug)]
pub struct UploadPostFile {
    /// The file name
    file_name: String,

    /// The file body
    body: reqwest::Body,
}

impl UploadPostFile {
    /// Create this from a raw reqwest body.
    pub fn from_body(file_name: &str, body: reqwest::Body) -> Self {
        Self {
            file_name: file_name.into(),
            body,
        }
    }

    /// Create this from bytes.
    pub fn from_bytes(file_name: &str, file_data: Vec<u8>) -> Self {
        Self::from_body(file_name, file_data.into())
    }

    /// Create this from a file.
    pub fn from_file(file_name: &str, file: tokio::fs::File) -> Self {
        let stream = FramedRead::new(file, BytesCodec::new());
        let body = reqwest::Body::wrap_stream(stream);

        Self::from_body(file_name, body)
    }

    /// Create this from a file at the given path.
    pub async fn from_path<P>(path: P) -> std::io::Result<Self>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();

        let file_name = path
            .file_name()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "missing file name"))?
            .to_str()
            .ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::Other, "file name is not valid unicode")
            })?;

        let file = tokio::fs::File::open(path).await?;

        Ok(Self::from_file(file_name, file))
    }
}

/// A builder for updating a post.
#[derive(Debug)]
pub struct UpdatePostBuilder {
    /// The title
    ///
    /// If specified, it must be at least 3 characters long.
    pub title: Option<String>,

    /// The post privacy
    pub privacy: Option<PostPrivacy>,

    /// Whether the post is nsfw
    pub nsfw: Option<bool>,
}

impl UpdatePostBuilder {
    /// Create an empty post update.
    pub fn new() -> Self {
        Self {
            title: None,
            privacy: None,
            nsfw: None,
        }
    }

    /// Update the title.
    ///
    /// It must be at least 3 characters long.
    pub fn title(&mut self, title: impl Into<String>) -> &mut Self {
        self.title = Some(title.into());
        self
    }

    /// Update the privacy.
    pub fn privacy(&mut self, privacy: PostPrivacy) -> &mut Self {
        self.privacy = Some(privacy);
        self
    }

    /// Update the nsfw flag.
    pub fn nsfw(&mut self, nsfw: bool) -> &mut Self {
        self.nsfw = Some(nsfw);
        self
    }
}

impl Default for UpdatePostBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// The client
#[derive(Debug, Clone)]
pub struct Client {
    /// The inner http client
    pub client: reqwest::Client,

    /// Inner client state
    state: Arc<ClientState>,
}

impl Client {
    /// Make a new client
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .cookie_store(true)
            .build()
            .expect("failed to build client");
        let state = Arc::new(ClientState::new());

        Self { client, state }
    }

    /// Scrape a post from a post id.
    ///
    /// # Authorization
    /// This function does NOT require the use of a token.
    pub async fn get_scraped_post(&self, id: &str) -> Result<ScrapedPost, Error> {
        let url = format!("https://imgchest.com/p/{id}");
        let text = self
            .client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        let post = tokio::task::spawn_blocking(move || {
            let html = Html::parse_document(text.as_str());
            ScrapedPost::from_html(&html)
        })
        .await??;

        Ok(post)
    }

    /// Set the token to use for future requests.
    ///
    /// This allows the use of functions that require authorization.
    pub fn set_token<T>(&self, token: T)
    where
        T: AsRef<str>,
    {
        *self
            .state
            .token
            .write()
            .unwrap_or_else(|error| error.into_inner()) = Some(token.as_ref().into());
    }

    /// Get the current token.
    fn get_token(&self) -> Option<Arc<str>> {
        self.state
            .token
            .read()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    /// Get a post by id.
    ///
    /// # Authorization
    /// This function REQUIRES a token.
    pub async fn get_post(&self, id: &str) -> Result<Post, Error> {
        let token = self.get_token().ok_or(Error::MissingToken)?;
        let url = format!("{API_BASE}/v1/post/{id}");

        self.state.ratelimit().await;

        let response = self
            .client
            .get(url)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .send()
            .await?;

        let post: ApiResponse<_> = response.error_for_status()?.json().await?;

        Ok(post.data)
    }

    /// Create a post.
    ///
    /// # Authorization
    /// This function REQUIRES a token.
    pub async fn create_post(&self, data: CreatePostBuilder) -> Result<Post, Error> {
        let token = self.get_token().ok_or(Error::MissingToken)?;
        let url = format!("{API_BASE}/v1/post");

        let mut form = Form::new();

        if let Some(title) = data.title {
            if title.len() < 3 {
                return Err(Error::TitleTooShort);
            }

            form = form.text("title", title);
        }

        if let Some(privacy) = data.privacy {
            form = form.text("privacy", privacy.as_str());
        }

        if let Some(anonymous) = data.anonymous {
            form = form.text("anonymous", bool_to_str(anonymous));
        }

        if let Some(nsfw) = data.nsfw {
            form = form.text("nsfw", bool_to_str(nsfw));
        }

        if data.images.is_empty() {
            return Err(Error::MissingImages);
        }

        for file in data.images {
            let part = reqwest::multipart::Part::stream(file.body).file_name(file.file_name);

            form = form.part("images[]", part);
        }

        self.state.ratelimit().await;

        let response = self
            .client
            .post(url)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .multipart(form)
            .send()
            .await?;

        let post: ApiResponse<_> = response.error_for_status()?.json().await?;

        Ok(post.data)
    }

    /// Update a post.
    ///
    /// # Authorization
    /// This function REQUIRES a token.
    pub async fn update_post(&self, id: &str, data: UpdatePostBuilder) -> Result<Post, Error> {
        let token = self.get_token().ok_or(Error::MissingToken)?;
        let url = format!("{API_BASE}/v1/post/{id}");

        let mut form = Vec::new();

        if let Some(title) = data.title.as_ref() {
            if title.len() < 3 {
                return Err(Error::TitleTooShort);
            }

            form.push(("title", title.as_str()));
        }

        if let Some(privacy) = data.privacy {
            form.push(("privacy", privacy.as_str()));
        }

        if let Some(nsfw) = data.nsfw {
            form.push(("nsfw", bool_to_str(nsfw)));
        }

        self.state.ratelimit().await;

        // Not using a multipart form here is intended.
        // Even though we use a multipart form for creating a post,
        // the server will silently ignore requests that aren't form-urlencoded.
        let response = self
            .client
            .patch(url)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .form(&form)
            .send()
            .await?;

        let post: ApiResponse<_> = response.error_for_status()?.json().await?;

        Ok(post.data)
    }

    /// Delete a post.
    ///
    /// # Authorization
    /// This function REQUIRES a token.
    pub async fn delete_post(&self, id: &str) -> Result<(), Error> {
        let token = self.get_token().ok_or(Error::MissingToken)?;
        let url = format!("{API_BASE}/v1/post/{id}");

        self.state.ratelimit().await;

        let response = self
            .client
            .delete(url)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .send()
            .await?;

        let response: ApiCompletedResponse = response.error_for_status()?.json().await?;
        if !response.success {
            return Err(Error::ApiOperationFailed);
        }

        Ok(())
    }

    /// Favorite or unfavorite a post.
    ///
    /// # Returns
    /// Returns true if the favorite was added.
    /// Returns false if the favorite was removed.
    ///
    /// # Authorization
    /// This function REQUIRES a token.
    pub async fn favorite_post(&self, id: &str) -> Result<bool, Error> {
        let token = self.get_token().ok_or(Error::MissingToken)?;
        let url = format!("{API_BASE}/v1/post/{id}/favorite");

        self.state.ratelimit().await;

        let response = self
            .client
            .post(url)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .send()
            .await?;

        let response: ApiCompletedResponse = response.error_for_status()?.json().await?;
        if !response.success {
            return Err(Error::ApiOperationFailed);
        }

        let message = response.message.ok_or(Error::ApiResponseMissingMessage)?;
        match &*message {
            "Favorite added." => Ok(true),
            "Favorite removed." => Ok(false),
            _ => Err(Error::ApiResponseUnknownMessage { message }),
        }
    }

    /// Add images to a post.
    ///
    /// # Authorization
    /// This function REQUIRES a token.
    pub async fn add_post_images<I>(&self, id: &str, images: I) -> Result<Post, Error>
    where
        I: IntoIterator<Item = UploadPostFile>,
    {
        let token = self.get_token().ok_or(Error::MissingToken)?;
        let url = format!("{API_BASE}/v1/post/{id}/add");

        let mut form = Form::new();

        let mut num_images = 0;
        for file in images {
            let part = reqwest::multipart::Part::stream(file.body).file_name(file.file_name);

            form = form.part("images[]", part);
            num_images += 1;
        }

        if num_images == 0 {
            return Err(Error::MissingImages);
        }

        self.state.ratelimit().await;

        let response = self
            .client
            .post(url)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .multipart(form)
            .send()
            .await?;

        let post: ApiResponse<_> = response.error_for_status()?.json().await?;

        Ok(post.data)
    }

    /// Get a user by username.
    ///
    /// # Authorization
    /// This function REQUIRES a token.
    pub async fn get_user(&self, username: &str) -> Result<User, Error> {
        let token = self.get_token().ok_or(Error::MissingToken)?;
        let url = format!("{API_BASE}/v1/user/{username}");

        self.state.ratelimit().await;

        let response = self
            .client
            .get(url)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .send()
            .await?;

        let user: ApiResponse<_> = response.error_for_status()?.json().await?;

        Ok(user.data)
    }

    /// Get a file by id.
    ///
    /// Currently, this is implemented according to the API spec,
    /// but the API will always return no data for some reason.
    /// It is likely that this endpoint is disabled.
    /// As a result, this function is currently useless.
    ///
    /// # Authorization
    /// This function REQUIRES a token.
    pub async fn get_file(&self, id: &str) -> Result<PostFile, Error> {
        let token = self.get_token().ok_or(Error::MissingToken)?;
        let url = format!("{API_BASE}/v1/file/{id}");

        self.state.ratelimit().await;

        let response = self
            .client
            .get(url)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .send()
            .await?;

        let file: ApiResponse<_> = response.error_for_status()?.json().await?;

        Ok(file.data)
    }

    /// Update a file.
    ///
    /// # Authorization
    /// This function REQUIRES a token.
    pub async fn update_file(&self, id: &str, description: &str) -> Result<(), Error> {
        let token = self.get_token().ok_or(Error::MissingToken)?;
        let url = format!("{API_BASE}/v1/file/{id}");

        if description.is_empty() {
            return Err(Error::MissingDescription);
        }

        self.state.ratelimit().await;

        let response = self
            .client
            .patch(url)
            .form(&[("description", description)])
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .send()
            .await?;

        let response: ApiCompletedResponse = response.error_for_status()?.json().await?;
        if !response.success {
            return Err(Error::ApiOperationFailed);
        }

        Ok(())
    }

    /// Delete a file.
    ///
    /// # Authorization
    /// This function REQUIRES a token.
    pub async fn delete_file(&self, id: &str) -> Result<(), Error> {
        let token = self.get_token().ok_or(Error::MissingToken)?;
        let url = format!("{API_BASE}/v1/file/{id}");

        self.state.ratelimit().await;

        let response = self
            .client
            .delete(url)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .send()
            .await?;

        let response: ApiCompletedResponse = response.error_for_status()?.json().await?;
        if !response.success {
            return Err(Error::ApiOperationFailed);
        }

        Ok(())
    }

    /// Update files in bulk.
    pub async fn update_files_bulk<I>(&self, files: I) -> Result<Vec<PostFile>, Error>
    where
        I: IntoIterator<Item = FileUpdate>,
    {
        let token = self.get_token().ok_or(Error::MissingToken)?;
        let url = format!("{API_BASE}/v1/files");

        let data = files
            .into_iter()
            .map(|file| {
                if file.description.is_empty() {
                    return Err(Error::MissingDescription);
                }
                Ok(file)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let data = ApiUpdateFilesBulkRequest { data };

        self.state.ratelimit().await;

        let response = self
            .client
            .patch(url)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .json(&data)
            .send()
            .await?;

        let file: ApiResponse<_> = response.error_for_status()?.json().await?;

        Ok(file.data)
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
struct ClientState {
    token: std::sync::RwLock<Option<Arc<str>>>,
    ratelimit_data: std::sync::Mutex<(Instant, u8)>,
}

impl ClientState {
    fn new() -> Self {
        let now = Instant::now();

        Self {
            token: std::sync::RwLock::new(None),
            ratelimit_data: std::sync::Mutex::new((now, REQUESTS_PER_MINUTE)),
        }
    }

    async fn ratelimit(&self) {
        loop {
            let sleep_duration = {
                let mut ratelimit_data = self
                    .ratelimit_data
                    .lock()
                    .expect("ratelimit mutex poisoned");
                let (ref mut last_refreshed, ref mut remaining_requests) = &mut *ratelimit_data;

                // Refresh the number of requests each minute.
                if last_refreshed.elapsed() >= ONE_MINUTE {
                    *last_refreshed = Instant::now();
                    *remaining_requests = REQUESTS_PER_MINUTE;
                }

                // If we are allowed to make a request now, make it.
                if *remaining_requests > 0 {
                    *remaining_requests -= 1;
                    return;
                }

                // Otherwise, sleep until the next refresh and try again.
                ONE_MINUTE.saturating_sub(last_refreshed.elapsed())
            };
            tokio::time::sleep(sleep_duration).await;
        }
    }
}

fn bool_to_str(b: bool) -> &'static str {
    if b {
        "true"
    } else {
        "false"
    }
}

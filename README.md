# imgchest-rs
A Rust library to interact with [imgchest.com](https://imgchest.com). 
It implements the entire API while also providing some scraping-based functionality without a login.

## Examples

### Create a Post
```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = imgchest::Client::new();
    client.set_token("TOKEN");

    let mut builder = imgchest::CreatePostBuilder::new();
    builder
        .title("test title")
        .nsfw(false)
        .image(imgchest::UploadPostFile::from_path("img.png").await?);
    let post = client.create_post(builder).await?;
    
    dbg!(&post);
    
    Ok(())
}
```

### Get a Post
```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = imgchest::Client::new();
    
    let post = client
        .get_post("3qe4gdvj4j2")
        .await?;
    
    dbg!(&post);
    
    Ok(())
}
```

### Scrape a Post
```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = imgchest::Client::new();
    
    let post = client
        .get_scraped_post("3qe4gdvj4j2")
        .await?;
    
    dbg!(&post);
    
    Ok(())
}
```

## Design
In the past, this library attempted to completely avoid official API usage.
This was due to the following 2 reasons:
1. The API required a login to access public data.
2. The API had obscene ratelimits, only 60 per hour.

While the ratelimits seem to have been updated to be tolerable (60 requests per minute),
fetching posts still requires a login.
As a result, API support has been added to this library while also attempting to provide scraping-based alternatives where possible.
It is suggested to use the scraping-based functionality when possible to avoid the need to use an API token and to avoid the ratelimit.

Scraped API objects in this library are tailored to match the official API's as much as possible, 
though some fields are missing.

### API Limitations
The API is limited in a few ways.
This library may gain more scraping-based functionality to work around these limitations.
These limitations are ordered by severity.
1. Missing post file reorder endpoint
2. Authentication for public data
3. Ratelimits

## References
 * https://imgchest.com/docs/api/1.0/general/overview
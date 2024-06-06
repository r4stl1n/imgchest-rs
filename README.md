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
        .get_scraped_post("https://imgchest.com/p/3qe4gdvj4j2")
        .await?;
    
    dbg!(&post);
    
    Ok(())
}
```

## Design
~~This library will attempt to completely avoid official API usage.~~
~~This is because of 2 reasons:~~
1. ~~The API requires a login to access public data.~~
2. ~~The API has obscene ratelimits, only 60 per hour.~~

~~As a result, a design based on scraping will be superior to one based on recommended API usage.~~
~~If the API is fixed, this library will be likely be reworked to target that instead.~~


This library implements the API while also attempting to provide scraping-based alternatives where possible.

(Scraped) API objects in this library are tailored to match the official API's as much as possible, 
though some fields are missing and extra fields are included where needed.
Usage of these objects is also noticablely more complicated.
As an example, the Post object may not load all images in one API call and may need a second call.

## References
 * https://imgchest.com/docs/api/1.0/general/overview
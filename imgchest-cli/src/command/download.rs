use anyhow::ensure;
use anyhow::Context;
use std::path::Path;
use std::path::PathBuf;
use tokio::task::JoinSet;
use url::Url;

#[derive(Debug, argh::FromArgs)]
#[argh(
    subcommand,
    name = "download",
    description = "download a post from imgchest.com"
)]
pub struct Options {
    #[argh(positional, description = "the url of the post")]
    pub url: String,

    #[argh(
        option,
        short = 'o',
        long = "out-dir",
        default = "PathBuf::from(\".\")",
        description = "the directory to download to"
    )]
    pub out_dir: PathBuf,
}

pub async fn exec(client: imgchest::Client, options: Options) -> anyhow::Result<()> {
    let id = extract_id(options.url.as_str()).context("failed to determine post id")?;

    let post = client
        .get_scraped_post(&id)
        .await
        .context("failed to get post")?;

    let out_dir = options.out_dir.join(&*post.id);

    tokio::fs::create_dir_all(&out_dir)
        .await
        .context("failed to create out dir")?;

    let post_json = serde_json::to_string(&post)?;
    tokio::fs::write(out_dir.join("post.json"), &post_json).await?;

    let mut join_set = JoinSet::new();
    let total_downloads = post.image_count;
    for image in post.images.iter() {
        spawn_image_download(&client, &mut join_set, image, &out_dir);
    }

    let mut last_error = Ok(());
    let mut downloaded = 0;
    while let Some(result) = join_set.join_next().await {
        match result
            .context("failed to join tokio task")
            .and_then(|result| result)
        {
            Ok(_new_download) => {
                downloaded += 1;
                println!("{downloaded}/{total_downloads}...");
            }
            Err(error) => {
                eprintln!("{error:?}");
                last_error = Err(error);
            }
        }
    }

    last_error
}

fn extract_id(value: &str) -> anyhow::Result<String> {
    match Url::parse(value) {
        Ok(url) => {
            // Ensure the url is in the format:
            // https://imgchest.com/p/{id}
            ensure!(url.host_str() == Some("imgchest.com"));
            let mut path_iter = url.path_segments().context("url is missing path")?;
            ensure!(path_iter.next() == Some("p"));
            let id = path_iter.next().context("url missing id path segment")?;

            Ok(id.to_string())
        }
        Err(_error) => {
            // This isn't a url, but it might be a raw id.
            let is_valid_id = is_valid_id(value);
            ensure!(
                is_valid_id,
                "ids must be composed of 11 ascii alphanumeric characters"
            );
            Ok(value.to_string())
        }
    }
}

/// Ids are composed of 11 lowercase alphanumeric chars.
fn is_valid_id(value: &str) -> bool {
    value.len() == 11 && value.chars().all(is_ascii_alphanumeric_lowercase)
}

fn is_ascii_alphanumeric_lowercase(ch: char) -> bool {
    ch.is_ascii_digit() | ch.is_ascii_lowercase()
}

fn spawn_image_download(
    client: &imgchest::Client,
    join_set: &mut JoinSet<anyhow::Result<bool>>,
    file: &imgchest::ScrapedPostFile,
    out_dir: &Path,
) {
    let client = client.clone();
    let link = file.link.clone();
    let out_path_result = file
        .link
        .split('/')
        .next_back()
        .context("missing file name")
        .map(|file_name| out_dir.join(file_name));
    join_set.spawn(async move {
        let out_path = out_path_result?;
        if tokio::fs::try_exists(&out_path)
            .await
            .context("failed to check if file exists")?
        {
            return Ok(false);
        }

        nd_util::download_to_path(&client.client, &link, &out_path).await?;

        Ok(true)
    });
}

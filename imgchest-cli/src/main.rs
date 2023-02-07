use anyhow::ensure;
use anyhow::Context;
use std::path::Path;
use std::path::PathBuf;
use tokio::task::JoinSet;

#[derive(Debug, argh::FromArgs)]
#[argh(description = "a cli to interact with imgchest.com")]
struct Options {
    #[argh(subcommand)]
    subcommand: Subcommand,
}

#[derive(Debug, argh::FromArgs)]
#[argh(subcommand)]
enum Subcommand {
    Download(DownloadOptions),
}

#[derive(Debug, argh::FromArgs)]
#[argh(
    subcommand,
    name = "download",
    description = "download a post from imgchest.com"
)]
struct DownloadOptions {
    #[argh(positional, description = "the url of the post")]
    url: String,

    #[argh(
        option,
        short = 'o',
        long = "out-dir",
        default = "PathBuf::from(\".\")",
        description = "the directory to download to"
    )]
    out_dir: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let options = argh::from_env();
    let tokio_rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    tokio_rt.block_on(async_main(options))
}

async fn async_main(options: Options) -> anyhow::Result<()> {
    let client = imgchest::Client::new();

    match options.subcommand {
        Subcommand::Download(options) => {
            let post = client
                .get_post(options.url.as_str())
                .await
                .context("failed to get post")?;

            let out_dir = options.out_dir.join(&post.id);

            tokio::fs::create_dir_all(&out_dir)
                .await
                .context("failed to create out dir")?;

            let post_json = serde_json::to_string(&post)?;
            tokio::fs::write(out_dir.join("post.json"), &post_json).await?;

            let mut join_set = JoinSet::new();
            let mut total_downloads = post.image_count;
            for image in post.images.iter() {
                if image.video_link.is_some() {
                    total_downloads += 1;
                }
                spawn_image_download(&client, &mut join_set, image, &out_dir);
            }

            if let Some(extra_image_count) = post.extra_image_count {
                let extra_images = client
                    .load_extra_images_for_post(&post)
                    .await
                    .context("failed to load extra for post")?;

                ensure!(extra_images.len() == usize::try_from(extra_image_count)?);

                for image in extra_images.iter() {
                    if image.video_link.is_some() {
                        total_downloads += 1;
                    }
                    spawn_image_download(&client, &mut join_set, image, &out_dir);
                }
            }

            let mut last_error = None;
            let mut downloaded = 0;
            while let Some(result) = join_set.join_next().await {
                match result
                    .context("failed to join tokio task")
                    .and_then(|result| result)
                {
                    Ok(_new_download) => {
                        downloaded += 1;
                        println!("{downloaded} / {total_downloads}...");
                    }
                    Err(e) => {
                        last_error = Some(e);
                    }
                }
            }

            if let Some(e) = last_error {
                return Err(e);
            }
        }
    }

    Ok(())
}

fn spawn_image_download(
    client: &imgchest::Client,
    join_set: &mut JoinSet<anyhow::Result<bool>>,
    image: &imgchest::PostImage,
    out_dir: &Path,
) {
    {
        let client = client.clone();
        let link = image.link.clone();
        let out_path_result = image
            .link
            .split('/')
            .rev()
            .next()
            .context("missing file name")
            .map(|file_name| out_dir.join(file_name));
        join_set.spawn(async move {
            let out_path = out_path_result?;
            match tokio::fs::metadata(&out_path).await {
                Ok(_metadata) => {
                    return Ok(false);
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => {
                    return Err(e).context("failed to check if file exists");
                }
            }

            nd_util::download_to_path(&client.client, &link, &out_path).await?;

            Ok(true)
        });
    }

    if let Some(video_link) = image.video_link.clone() {
        let client = client.clone();
        let out_path_result = video_link
            .split('/')
            .rev()
            .next()
            .context("missing file name")
            .map(|file_name| out_dir.join(file_name));
        join_set.spawn(async move {
            let out_path = out_path_result?;
            match tokio::fs::metadata(&out_path).await {
                Ok(_metadata) => {
                    return Ok(false);
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => {
                    return Err(e).context("failed to check if file exists");
                }
            }

            nd_util::download_to_path(&client.client, &video_link, &out_path).await?;

            Ok(true)
        });
    }
}

mod command;

#[derive(Debug, argh::FromArgs)]
#[argh(description = "a cli to interact with imgchest.com")]
struct Options {
    #[argh(subcommand)]
    subcommand: Subcommand,
}

#[derive(Debug, argh::FromArgs)]
#[argh(subcommand)]
enum Subcommand {
    Download(self::command::download::Options),
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
        Subcommand::Download(options) => self::command::download::exec(client, options).await?,
    }

    Ok(())
}

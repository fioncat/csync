use anyhow::Result;
use async_trait::async_trait;
use clap::Args;
use serde::{de::DeserializeOwned, Serialize};

use crate::client::factory::ClientFactory;
use crate::client::Client;
use crate::display::{display_json, display_list, DisplayStyle, TerminalDisplay};
use crate::types::file::FileInfo;
use crate::types::image::Image;
use crate::types::text::Text;
use crate::types::user::{Role, User};

use super::{ConfigArgs, QueryArgs, ResourceType, RunCommand};

/// Retrieve resource information from the server and print it. This command can be used to
/// display a single or multiple resources
#[derive(Args)]
pub struct GetArgs {
    /// Type of resource to display
    pub resource: ResourceType,

    /// Optional filter by ID; if provided, it will return the unique resource with that ID,
    /// otherwise, it will return multiple resources.
    pub id: Option<String>,

    /// The display style.
    #[arg(short, long, default_value = "table")]
    pub output: DisplayStyle,

    /// When displaying in CSV format, do not show the header row.
    #[arg(long)]
    pub headless: bool,

    /// When displaying in CSV format, manually specify the rows to display.
    #[arg(long)]
    pub csv_titles: Option<String>,

    #[command(flatten)]
    pub query: QueryArgs,

    #[command(flatten)]
    pub config: ConfigArgs,
}

#[async_trait]
impl RunCommand for GetArgs {
    async fn run(&self) -> Result<()> {
        let ps = self.config.build_path_set()?;

        let client_factory = ClientFactory::load(&ps)?;
        let client = client_factory.build_client_with_token_file().await?;

        match self.resource {
            ResourceType::User | ResourceType::Users => self.get_resource::<User>(client).await,
            ResourceType::Role | ResourceType::Roles => self.get_resource::<Role>(client).await,
            ResourceType::Text | ResourceType::Texts => self.get_resource::<Text>(client).await,
            ResourceType::Image | ResourceType::Images => self.get_resource::<Image>(client).await,
            ResourceType::File | ResourceType::Files => self.get_resource::<FileInfo>(client).await,
        }
    }
}

impl GetArgs {
    async fn get_resource<T>(&self, client: Client) -> Result<()>
    where
        T: Serialize + DeserializeOwned + TerminalDisplay,
    {
        let resource_name = self.resource.get_name();
        match self.id {
            Some(ref name) => {
                let resp: T = client.get_resource(resource_name, name.clone()).await?;
                if matches!(self.output, DisplayStyle::Json) {
                    return display_json(resp);
                }
                display_list(
                    vec![resp],
                    self.output,
                    self.headless,
                    self.csv_titles.clone(),
                )
            }
            None => {
                let resp: Vec<T> = client
                    .list_resources(resource_name, self.query.build_query()?)
                    .await?;
                display_list(resp, self.output, self.headless, self.csv_titles.clone())
            }
        }
    }
}

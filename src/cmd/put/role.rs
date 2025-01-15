use std::collections::HashSet;

use anyhow::{bail, Result};
use async_trait::async_trait;
use clap::Args;

use crate::client::factory::ClientFactory;
use crate::cmd::{ConfigArgs, RunCommand};
use crate::types::user::{Role, RoleRule};

/// Create or update a role.
#[derive(Args)]
pub struct RoleArgs {
    /// Role name, which must be unique on the server.
    pub name: String,

    /// The operation rules for the role, formatted as "resources:verb", and there can be
    /// multiple rules. This indicates that the role can perform a certain operation on a
    /// resource. Resources or operations can be specified as "*", meaning any resource or
    /// any operation. Optional operations include "put, get, list, delete". If the role does
    /// not exist, it will be created with these rules; if the role exists, its rules will
    /// be updated.
    #[arg(short, long)]
    pub rules: Vec<String>,

    #[command(flatten)]
    pub config: ConfigArgs,
}

#[async_trait]
impl RunCommand for RoleArgs {
    async fn run(&self) -> Result<()> {
        let ps = self.config.build_path_set()?;

        let client_factory = ClientFactory::load(&ps)?;
        let client = client_factory.build_client_with_token_file().await?;

        let rules = self.parse_rules()?;

        let role = Role {
            name: self.name.clone(),
            rules,
            create_time: 0,
            update_time: 0,
        };
        client.put_role(&role).await?;

        Ok(())
    }
}

impl RoleArgs {
    fn parse_rules(&self) -> Result<Vec<RoleRule>> {
        let mut rules = vec![];
        for rule_str in self.rules.iter() {
            let parts: Vec<&str> = rule_str.split(':').collect();
            if parts.len() != 2 {
                bail!(
                    "invalid rule: '{}', the format is: '<resources>:<verbs>'",
                    rule_str
                );
            }

            let resources: HashSet<String> = parts[0].split(',').map(|r| r.to_string()).collect();
            let verbs: HashSet<String> = parts[1].split(',').map(|r| r.to_string()).collect();

            if resources.is_empty() {
                bail!("empty resources in rule: '{}'", rule_str);
            }
            if verbs.is_empty() {
                bail!("empty verbs in rule: '{}'", rule_str);
            }

            let rule = RoleRule { resources, verbs };
            rules.push(rule);
        }
        Ok(rules)
    }
}

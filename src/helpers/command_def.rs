use serenity::builder::CreateApplicationCommand;
use serenity::futures::future::BoxFuture;
use serenity::model::interactions::application_command::ApplicationCommandInteraction;
use serenity::model::prelude::*;
use serenity::prelude::*;

// Inspired by:
// https://github.com/rcos/Telescope/blob/c541baee882087e0920b4a7aff7477c8af2b1622/src/discord_bot/commands/mod.rs

pub type InteractionResult = BoxFuture<'static, anyhow::Result<()>>;
pub type CommandBuilder = fn(&mut CreateApplicationCommand) -> &mut CreateApplicationCommand;
pub type InteractionHandler = fn(Context, ApplicationCommandInteraction) -> InteractionResult;

pub struct CommandDef {
    pub name: &'static str,
    pub builder: CommandBuilder,
    pub handler: InteractionHandler,
    pub re_register: bool,
    pub whitelisted_servers: Option<&'static [GuildId]>,
}

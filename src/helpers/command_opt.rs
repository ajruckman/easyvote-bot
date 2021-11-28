use serenity::client::Context;
use serenity::model::id::UserId;
use serenity::model::interactions::application_command::{ApplicationCommandInteraction, ApplicationCommandInteractionDataOption};
use serenity::model::interactions::application_command::ApplicationCommandInteractionDataOptionValue;

use crate::helpers::command_resp;

pub async fn find_required<T, F>(ctx: &Context, interaction: &ApplicationCommandInteraction, opts: &[ApplicationCommandInteractionDataOption], getter: F, name: &str) -> anyhow::Result<Option<T>>
    where F: Fn(&[ApplicationCommandInteractionDataOption], &str) -> Option<T>
{
    let v = getter(opts, name);
    match v {
        None => {
            command_resp::reply(ctx, &interaction, |x| x.content(format!("{} is required", name))).await?;
            Ok(None)
        }
        Some(v) => Ok(Some(v))
    }
}

#[must_use]
pub fn find_string_opt(opts: &[ApplicationCommandInteractionDataOption], name: &str) -> Option<String> {
    let i = opts.iter().find(|v| v.name == name)?;

    match &i.resolved {
        Some(ApplicationCommandInteractionDataOptionValue::String(x)) => Some(x.clone()),
        _ => None,
    }
}

pub struct FindUserOptResult {
    pub id: UserId,
    pub id_string: String,
    pub name: String,
    pub nick: Option<String>,
}

impl FindUserOptResult {
    #[must_use]
    pub fn display_name(&self) -> &String {
        match &self.nick {
            None => &self.name,
            Some(n) => n,
        }
    }
}

#[must_use]
pub fn find_user_opt(opts: &[ApplicationCommandInteractionDataOption], name: &str) -> Option<FindUserOptResult> {
    let opt = opts.iter().find(|v| v.name == name)?;

    let user_id;
    let user_name;
    let mut user_nick = None;

    match &opt.resolved {
        Some(ApplicationCommandInteractionDataOptionValue::User(u, p)) => {
            user_id = u.id;
            user_name = u.name.clone();
            if p.is_some() { user_nick = p.as_ref().unwrap().nick.clone(); }
        }
        _ => return None,
    }

    return Some(FindUserOptResult {
        id: user_id,
        id_string: user_id.as_u64().to_string(),
        name: user_name,
        nick: user_nick,
    });
}

#[must_use]
pub fn find_integer_opt(opts: &[ApplicationCommandInteractionDataOption], name: &str) -> Option<i64> {
    let i = opts.iter().find(|v| v.name == name)?;

    match &i.resolved {
        Some(ApplicationCommandInteractionDataOptionValue::Integer(x)) => Some(*x),
        _ => None,
    }
}

#[must_use]
pub fn find_boolean_opt(opts: &[ApplicationCommandInteractionDataOption], name: &str) -> Option<bool> {
    let i = opts.iter().find(|v| v.name == name)?;

    match &i.resolved {
        Some(ApplicationCommandInteractionDataOptionValue::Boolean(x)) => Some(*x),
        _ => None,
    }
}

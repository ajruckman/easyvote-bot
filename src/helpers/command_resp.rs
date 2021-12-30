use serenity::builder::CreateInteractionResponseData;
use serenity::client::Context;
use serenity::model::id::MessageId;
use serenity::model::interactions::InteractionResponseType;
use serenity::model::interactions::application_command::ApplicationCommandInteraction;

pub async fn reply<T>(ctx: &Context, interaction: &ApplicationCommandInteraction, content: T) -> anyhow::Result<()>
    where T: FnOnce(&mut CreateInteractionResponseData) -> &mut CreateInteractionResponseData,
{
    interaction.create_interaction_response(&ctx, |response| {
        response.kind(InteractionResponseType::ChannelMessageWithSource);
        response.interaction_response_data(content);
        response
    }).await?;

    Ok(())
}

pub async fn edit_reply<D>(ctx: &Context, interaction: &ApplicationCommandInteraction, text: D) -> anyhow::Result<()>
    where D: ToString {
    interaction.edit_original_interaction_response(&ctx, |response| {
        response.content(text);
        response
    }).await?;

    Ok(())
}

pub async fn reply_deferred_ack(ctx: &Context, interaction: &ApplicationCommandInteraction) -> anyhow::Result<()> {
    interaction.create_interaction_response(&ctx, |response| {
        response.kind(InteractionResponseType::DeferredChannelMessageWithSource);
        response
    }).await?;

    Ok(())
}

pub async fn reply_deferred_result<D>(ctx: &Context, interaction: &ApplicationCommandInteraction, text: D) -> anyhow::Result<MessageId>
    where D: ToString {
    let resp = interaction.edit_original_interaction_response(&ctx, |response| {
        response.content(text);
        response
    }).await?;

    Ok(resp.id)
}

pub async fn edit_reply_deferred_result<D>(ctx: &Context, interaction: &ApplicationCommandInteraction, id: MessageId, text: D) -> anyhow::Result<()>
    where D: ToString {
    interaction.edit_followup_message(&ctx, id, |response| {
        response.content(text);
        response
    }).await?;

    Ok(())
}

pub async fn delete_deferred_result(ctx: &Context, interaction: &ApplicationCommandInteraction) -> anyhow::Result<()> {
    interaction.delete_original_interaction_response(&ctx).await?;
    Ok(())
}

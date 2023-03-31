mod commands;
mod data;

use serenity::async_trait;
use serenity::client::bridge::gateway::ShardManager;
use serenity::framework::standard::macros::{group, hook};
use serenity::framework::standard::CommandResult;
use serenity::framework::StandardFramework;
use serenity::http::Http;
use serenity::model::event::ResumedEvent;
use serenity::model::{
    channel::{Message, Reaction, ReactionType},
    gateway::Ready,
    id::{MessageId, RoleId},
};
use serenity::prelude::*;
use std::collections::HashSet;
use std::sync::Arc;
use std::{
    convert::TryFrom,
    env, fs,
    sync::atomic::{AtomicU64, Ordering},
};
use tracing::{error, info};

use crate::commands::floor::*;
use crate::commands::meta::*;
use crate::commands::reactionroles::*;
use crate::commands::replycommands::*;
use crate::data::{config::Config, messagemap::MessageMap, reactionmap::ReactionMap};

pub struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        println!("Connected as {}", ready.user.name);
    }

    async fn resume(&self, _: Context, _: ResumedEvent) {
        info!("Resumed");
    }
    async fn reaction_add(&self, ctx: Context, reaction: Reaction) {
        handle_reaction(ctx, reaction, true).await;
    }
    async fn reaction_remove(&self, ctx: Context, reaction: Reaction) {
        handle_reaction(ctx, reaction, false).await;
    }
}

async fn handle_reaction(ctx: Context, reaction: Reaction, add_role: bool) {
    let data_read = ctx.data.read().await;
    let message_data = data_read
        .get::<MessageMap>()
        .expect("Expected MessageMap in TypeMap.")
        .clone();
    if reaction.message_id != MessageId(message_data.load(Ordering::SeqCst)) {
        return;
    }
    let reaction_roles_data = data_read
        .get::<ReactionMap>()
        .expect("Expected ReactionMap in TypeMap.")
        .clone();

    let reaction_roles = &*reaction_roles_data.read().await;
    for (emoji, role_id) in reaction_roles.into_iter() {
        if emoji != &reaction.emoji {
            continue;
        }

        if let Some(guild_id) = reaction.guild_id {
            if let Some(user_id) = reaction.user_id {
                if let Ok(mut member) = guild_id.member(&ctx, user_id).await {
                    if add_role {
                        if let Err(err) = member.add_role(&ctx, role_id).await {
                            error!("Role could not be added: {}", err);
                        }
                        info!(
                            "Role {} added to user {} by reacting with {}.",
                            role_id, member, emoji
                        )
                    } else {
                        if let Err(err) = member.remove_role(&ctx, role_id).await {
                            error!("Role could not be removed: {}", err);
                        }
                        info!(
                            "Role {} removed from user {} by un-reacting with {}.",
                            role_id, member, emoji
                        )
                    }
                }
            }
        }
    }
}
#[group]
#[commands(
    ping,
    izee,
    josh,
    swypes,
    zyo,
    ziz,
    flipcreed,
    absinthe,
    zilbag,
    ilv,
    gm,
    gn,
    panels,
    reactionroles,
    floor
)]
struct General;

#[hook]
async fn unknown_command(_ctx: &Context, _msg: &Message, unknown_command_name: &str) {
    println!("Could not find command named '{}'", unknown_command_name);
}

#[hook]
async fn after(_ctx: &Context, _msg: &Message, command_name: &str, command_result: CommandResult) {
    match command_result {
        Ok(()) => println!("Processed command '{}'", command_name),
        Err(why) => println!("Command '{}' returned error {:?}", command_name, why),
    }
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().expect("Failed to load .env file");
    // `RUST_LOG` to `debug`.
    tracing_subscriber::fmt::init();

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let http = Http::new(&token);

    let config_raw = fs::read_to_string(env::current_dir().unwrap().join("config.json"))
        .expect("Unable to read config");
    let config: Config = serde_json::from_str(&config_raw).unwrap();

    // We will fetch your bot's owners and id
    let (owners, _bot_id) = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();
            owners.insert(info.owner.id);

            (owners, info.id)
        }
        Err(why) => panic!("Could not access application info: {:?}", why),
    };

    // Create the framework
    let framework = StandardFramework::new()
        .configure(|c| c.owners(owners).prefix("-"))
        .group(&GENERAL_GROUP)
        .unrecognised_command(unknown_command)
        .after(after);

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_MESSAGE_REACTIONS;
    let mut client = Client::builder(&token, intents)
        .framework(framework)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    {
        let mut data = client.data.write().await;
        data.insert::<ShardManagerContainer>(client.shard_manager.clone());
        let mut reaction_roles = vec![];

        for index in 0..config.emotes.len() {
            reaction_roles.push((
                ReactionType::try_from(config.emotes[index].as_str()).unwrap(),
                RoleId(config.role_ids[index]),
            ));
        }
        data.insert::<MessageMap>(Arc::new(AtomicU64::new(config.message_id)));
        data.insert::<ReactionMap>(Arc::new(RwLock::new(reaction_roles)));
    }

    let shard_manager = client.shard_manager.clone();

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Could not register ctrl+c handler");
        shard_manager.lock().await.shutdown_all().await;
    });

    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }
}

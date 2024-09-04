use http::Uri;

use futures_util::{SinkExt, StreamExt};

use serenity::all::ChannelId;
use serenity::async_trait;
use serenity::model::{channel, gateway::Ready};
use serenity::prelude::*;

use std::env;

use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;

use tokio_websockets::{ClientBuilder, Error, Error as WebSocketError, Message, ServerBuilder};

struct GlobalChannelId;

impl TypeMapKey for GlobalChannelId {
    type Value = Option<ChannelId>;
}

struct Listener;

impl TypeMapKey for Listener {
    type Value = Option<TcpListener>;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: channel::Message) {
        let message_parts: Vec<&str> = msg.content.split_whitespace().collect();
        match message_parts[0] {
            "!start" => {
                {
                    let mut data = ctx.data.write().await;
                    let global_channel_id = data.get_mut::<GlobalChannelId>().unwrap();
                    if global_channel_id.is_none() {
                        *global_channel_id = Some(msg.channel_id);
                    }
                }

                if let Err(e) = msg
                    .channel_id
                    .say(
                        &ctx.http,
                        format!(
                            "Started websocket server!\nReceived messages will be posted in #{}",
                            msg.channel_id.name(&ctx.http).await.unwrap()
                        )
                        .as_str(),
                    )
                    .await
                {
                    eprintln!("{}", e);
                }

                let (tx, _) = broadcast::channel(50);
                {
                    let mut data = ctx.data.write().await;
                    let listener = data.get_mut::<Listener>().unwrap().take().unwrap();

                    // listen for connections and open websockets.
                    tokio::spawn(async move {
                        while let Ok((stream, _)) = listener.accept().await {
                            tokio::spawn(accept_connection(stream, tx.clone()));
                        }
                    });
                }

                tokio::spawn(send_discord_messages(ctx));
            }
            _ => return,
        };
    }
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

async fn send_discord_messages(ctx: Context) {
    let uri = Uri::from_static("ws://127.0.0.1:3000");
    let (mut client, _) = ClientBuilder::from_uri(uri).connect().await.unwrap();
    println!("Created client");

    // Receive the text from the server
    loop {
        if let Some(Ok(msg)) = client.next().await {
            let data = ctx.data.read().await;
            if let Some(id) = data.get::<GlobalChannelId>().unwrap() {
                let _ = id.say(&ctx.http, msg.as_text().unwrap()).await;
            }
        }
    }
}

async fn accept_connection(stream: TcpStream, tx: broadcast::Sender<String>) {
    if let Err(e) = handle_connection(stream, tx).await {
        match e {
            Error::Protocol(_) => (),
            err => eprintln!("Error processing connection: {err:?}"),
        }
    }
}

async fn handle_connection(stream: TcpStream, tx: broadcast::Sender<String>) -> Result<(), Error> {
    let mut rx = tx.subscribe();
    let mut ws_stream = ServerBuilder::new().accept(stream).await?;

    loop {
        tokio::select! {
            incoming = ws_stream.next() => {
                if let Some(msg) = incoming {
                    if let Some(text) = msg?.as_text() {
                        tx.send(text.into()).unwrap();
                    }
                } else {
                    break;
                }
            }
            msg = rx.recv() => {
                let msg = match msg {
                    Ok(msg) => msg,
                    Err(_) => break,
                };

                ws_stream.send(Message::text(msg)).await?;
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), WebSocketError> {
    let listener = TcpListener::bind("127.0.0.1:3000").await?;
    // Initialize discord bot
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut discord_client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Err creating client");
    {
        let mut data = discord_client.data.write().await;
        data.insert::<GlobalChannelId>(None);
        data.insert::<Listener>(Some(listener));
    }

    if let Err(e) = discord_client.start().await {
        println!("Client error: {e:?}");
    };

    Ok(())
}

use worker::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Debug, Clone, Deserialize)]
pub struct Expansion {
    short_code: String,
    status_code: u32,
}

#[event(fetch, respond_with_errors)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    let router = Router::new();

    router
        .get_async("/:short_code", |_req, ctx| async move {
            if let Some(short_code) = ctx.param("short_code") {
                let urls = ctx.kv("URLS")?;
                let queue_a = ctx.env.queue("QUEUE_A")?;
                return match urls.get(short_code).text().await? {
                    Some(full_url) => {
                        queue_a.send(&Expansion {
                            short_code: short_code.to_string(),
                            status_code: 200,
                        }).await?;
                        Response::redirect(Url::parse(full_url.as_str())?)
                    }
                    None => {
                        queue_a.send(&Expansion {
                            short_code: short_code.to_string(),
                            status_code: 404,
                        }).await?;
                        Response::error("Not found", 404)
                    }
                };
            }

            Response::error("Bad Request", 400)
        })
        .run(req, env)
        .await
}

// Consume messages from a queue
#[event(queue)]
pub async fn main(message_batch: MessageBatch<Expansion>, env: Env, _ctx: Context) -> Result<()> {
    // Deserialize the message batch
    let messages = message_batch.messages()?;

    // Loop through the messages
    for message in messages {
        // Log the message and meta data
        console_log!(
            "Got message {:?}, with id {} and timestamp: {}",
            message.body,
            message.id,
            message.timestamp.to_string()
        );

        // Get a queue with the binding 'queue_b'
        let queue_b = env.queue("QUEUE_B")?;

        // Send the message body to the other queue
        queue_b.send(&message.body).await?;
    }

    // Retry all messages
    message_batch.retry_all();
    Ok(())
}

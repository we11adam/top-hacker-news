mod hn;
mod telegram;

use hn::HackerNewsClient;
use telegram::TelegramBot;
use worker::*;

const TOP_STORY: usize = 30;
const MIN_SCORE: u64 = 50;
const MIN_COMMENTS: u64 = 5;

#[event(fetch)]
async fn fetch(_req: Request, env: Env, _ctx: Context) -> Result<Response> {
    match handle_push(env).await {
        Ok(msg) => Response::ok(msg),
        Err(e) => {
            console_error!("Push failed: {:?}", e);
            Response::error(format!("{:?}", e), 500)
        }
    }
}

#[event(scheduled)]
async fn scheduled(_: ScheduledEvent, env: Env, _: ScheduleContext) {
    if let Err(e) = handle_push(env).await {
        console_error!("Scheduled push failed: {:?}", e);
    }
}

async fn handle_push(env: Env) -> Result<String> {
    let kv = env.kv("thn")?;
    let bot_token = env.secret("TELEGRAM_BOT_TOKEN")?.to_string();
    let chat_id = env.secret("TELEGRAM_CHAT_ID")?.to_string();

    let hn_client = HackerNewsClient;
    let telegram = TelegramBot::new(&bot_token, &chat_id);

    let story_ids = hn_client.get_top_stories(TOP_STORY).await?;
    console_log!("Fetched {} top stories from HN", story_ids.len());

    let mut pushed_count = 0;

    for story_id in story_ids {
        let key = format!("story:{}", story_id);
        let already_pushed = kv.get(&key).text().await?;

        if already_pushed.is_some() {
            continue;
        }

        match hn_client.get_story(story_id).await {
            Ok(story) => {
                if story.score < MIN_SCORE {
                    console_log!("Skipping story {} due to low score: {}", story_id, story.score);
                    continue;
                }

                if story.descendants < MIN_COMMENTS {
                    console_log!("Skipping story {} due to low comments: {}", story_id, story.descendants);
                    continue;
                }

                if let Err(e) = telegram.send_story_message(&story).await {
                    console_error!("Failed to send story {}: {:?}", story_id, e);
                } else {
                    let now = chrono::Utc::now().to_rfc3339();
                    kv.put(&key, &now)?.execute().await?;
                    pushed_count += 1;
                }
            }
            Err(e) => console_error!("Failed to fetch story {}: {:?}", story_id, e),
        }
    }

    console_log!("Successfully pushed {} stories", pushed_count);
    Ok(format!("Pushed {} stories", pushed_count))
}

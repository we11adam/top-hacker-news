mod hn;
mod telegram;

use futures::future::join_all;
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

    // Check which stories have already been pushed (parallel KV check)
    let kv_check_futures: Vec<_> = story_ids
        .iter()
        .map(|&story_id| {
            let kv = &kv;
            async move {
                let key = format!("story:{}", story_id);
                match kv.get(&key).text().await {
                    Ok(Some(_)) => None,
                    Ok(None) => Some(story_id),
                    Err(e) => {
                        console_error!("Failed to check KV for story {}: {:?}", story_id, e);
                        None
                    }
                }
            }
        })
        .collect();

    let new_story_ids: Vec<_> = join_all(kv_check_futures)
        .await
        .into_iter()
        .flatten()
        .collect();

    console_log!("Found {} new stories to process", new_story_ids.len());

    // Fetch all stories in parallel
    let fetch_futures: Vec<_> = new_story_ids
        .iter()
        .map(|&story_id| {
            let hn_client = &hn_client;
            async move {
                match hn_client.get_story(story_id).await {
                    Ok(story) => Some((story_id, story)),
                    Err(e) => {
                        console_error!("Failed to fetch story {}: {:?}", story_id, e);
                        None
                    }
                }
            }
        })
        .collect();

    let stories: Vec<_> = join_all(fetch_futures)
        .await
        .into_iter()
        .flatten()
        .filter(|(_, story)| story.score >= MIN_SCORE && story.descendants >= MIN_COMMENTS)
        .collect();

    console_log!("Fetched {} stories meeting criteria", stories.len());

    // Send messages and mark as pushed in parallel
    let push_futures: Vec<_> = stories
        .iter()
        .map(|(story_id, story)| {
            let telegram = &telegram;
            let kv = &kv;
            async move {
                if let Err(e) = telegram.send_story_message(story).await {
                    console_error!("Failed to send story {}: {:?}", story_id, e);
                    return 0;
                }

                let key = format!("story:{}", story_id);
                let now = chrono::Utc::now().to_rfc3339();
                match kv.put(&key, &now) {
                    Ok(put_builder) => {
                        if let Err(e) = put_builder.execute().await {
                            console_error!(
                                "Failed to execute KV put for story {}: {:?}",
                                story_id,
                                e
                            );
                            return 0;
                        }
                    }
                    Err(e) => {
                        console_error!("Failed to create KV put for story {}: {:?}", story_id, e);
                        return 0;
                    }
                }

                1
            }
        })
        .collect();

    let pushed_count: usize = join_all(push_futures).await.iter().sum();

    console_log!("Successfully pushed {} stories", pushed_count);
    Ok(format!("Pushed {} stories", pushed_count))
}

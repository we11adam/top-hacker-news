use crate::hn::Story;
use crate::wasm_bindgen;
use worker::{Error, Fetch, Request, RequestInit, Result, Url, console_error, console_log};

static FIRE_SUFFIX: &str = "🔥";

pub struct TelegramBot {
    bot_token: String,
    chat_id: String,
}

impl TelegramBot {
    const API_BASE: &'static str = "https://api.telegram.org/bot";

    pub fn new(bot_token: &str, chat_id: &str) -> Self {
        Self {
            bot_token: bot_token.to_string(),
            chat_id: chat_id.to_string(),
        }
    }

    fn escape_html(text: &str) -> String {
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
    }

    fn format_story_message(&self, story: &Story) -> String {
        let title = Self::escape_html(&story.title);
        let url = story.url.clone().unwrap_or_default();
        format!("<b>{}</b> \n{}", title, url)
    }

    pub async fn send_story_message(&self, story: &Story) -> Result<()> {
        let url = Url::parse(&format!("{}{}/sendMessage", Self::API_BASE, self.bot_token))?;

        let mut score_str = format!("Score: {}+", story.score);
        if story.score >= 100 {
            score_str.push_str(FIRE_SUFFIX);
        }
        let mut comments_str = format!("Comments: {}+", story.descendants);
        if story.descendants >= 100 {
            comments_str.push_str(FIRE_SUFFIX);
        }
        let keyboard = serde_json::json!([
            [
                {
                    "text": score_str,
                    "url": story.link_url()
                },
                {
                    "text":comments_str,
                    "url": story.comments_url()
                }
            ]
        ]);

        let body = serde_json::json!({
            "chat_id": self.chat_id,
            "text": self.format_story_message(story),
            "parse_mode": "HTML",
            "disable_web_page_preview": false,
            "reply_markup": {
                "inline_keyboard": keyboard
            }
        });

        let mut init = RequestInit::new();
        init.with_method(worker::Method::Post);
        init.with_body(Some(wasm_bindgen::JsValue::from_str(&body.to_string())));

        let headers = worker::Headers::new();
        headers.set("Content-Type", "application/json")?;
        init.with_headers(headers);

        let req = Request::new_with_init(url.as_ref(), &init)?;
        let mut resp = Fetch::Request(req).send().await?;

        let status = resp.status_code();
        let text = resp.text().await?;

        if status != 200 {
            console_error!("Telegram API error (status {}): {}", status, text);
            return Err(Error::RustError(format!(
                "Telegram API error: status {}, response: {}",
                status, text
            )));
        }

        console_log!("Sent story {} to Telegram", story.id);
        Ok(())
    }
}

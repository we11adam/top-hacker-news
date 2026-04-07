# Top Hacker News Telegram Bot

A Cloudflare Worker written in Rust that fetches the top 30 stories from Hacker News and pushes new ones to a Telegram channel.

## Features

- Fetches top 30 stories from the Hacker News API
- Deduplicates stories using Cloudflare KV storage
- Sends formatted messages to a Telegram channel
- Can be triggered manually via HTTP or automatically on a schedule
- Telegram secrets stored securely using Cloudflare Secrets

## Setup

### 1. Install dependencies

```bash
# Install wrangler CLI
npm install -g wrangler

# Install the worker-build tool
cargo install worker-build
```

### 2. Create a Telegram Bot

1. Talk to [@BotFather](https://t.me/BotFather) on Telegram
2. Send `/newbot` and follow the instructions
3. Copy the bot token (looks like `123456:ABC-DEF1234ghIkl-zyx57W2v1u123ew11`)
4. Create a channel or group, add your bot as an admin
5. Get the chat ID (you can use [@userinfobot](https://t.me/userinfobot) or send a message to the channel and check `https://api.telegram.org/bot<YOUR_BOT_TOKEN>/getUpdates`)

### 3. Configure Wrangler

Edit `wrangler.toml`:

```toml
name = "top-hacker-news"
main = "build/worker/shim.mjs"
compatibility_date = "2025-01-01"

kv_namespaces = [
  { binding = "PUSHED_STORIES", id = "<your-kv-namespace-id>", preview_id = "<your-preview-kv-namespace-id>" }
]
```

### 4. Create KV Namespace

```bash
# Create production KV namespace
wrangler kv namespace create PUSHED_STORIES

# Create preview KV namespace
wrangler kv namespace create PUSHED_STORIES --env preview
```

Copy the IDs into `wrangler.toml`.

### 5. Set Secrets

**Never commit secrets to source code.** Use wrangler to set them securely:

```bash
# Set Telegram bot token
wrangler secret put TELEGRAM_BOT_TOKEN

# Set Telegram chat ID
wrangler secret put TELEGRAM_CHAT_ID
```

These secrets are encrypted at rest and only accessible at runtime.

### 6. Deploy

```bash
wrangler deploy
```

### 7. Set up Cron Triggers (Optional)

To automatically push stories on a schedule, add to `wrangler.toml`:

```toml
[triggers]
crons = ["0 */6 * * *"]  # Every 6 hours
```

Then deploy:

```bash
wrangler deploy
```

You can also trigger manually:

```bash
curl https://top-hacker-news.<your-subdomain>.workers.dev
```

## How it Works

1. Worker fetches the top 30 story IDs from the HN Firebase API
2. For each story, checks KV storage to see if it was already pushed
3. Fetches full story details for new stories
4. Formats a nice HTML message with links
5. Sends the message via the Telegram Bot API
6. Stores each pushed story ID in KV with a timestamp

## Project Structure

```
├── Cargo.toml          # Rust dependencies
├── wrangler.toml       # Cloudflare Worker configuration
├── src/
│   ├── lib.rs          # Main worker entry point
│   ├── hn.rs           # Hacker News API client
│   └── telegram.rs     # Telegram Bot API client
└── .gitignore
```

## Security

- Telegram bot token and chat ID are stored as Cloudflare Secrets, not in source code
- KV storage is used for deduplication with timestamps
- No secrets are logged or exposed in responses

use std::sync::Arc;
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::{
    fmt,
    layer::{Context, Layer},
    util::SubscriberInitExt,
    EnvFilter,
};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use reqwest::Client;
use tracing::field::{Visit, Field};
use serde_json::json;
use tracing_subscriber::layer::SubscriberExt;
use tokio;
use tracing_subscriber::filter::LevelFilter;

pub fn setup_logging() {
    std::fs::create_dir_all("../../logs").ok();

    let app_file = RollingFileAppender::new(Rotation::DAILY, "../../logs", "app.log");
    let error_file = RollingFileAppender::new(Rotation::DAILY, "../../logs", "error.log");

    let app_layer = fmt::layer()
        .with_writer(app_file)
        .json()
        .with_file(true)
        .with_line_number(true)
        .flatten_event(true)
        .with_filter(LevelFilter::TRACE);

    let error_layer = fmt::layer()
        .with_writer(error_file)
        .json()
        .with_file(true)
        .with_line_number(true)
        .flatten_event(true)
        .with_filter(LevelFilter::ERROR);

    let console_layer = fmt::layer()
        .with_file(true)
        .with_line_number(true)
        .with_writer(std::io::stdout);

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let discord_layer = DiscordAlertLayer::new();

    tracing_subscriber::registry()
        .with(env_filter)
        .with(console_layer)
        .with(app_layer)
        .with(error_layer)
        .with(discord_layer)
        .init();
}

struct DiscordAlertLayer {
    client: Arc<Client>,
    webhook_url: String,
}

impl DiscordAlertLayer {
    pub fn new() -> Self {
        let webhook_url = std::env::var("DISCORD_WEBHOOK_URL").expect("Missing DISCORD_WEBHOOK_URL in .env");

        Self {
            client: Arc::new(Client::new()),
            webhook_url,
        }
    }
}

impl<S> Layer<S> for DiscordAlertLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        if *event.metadata().level() != Level::ERROR {
            return;
        }

        let mut visitor = MsgVisitor::default();
        event.record(&mut visitor);

        let timestamp = chrono::Utc::now();
        let timestamp_str = timestamp.format("%Y-%m-%d %H:%M:%S").to_string();

        let mut main_message = String::new();
        let mut other_fields = Vec::new();

        for (k, v) in &visitor.fields {
            if k == "message" {
                main_message = v.clone();
            } else {
                other_fields.push(format!("**{}:** {}", k, v));
            }
        }

        let mut description = String::new();
        description.push_str(&format!("**Message:** {}\n", main_message));
        if !other_fields.is_empty() {
            description.push_str(&other_fields.join("\n"));
            description.push('\n');
        }
        description.push_str(&format!("**Timestamp:** {}", timestamp_str));

        let client = self.client.clone();
        let webhook = self.webhook_url.clone();

        tokio::spawn(async move {
            let _ = client
                .post(&webhook)
                .json(&json!({
                    "embeds": [
                        {
                            "title": "⛔ Error Logged",
                            "color": 0xD32F2F,
                            "description": description,
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                            "footer": {
                                "text": "Backend Server"
                            }
                        }
                    ]
                }))
                .send()
                .await;
        });
    }
}

#[derive(Default)]
struct MsgVisitor {
    fields: Vec<(String, String)>,
}

impl Visit for MsgVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.fields.push((field.name().to_string(), value.to_string()));
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.fields.push((field.name().to_string(), format!("{:?}", value)));
    }
}

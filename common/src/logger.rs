use tracing::{Event, Level, Subscriber};
use tracing_subscriber::{fmt, layer::{Context, Layer}, util::SubscriberInitExt, EnvFilter};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing::field::{Visit, Field};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::filter::LevelFilter;
use crate::discord_logger::log_error;

pub fn setup_logging(namespace: &str) {
    let path = format!("logs/{namespace}");
    std::fs::create_dir_all(&path).ok();

    let app_file = RollingFileAppender::new(Rotation::DAILY, &path, "app.log");
    let error_file = RollingFileAppender::new(Rotation::DAILY, &path, "error.log");

    let app_layer = fmt::layer()
        .with_writer(app_file)
        .json()
        .with_file(true)
        .with_target(false)
        .with_line_number(true)
        .flatten_event(true)
        .with_filter(LevelFilter::TRACE);

    let error_layer = fmt::layer()
        .with_writer(error_file)
        .json()
        .with_target(false)
        .with_file(true)
        .with_line_number(true)
        .flatten_event(true)
        .with_filter(LevelFilter::ERROR);

    let console_layer = fmt::layer()
        .with_target(false)
        .with_file(true)
        .with_line_number(true)
        .with_writer(std::io::stdout);

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,tantivy=warn"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(console_layer)
        .with(app_layer)
        .with(error_layer)
        .with(DiscordAlertLayer)
        .init();
}

struct DiscordAlertLayer;

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

        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        let mut main_message = String::new();
        let mut other_fields = Vec::new();

        for (k, v) in &visitor.fields {
            match k.as_str() {
                "message" => main_message = v.clone(),
                "context" => match serde_json::from_str::<Vec<(String, String)>>(&v) {
                    Err(_) => other_fields.push(format!("**{}:** {}", k, v)),
                    Ok(parsed) => {
                        for (key, val) in parsed {
                            other_fields.push(format!("**{}:** {}", key, val));
                        }
                    }
                }
                _ => other_fields.push(format!("**{}:** {}", k, v))
            }
        }

        let mut description = String::new();
        description.push_str(&format!("**Message:** {}\n", main_message));
        if !other_fields.is_empty() {
            description.push_str(&other_fields.join("\n"));
            description.push('\n');
        }
        description.push_str(&format!("**Timestamp:** {}", timestamp));

        log_error(description);
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
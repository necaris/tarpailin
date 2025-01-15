use misanthropic::tool::{Result as ToolResult, Use as ToolUse};
use misanthropic::{json, prompt::Message, Tool};
use nanohtml2text::html2text;

pub(crate) mod fetch_url {
    use super::{html2text, json, Tool};

    pub async fn run(u: &str) -> Result<String, Box<dyn std::error::Error>> {
        let req = reqwest::get(u).await?;
        let content = req.text().await?;
        let stripped = html2text(&content);
        Ok(stripped)
    }

    pub fn build() -> Result<Tool<'static>, Box<dyn std::error::Error>> {
        Ok(Tool::builder("fetch_url")
            .description("Fetch a URL and return its contents as text. Use this tool when making a web request is required.")
            .schema(json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "The URL to fetch",
                    },
                },
                "required": ["url"],
            }))
            .build()?)
    }
}

/// Handle the tool call. Returns a [`User`] [`Message`] with the result.
///
/// [`User`]: Role::User
pub async fn handle_call(
    call: &ToolUse<'_>,
) -> Result<Message<'static>, Box<dyn std::error::Error>> {
    let call_result = (match call.name.as_ref() {
        "fetch_url" => {
            let s = call.input["url"]
                .as_str()
                .expect("No URL provided to tool call");
            fetch_url::run(s).await
        }
        _ => Err(format!("Unknown tool: {}", call.name).into()),
    })?;
    let tool_result = ToolResult {
        tool_use_id: call.id.to_string().into(),
        content: call_result.to_string().into(),
        is_error: false,
    };
    Ok(tool_result.into_static().into())
}

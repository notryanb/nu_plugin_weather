use futures::executor::block_on;
use nu_errors::ShellError;
use nu_plugin::{serve_plugin, Plugin};
use nu_protocol::{
    CallInfo, CommandAction, ReturnSuccess, ReturnValue, Signature, UntaggedValue, Value,
};
use nu_source::{AnchorLocation, Span, Tag};

struct Weather {
    pub api_key: String,
}

impl Plugin for Weather {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("weather")
            .desc("Displays weather information")
            .filter())
    }

    fn begin_filter(&mut self, call_info: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        let url = format!(
            "https://api.openweathermap.org/data/2.5/forecast?&mode=json&q=huntington&appid={}",
            self.api_key
        );
        Ok(vec![block_on(weather_helper(&url, &call_info.name_tag))])
    }

    fn filter(&mut self, value: Value) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(vec![])
    }

    fn end_filter(&mut self) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(vec![])
    }
}

fn main() {
    let api_key = std::env::var("OPEN_WEATHER_API_KEY")
        .expect("Missing OPEN_WEATHER_API_KEY ENV var")
        .to_string();
    serve_plugin(&mut Weather { api_key });
}

pub async fn weather_helper(url: &str, tag: &Tag) -> ReturnValue {
    let span = tag.span;
    let result = make_request(&url, &span).await;

    if let Err(e) = result {
        return Err(e);
    }

    let (file_extension, contents, contents_tag) = result?;
    let tagged_contents = contents.retag(tag);

    Ok(ReturnSuccess::Action(CommandAction::AutoConvert(
        tagged_contents,
        "json".to_string(),
    )))
}

async fn make_request(
    url: &str,
    span: &Span,
) -> Result<(Option<String>, UntaggedValue, Tag), ShellError> {
    let mut response = surf::get(&url).await?;

    Ok((
        Some("json".to_string()),
        UntaggedValue::string(response.body_string().await.map_err(|_| {
            ShellError::labeled_error(
                "Could not load text from remote url",
                "could not load",
                span,
            )
        })?),
        Tag {
            span: *span,
            anchor: Some(AnchorLocation::Url(url.to_string())),
        },
    ))
}

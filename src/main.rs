use futures::executor::block_on;
use nu_errors::ShellError;
use nu_plugin::{serve_plugin, Plugin};
use nu_protocol::{
    CallInfo, CommandAction, ReturnSuccess, ReturnValue, Signature, SyntaxShape, UntaggedValue, Value,
};
use nu_source::{AnchorLocation, Span, Tag};
use serde::{Deserialize, Serialize};
use serde::ser::{Serializer, SerializeStruct};

struct Weather {
    pub api_key: String,
    pub city: Option<String>,
}

impl Plugin for Weather {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("weather")
           .desc("Displays weather information")
           .named(
               "city",
               SyntaxShape::Any,
               "the city to retrieve weather for",
               Some('c'),
               )
           .filter())
    }

    fn begin_filter(&mut self, call_info: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        self.city = match call_info.args.get("city") {
            Some(city) => Some(city.as_string()?),
            None => Some("huntington".to_string()),
        };

        let url = format!(
            "https://api.openweathermap.org/data/2.5/forecast?&mode=json&q={}&appid={}",
            self.city.as_ref().unwrap(),
            self.api_key
            );
        Ok(vec![block_on(weather_helper(&url, &call_info))])
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
    serve_plugin(&mut Weather { api_key, city: None });
}

pub async fn weather_helper(url: &str, call_info: &CallInfo) -> ReturnValue {
    let tag = &call_info.name_tag;
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

    // Deserialize json
    let api_response: ApiResponse = serde_json::from_str(&response.body_string().await.unwrap()).unwrap();
    dbg!(&api_response);

    let serialized = serde_json::to_string(&api_response.list);

    Ok((
            Some("json".to_string()),
            UntaggedValue::string(serialized.map_err(|_| {
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

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse {
    pub city: City,
    pub list: Vec<List>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct List {
    pub main: Main,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Main {
    pub temp: f32,
}

#[derive(Debug, Deserialize)]
pub struct City {
    pub id: i32,
    pub name: String,
    pub population: i32,
    pub timezone: i64,
    pub sunrise: i64,
    pub sunset: i64,
}

impl Serialize for City {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use chrono::{Utc, TimeZone};
        let sunrise_str = Utc.timestamp(self.sunrise, 0).to_string();
        let sunset_str = Utc.timestamp(self.sunset, 0).to_string();

        let sunrise_tz = &self.sunrise + &self.timezone;
        let sunrise_tz_str = Utc.timestamp(sunrise_tz, 0).to_string();
        let sunset_tz = &self.sunset + &self.timezone;
        let sunset_tz_str = Utc.timestamp(sunset_tz, 0).to_string();

        let mut state = serializer.serialize_struct("City", 6)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("population", &self.population)?;
        state.serialize_field("sunrise_utc", &sunrise_str)?;
        state.serialize_field("sunset_utc", &sunset_str)?;
        state.serialize_field("sunrise_tz", &sunrise_tz_str)?;
        state.serialize_field("sunset_tz", &sunset_tz_str)?;
        state.end()
    }
}

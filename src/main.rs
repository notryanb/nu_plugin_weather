use futures::executor::block_on;
use nu_errors::ShellError;
use nu_plugin::{serve_plugin, Plugin};
use nu_protocol::{
    CallInfo, CommandAction, ReturnSuccess, ReturnValue, Signature, SyntaxShape, UntaggedValue,
    Value,
};
use nu_source::{AnchorLocation, Span, Tag};
use serde::ser::{SerializeStruct, Serializer};
use serde::{Deserialize, Serialize};

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
            "https://api.openweathermap.org/data/2.5/weather?q={}&appid={}",
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
    serve_plugin(&mut Weather {
        api_key,
        city: None,
    });
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
    let api_response: List = serde_json::from_str(&response.body_string().await.unwrap()).unwrap();
    let serialized = serde_json::to_string(&api_response);

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

#[derive(Debug, Deserialize)]
pub struct List {
    pub dt_txt: Option<String>,
    pub dt: i64,
    pub main: Main,
    pub weather: Vec<CurrentWeather>,
    pub timezone: i64,
}

impl Serialize for List {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use chrono::{TimeZone, Utc};
        let dt = Utc.timestamp(self.dt + self.timezone, 0);
        let day_of_week = &dt.format("%A").to_string();
        let date = &dt.format("%b %e %Y").to_string();
        let time = &dt.format("%I:%M:%S %P").to_string();

        let mut state = serializer.serialize_struct("List", 8)?;
        state.serialize_field("date", &date)?;
        state.serialize_field("time", &time)?;
        state.serialize_field("day_of_week", &day_of_week)?;
        state.serialize_field("temperature", &(1.8 * (&self.main.temp - 273.15) + 32.0))?;
        state.serialize_field(
            "feels_like",
            &(1.8 * (&self.main.feels_like - 273.15) + 32.0),
        )?;

        if let Some(weather) = &self.weather.iter().take(1).next() {
            let emoji = match &weather.main {
                WeatherCondition::Clouds => "☁",
                WeatherCondition::Clear => "🌞",
                WeatherCondition::Rain => "🌧",
                WeatherCondition::Haze => "🌫",
                _ => "no emoji",
            };

            state.serialize_field("main", &weather.main)?;
            state.serialize_field("description", &weather.description)?;
            state.serialize_field("emoji", &emoji)?;
        }

        state.end()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum WeatherCondition {
    Clouds,
    Clear,
    Thunderstorm,
    Drizzle,
    Rain,
    Snow,
    Mist,
    Smoke,
    Haze,
    Dust,
    Fog,
    Sand,
    Ash,
    Squall,
    Tornado,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Main {
    pub temp: f32,
    pub feels_like: f32,
    pub temp_min: f32,
    pub temp_max: f32,
    pub pressure: i32,
    pub sea_level: Option<i32>,
    pub grnd_level: Option<i32>,
    pub humidity: i32,
    pub temp_kf: Option<f32>,
}

#[derive(Debug, Deserialize)]
pub struct CurrentWeather {
    pub main: WeatherCondition,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct City {
    pub id: i32,
    pub name: String,
    pub population: i32,
    pub timezone: i64,
    pub sunrise: i64,
    pub sunset: i64,
}

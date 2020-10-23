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
    pub api_key: Option<String>,
    pub city: Option<String>,
    pub info_type: Option<String>,
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
            .named("type", SyntaxShape::Any, "current or forecast", Some('t'))
            .filter())
    }

    fn begin_filter(&mut self, call_info: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        let existing_tag = call_info.name_tag.clone();
        let result = nu_data::config::read(&existing_tag, &None)?;
        let cloned_span = existing_tag.clone();

        let value = result.get("open_weather_api_key").ok_or_else(|| {
            ShellError::labeled_error(
                "Missing 'open_weather_api_key' key in config",
                "key",
                cloned_span,
            )
        })?;

        self.api_key = Some(value.expect_string().to_owned());

        self.city = match call_info.args.get("city") {
            Some(city) => Some(city.as_string()?),
            None => Some("huntington".to_string()),
        };

        self.info_type = match call_info.args.get("type") {
            Some(info_type) => Some(info_type.as_string()?),
            None => Some("current".to_string()),
        };

        let url;

        if self.info_type.as_ref().unwrap() == &"current".to_string() {
            url = format!(
                "https://api.openweathermap.org/data/2.5/weather?q={}&appid={}",
                self.city.as_ref().unwrap(),
                self.api_key.as_ref().unwrap(),
            );
        } else {
            url = format!(
                "https://api.openweathermap.org/data/2.5/forecast?q={}&mode=json&appid={}",
                self.city.as_ref().unwrap(),
                self.api_key.as_ref().unwrap(),
            );
        }

        Ok(vec![block_on(weather_helper(&url, &call_info))])
    }

    fn filter(&mut self, _value: Value) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(vec![])
    }

    fn end_filter(&mut self) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(vec![])
    }
}

fn main() {
    serve_plugin(&mut Weather {
        api_key: None,
        city: None,
        info_type: None,
    });
}

pub async fn weather_helper(url: &str, call_info: &CallInfo) -> ReturnValue {
    let tag = &call_info.name_tag;
    let span = tag.span;
    let info_type = match call_info.args.get("type") {
        Some(info_type) => Some(info_type.as_string()?),
        None => Some("current".to_string()),
    };
    let result = make_request(&url, &info_type.unwrap(), &span).await;

    if let Err(e) = result {
        return Err(e);
    }

    let (_file_extension, contents, _contents_tag) = result?;

    Ok(ReturnSuccess::Action(CommandAction::AutoConvert(
        contents.into_value(tag),
        "json".to_string(),
    )))
}

async fn make_request(
    url: &str,
    info_type: &str,
    span: &Span,
) -> Result<(Option<String>, UntaggedValue, Tag), ShellError> {
    let mut response = surf::get(&url).await?;
    let response_body = &response.body_string().await?;

    if info_type == "current" {
        // Deserialize json
        let api_response: List = serde_json::from_str(response_body)?;
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
    } else {
        // Deserialize json
        let api_response: ApiResponse =
            serde_json::from_str(&response.body_string().await.unwrap()).unwrap();
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
    pub timezone: Option<i64>,
}

impl Serialize for List {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use chrono::prelude::*;
        let dt = Utc.timestamp(self.dt + self.timezone.unwrap_or(0), 0);
        let hour = &dt.hour();
        // let the_time = &dt.time();
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
                WeatherCondition::Clear if (*hour > 6 && *hour < 16) => "☀",
                WeatherCondition::Clear if (*hour <= 6 || *hour >= 16 )=> "🌑",
                WeatherCondition::Rain => "🌧",
                WeatherCondition::Snow => "🌨",
                WeatherCondition::Thunderstorm => "⛈",
                WeatherCondition::Tornado => "🌪",
                WeatherCondition::Haze => "🌫",
                _ => "",
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

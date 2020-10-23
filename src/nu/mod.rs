use futures::executor::block_on;
use nu_errors::ShellError;
use nu_plugin::Plugin;
use nu_protocol::{CallInfo, ReturnValue, Signature, SyntaxShape};

use crate::weather::weather_helper;
use crate::Weather;

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
}

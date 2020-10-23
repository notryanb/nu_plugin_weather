use nu_plugin::serve_plugin;
use nu_plugin_weather::Weather;

fn main() {
    serve_plugin(&mut Weather {
        api_key: None,
        city: None,
        info_type: None,
    });
}

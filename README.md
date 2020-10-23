# nu_plugin_weather

This is currently a _very experimental_ project for wrapping the OpenWeatherAPI in a nu plugin.
The default location is Huntington, NY, however a city can be passed in with `--city` or `-c`.
The city must currently have no spaces... because I have not yet safely encoded the URL query params.

The raw JSON is returned and turned into a table. 
Eventually this will be mapped to much nicer data displaying emojis.

## Setup (currently only build from source)

- From the build directory, `cargo install --path .`
- You must have an API Key from the [OpenWeather API](https://openweathermap.org/api)
- Add a section to your nushell config called `open_weather_api_key`. ex. `config set open_weather_api_key <YOUR API KEY HERE>`

## Usage

- `weather`
- `weather --city philadelphia`

## Ideas & Goals

- Have good top level information for 7 day forecast, hourly forecast, and current forecast.
- Support emojis for weather display
- Optional temperature scales [ Fahrenheit, Celcius, Kelvin ]
- Easier to read date time (conversions from Unix epoch). Timezones!
- Support for more countries than US (default scales based on country?)
- Better structure for code - see supported nu plugins.
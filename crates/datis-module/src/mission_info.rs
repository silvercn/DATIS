use std::str::FromStr;
use std::sync::{Arc, Mutex};

use datis_core::{
    mission_info::{Clouds, MissionInfo, WeatherInfo},
    station::Position,
};
use hlua51::{Lua, LuaFunction, LuaTable};

#[derive(Debug)]
pub struct DcsMissionInfoInner {
    lua: Lua<'static>,
    clouds: Option<Clouds>,
    fog_thickness: u32,  // in m
    fog_visibility: u32, // in m
}

#[derive(Debug, Clone)]
pub struct DcsMissionInfo(Arc<Mutex<DcsMissionInfoInner>>);

impl DcsMissionInfo {
    pub fn create(
        mut lua: Lua<'static>,
        clouds: Option<Clouds>,
        fog_thickness: u32,
        fog_visibility: u32,
    ) -> Result<Self, anyhow::Error> {
        {
            lua.execute::<()>(LUA_CODE)?;
        }

        Ok(DcsMissionInfo(Arc::new(Mutex::new(DcsMissionInfoInner {
            lua,
            clouds,
            fog_thickness,
            fog_visibility,
        }))))
    }
}

impl MissionInfo for DcsMissionInfo {
    fn get_weather_at(&self, x: f64, y: f64, alt: f64) -> Result<WeatherInfo, anyhow::Error> {
        let mut inner = self.0.lock().unwrap();
        let clouds = inner.clouds.clone();

        let visibility = if inner.fog_thickness > 200 {
            Some(inner.fog_visibility)
        } else {
            None
        };

        // call `getWeather(x, y, alt)`
        let mut get_weather: LuaFunction<_> = get!(inner.lua, "getWeather")?;

        let pressure_qnh: f64 = {
            let mut weather: LuaTable<_> = get_weather
                .call_with_args((x, y, 0))
                .map_err(|_| anyhow!("failed to call lua function getWeather"))?;
            get!(weather, "pressure")
        }?;

        let mut weather: LuaTable<_> = get_weather
            .call_with_args((x, y, alt))
            .map_err(|_| anyhow!("failed to call lua function getWeather"))?;
        let wind_speed: f64 = get!(weather, "windSpeed")?;
        let mut wind_dir: f64 = get!(weather, "windDir")?; // in knots
        let temperature: f64 = get!(weather, "temp")?;
        let pressure_qfe: f64 = get!(weather, "pressure")?;

        // convert to degrees and rotate wind direction
        wind_dir = wind_dir.to_degrees() - 180.0;

        // normalize wind direction
        while wind_dir < 0.0 {
            wind_dir += 360.0;
        }

        Ok(WeatherInfo {
            clouds,
            visibility,
            wind_speed,
            wind_dir,
            temperature,
            pressure_qnh,
            pressure_qfe,
        })
    }

    fn get_unit_position(&self, name: &str) -> Result<Option<Position>, anyhow::Error> {
        let mut inner = self.0.lock().unwrap();

        // call `getUnitPosition(name)`
        let mut get_unit_position: LuaFunction<_> = get!(inner.lua, "getUnitPosition")?;
        let mut pos: LuaTable<_> = get_unit_position
            .call_with_args(name)
            .map_err(|err| anyhow!("failed to call lua function getUnitPosition {}", err))?;
        let x: f64 = get!(pos, "x")?;
        let y: f64 = get!(pos, "y")?;
        let alt: f64 = get!(pos, "alt")?;

        if x == 0.0 && y == 0.0 && alt == 0.0 {
            Ok(None)
        } else {
            Ok(Some(Position { x, y, alt }))
        }
    }

    fn get_unit_heading(&self, name: &str) -> Result<Option<f64>, anyhow::Error> {
        let mut inner = self.0.lock().unwrap();

        // call `getUnitHeading(name)`
        let mut get_unit_position: LuaFunction<_> = get!(inner.lua, "getUnitHeading")?;
        let heading: String = get_unit_position
            .call_with_args(name)
            .map_err(|err| anyhow!("failed to call lua function getUnitHeading {}", err))?;

        Ok(if heading.is_empty() {
            None
        } else {
            f64::from_str(&heading).ok()
        })
    }
}

impl PartialEq for DcsMissionInfo {
    fn eq(&self, other: &DcsMissionInfo) -> bool {
        let lhs = self.0.lock().unwrap();
        let rhs = other.0.lock().unwrap();
        lhs.clouds == rhs.clouds
            && lhs.fog_thickness == rhs.fog_thickness
            && lhs.fog_thickness == rhs.fog_thickness
    }
}

// north correction is based on https://github.com/mrSkortch/MissionScriptingTools
#[cfg(not(test))]
static LUA_CODE: &str = r#"
    local Weather = require 'Weather'
    getWeather = function(x, y, alt)
        local position = {
            x = x,
            y = alt,
            z = y,
        }
        local wind = Weather.getGroundWindAtPoint({
            position = position
        })
        local temp, pressure = Weather.getTemperatureAndPressureAtPoint({
            position = position
        })

        return {
            windSpeed = wind.v,
            windDir = wind.a,
            temp = temp,
            pressure = pressure,
        }
    end

    getUnitPosition = function(name)
        local get_unit_position = [[
            local unit = Unit.getByName("]] .. name .. [[")
            if unit == nil then
                return ""
            else
                local pos = unit:getPoint()
                return  pos.x .. ":" .. pos.z .. ":" .. pos.y
            end
        ]]

        local result = net.dostring_in("server", get_unit_position)
        local x, y, alt = string.match(result, "(%-?[0-9%.-]+):(%-?[0-9%.]+):(%-?[0-9%.]+)")

        return {
            x = x,
            y = y,
            alt = alt,
        }
    end

    getUnitHeading = function(name)
        local get_unit_heading = [[
            local unit = Unit.getByName("]] .. name .. [[")
            if unit == nil then
                return ""
            else
                local unit_pos = unit:getPosition()
                local lat, lon = coord.LOtoLL(unit_pos.p)
                local north_pos = coord.LLtoLO(lat + 1, lon)
                local northCorrection = math.atan2(north_pos.z - unit_pos.p.z, north_pos.x - unit_pos.p.x)

                local heading = math.atan2(unit_pos.x.z, unit_pos.x.x) + northCorrection
                if heading < 0 then
                    heading = heading + 2*math.pi
                end

                return tostring(heading)
            end
        ]]

        return net.dostring_in("server", get_unit_heading)
    end
"#;

#[cfg(test)]
static LUA_CODE: &str = r#"
    function getWeather(x, y, alt)
        return {
            windSpeed = x,
            windDir = y,
            temp = alt,
            pressure = 42,
        }
    end

    function getUnitPosition(name)
        return {
            x = 1,
            y = 2,
            alt = 3,
        }
    end
"#;

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_get_weather() {
        let dw = DcsMissionInfo::create("", None, None).unwrap();
        assert_eq!(
            dw.get_at(1.0, 2.0_f64.to_radians(), 3.0).unwrap(),
            WeatherInfo {
                position: Position {
                    x: 1.0,
                    y: 2.0_f64.to_radians(),
                    alt: 3.0
                },
                clouds: None,
                visibility: None,
                wind_speed: 1.0,
                wind_dir: 182.0,
                temperature: 3.0,
                pressure_qnh: 42.0,
                pressure_qfe: 42.0,
            }
        );
    }

    #[test]
    fn test_get_weather_for_unit() {
        let dw = DcsMissionInfo::create("", None, None).unwrap();
        assert_eq!(
            dw.get_for_unit("foobar").unwrap(),
            Some(WeatherInfo {
                position: Position {
                    x: 1.0,
                    y: 2.0,
                    alt: 3.0
                },
                clouds: None,
                visibility: None,
                wind_speed: 1.0,
                wind_dir: 294.59155902616465,
                temperature: 3.0,
                pressure_qnh: 42.0,
                pressure_qfe: 42.0,
            })
        );
    }
}
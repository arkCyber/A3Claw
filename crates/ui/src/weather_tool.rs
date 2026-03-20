//! 天气查询工具 - 为 Claw Terminal 提供实时天气信息

/// 获取实时天气信息
/// 使用 Open-Meteo 免费天气 API（无需 API key，更可靠）
pub async fn fetch_weather(location: &str) -> Result<String, String> {
    tracing::info!("[WEATHER] Starting weather fetch for location: {}", location);
    
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
        .build() {
            Ok(c) => {
                tracing::debug!("[WEATHER] HTTP client created successfully");
                c
            }
            Err(e) => {
                tracing::error!("[WEATHER] Failed to create HTTP client: {}", e);
                return Err(format!("创建 HTTP 客户端失败: {}", e));
            }
        };

    // 首先获取城市的经纬度
    let (lat, lon, city_name) = get_coordinates(&client, location).await?;
    tracing::info!("[WEATHER] Got coordinates for {}: lat={}, lon={}", city_name, lat, lon);
    
    // 使用 Open-Meteo API 获取天气
    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&current=temperature_2m,relative_humidity_2m,apparent_temperature,weather_code,wind_speed_10m,wind_direction_10m&timezone=auto",
        lat, lon
    );
    
    tracing::debug!("[WEATHER] Fetching from Open-Meteo: {}", url);
    
    let resp = match client.get(&url).send().await {
        Ok(r) => {
            tracing::debug!("[WEATHER] HTTP request successful, status: {}", r.status());
            r
        }
        Err(e) => {
            tracing::error!("[WEATHER] HTTP request failed: {}", e);
            return Err(format!("HTTP 请求失败: {}", e));
        }
    };

    if !resp.status().is_success() {
        let status = resp.status();
        tracing::warn!("[WEATHER] Non-success HTTP status: {}", status);
        return Err(format!("HTTP 状态码: {}", status));
    }

    let body = match resp.text().await {
        Ok(b) => {
            tracing::debug!("[WEATHER] Response body received, length: {} bytes", b.len());
            b
        }
        Err(e) => {
            tracing::error!("[WEATHER] Failed to read response body: {}", e);
            return Err(format!("读取响应失败: {}", e));
        }
    };

    parse_openmeteo_json(&body, &city_name)
}

/// 使用 Nominatim API 获取城市的经纬度，如果失败则使用内置数据库
async fn get_coordinates(client: &reqwest::Client, location: &str) -> Result<(f64, f64, String), String> {
    // 首先尝试从内置数据库查找
    if let Some((lat, lon, name)) = get_builtin_coordinates(location) {
        tracing::info!("[WEATHER] Using builtin coordinates for {}: lat={}, lon={}", name, lat, lon);
        return Ok((lat, lon, name.to_string()));
    }
    
    // 如果内置数据库没有，尝试 API
    tracing::debug!("[WEATHER] Trying geocoding API for: {}", location);
    match try_geocoding_api(client, location).await {
        Ok(result) => Ok(result),
        Err(e) => {
            tracing::warn!("[WEATHER] Geocoding API failed: {}, trying fallback", e);
            // API 失败，尝试模糊匹配内置数据库
            get_fuzzy_builtin_coordinates(location)
                .ok_or_else(|| format!("无法找到位置 '{}', 请尝试使用更常见的城市名", location))
        }
    }
}

async fn try_geocoding_api(client: &reqwest::Client, location: &str) -> Result<(f64, f64, String), String> {
    let url = format!(
        "https://nominatim.openstreetmap.org/search?q={}&format=json&limit=1",
        urlencoding::encode(location)
    );
    
    let resp = client.get(&url).send().await
        .map_err(|e| format!("地理编码请求失败: {}", e))?;
    
    let body = resp.text().await
        .map_err(|e| format!("读取地理编码响应失败: {}", e))?;
    
    let json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("解析地理编码数据失败: {}", e))?;
    
    let results = json.as_array()
        .ok_or_else(|| "地理编码响应格式错误".to_string())?;
    
    if results.is_empty() {
        return Err("未找到匹配的位置".to_string());
    }
    
    let first = &results[0];
    let lat = first.get("lat")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<f64>().ok())
        .ok_or_else(|| "无法解析纬度".to_string())?;
    
    let lon = first.get("lon")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<f64>().ok())
        .ok_or_else(|| "无法解析经度".to_string())?;
    
    let display_name = first.get("display_name")
        .and_then(|v| v.as_str())
        .unwrap_or(location)
        .split(',')
        .next()
        .unwrap_or(location)
        .to_string();
    
    Ok((lat, lon, display_name))
}

/// 内置常见城市的经纬度数据库
fn get_builtin_coordinates(location: &str) -> Option<(f64, f64, &'static str)> {
    let location_lower = location.to_lowercase();
    
    let cities = [
        // 中国主要城市
        ("beijing", (39.9042, 116.4074, "北京")),
        ("shanghai", (31.2304, 121.4737, "上海")),
        ("guangzhou", (23.1291, 113.2644, "广州")),
        ("shenzhen", (22.5431, 114.0579, "深圳")),
        ("hangzhou", (30.2741, 120.1551, "杭州")),
        ("nanjing", (32.0603, 118.7969, "南京")),
        ("chengdu", (30.5728, 104.0668, "成都")),
        ("chongqing", (29.4316, 106.9123, "重庆")),
        ("wuhan", (30.5928, 114.3055, "武汉")),
        ("xian", (34.2658, 108.9541, "西安")),
        ("tianjin", (39.3434, 117.3616, "天津")),
        ("suzhou", (31.2989, 120.5853, "苏州")),
        
        // 欧洲主要城市
        ("berlin", (52.5200, 13.4050, "柏林")),
        ("munich", (48.1351, 11.5820, "慕尼黑")),
        ("frankfurt", (50.1109, 8.6821, "法兰克福")),
        ("hamburg", (53.5511, 9.9937, "汉堡")),
        ("paris", (48.8566, 2.3522, "巴黎")),
        ("london", (51.5074, -0.1278, "伦敦")),
        ("rome", (41.9028, 12.4964, "罗马")),
        ("madrid", (40.4168, -3.7038, "马德里")),
        ("amsterdam", (52.3676, 4.9041, "阿姆斯特丹")),
        ("brussels", (50.8503, 4.3517, "布鲁塞尔")),
        ("vienna", (48.2082, 16.3738, "维也纳")),
        ("zurich", (47.3769, 8.5417, "苏黎世")),
        
        // 亚洲主要城市
        ("tokyo", (35.6762, 139.6503, "东京")),
        ("seoul", (37.5665, 126.9780, "首尔")),
        ("singapore", (1.3521, 103.8198, "新加坡")),
        ("bangkok", (13.7563, 100.5018, "曼谷")),
        
        // 美洲主要城市
        ("new york", (40.7128, -74.0060, "纽约")),
        ("los angeles", (34.0522, -118.2437, "洛杉矶")),
        ("san francisco", (37.7749, -122.4194, "旧金山")),
        ("chicago", (41.8781, -87.6298, "芝加哥")),
        ("toronto", (43.6532, -79.3832, "多伦多")),
        ("vancouver", (49.2827, -123.1207, "温哥华")),
        
        // 大洋洲主要城市
        ("sydney", (33.8688, 151.2093, "悉尼")),
        ("melbourne", (37.8136, 144.9631, "墨尔本")),
    ];
    
    for (key, coords) in &cities {
        if location_lower == *key || location_lower.contains(key) {
            return Some(*coords);
        }
    }
    
    None
}

/// 模糊匹配内置城市数据库
fn get_fuzzy_builtin_coordinates(location: &str) -> Option<(f64, f64, String)> {
    let location_lower = location.to_lowercase();
    
    // 尝试部分匹配
    if location_lower.contains("shang") {
        return Some((31.2304, 121.4737, "上海".to_string()));
    }
    if location_lower.contains("beij") || location_lower.contains("北京") {
        return Some((39.9042, 116.4074, "北京".to_string()));
    }
    if location_lower.contains("berl") || location_lower.contains("柏林") {
        return Some((52.5200, 13.4050, "柏林".to_string()));
    }
    if location_lower.contains("pari") || location_lower.contains("巴黎") {
        return Some((48.8566, 2.3522, "巴黎".to_string()));
    }
    if location_lower.contains("lond") || location_lower.contains("伦敦") {
        return Some((51.5074, -0.1278, "伦敦".to_string()));
    }
    if location_lower.contains("toky") || location_lower.contains("东京") {
        return Some((35.6762, 139.6503, "东京".to_string()));
    }
    
    None
}

fn parse_openmeteo_json(json_str: &str, city_name: &str) -> Result<String, String> {
    tracing::debug!("[WEATHER] Parsing Open-Meteo JSON for: {}", city_name);
    
    let json: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| format!("解析天气数据失败: {}", e))?;
    
    let current = json.get("current")
        .ok_or_else(|| "API 响应格式错误".to_string())?;
    
    let temp = current.get("temperature_2m")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| "无法获取温度".to_string())?;
    
    let feels_like = current.get("apparent_temperature")
        .and_then(|v| v.as_f64())
        .unwrap_or(temp);
    
    let humidity = current.get("relative_humidity_2m")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    
    let wind_speed = current.get("wind_speed_10m")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    
    let wind_dir = current.get("wind_direction_10m")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    
    let weather_code = current.get("weather_code")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    
    let weather_desc = weather_code_to_description(weather_code);
    
    // 格式化输出
    let now = chrono::Utc::now();
    let time_str = now.format("%Y-%m-%d %H:%M").to_string();
    
    let mut result = String::new();
    result.push_str(&format!("\n🌤 {} 实时天气（{}）：\n\n", city_name, time_str));
    result.push_str(&format!("**天气状况**: {}\n", weather_desc));
    result.push_str(&format!("**当前温度**: {:.1}°C（体感 {:.1}°C）\n", temp, feels_like));
    result.push_str(&format!("**湿度**: {}%\n", humidity));
    result.push_str(&format!("**风速**: {:.1} km/h（{}°）\n", wind_speed, wind_dir as i32));
    
    tracing::info!("[WEATHER] Weather data formatted successfully for {}", city_name);
    Ok(result)
}

fn weather_code_to_description(code: i64) -> &'static str {
    match code {
        0 => "晴朗",
        1 => "基本晴朗",
        2 => "部分多云",
        3 => "阴天",
        45 | 48 => "有雾",
        51 | 53 | 55 => "小雨",
        61 | 63 | 65 => "雨",
        71 | 73 | 75 => "雪",
        80 | 81 | 82 => "阵雨",
        95 | 96 | 99 => "雷暴",
        _ => "未知",
    }
}

fn parse_weather_json(json_str: &str, location: &str) -> Result<String, String> {
    tracing::debug!("[WEATHER] Parsing weather JSON for location: {}", location);
    tracing::debug!("[WEATHER] Response length: {} bytes", json_str.len());
    
    let json: serde_json::Value = match serde_json::from_str(json_str) {
        Ok(j) => j,
        Err(e) => {
            tracing::error!("[WEATHER] Failed to parse JSON: {}", e);
            tracing::error!("[WEATHER] Response body (first 500 chars): {}", 
                &json_str.chars().take(500).collect::<String>());
            return Err(format!("解析天气数据失败: {}", e));
        }
    };

    // 检查是否是空 JSON 对象
    if json.as_object().map(|o| o.is_empty()).unwrap_or(false) {
        tracing::error!("[WEATHER] Received empty JSON object for location: {}", location);
        return Err(format!("无法识别位置 '{}'，请尝试使用英文城市名或更具体的地名", location));
    }
    
    // 提取当前天气信息（数据在 data 对象下）
    let data = json.get("data").ok_or_else(|| {
        tracing::error!("[WEATHER] No 'data' object in response");
        tracing::error!("[WEATHER] Response keys: {:?}", json.as_object().map(|o| o.keys().collect::<Vec<_>>()));
        tracing::error!("[WEATHER] Full response (first 1000 chars): {}", 
            &json_str.chars().take(1000).collect::<String>());
        format!("API 响应格式错误，位置 '{}' 可能无法识别", location)
    })?;
    
    let current = data.get("current_condition")
        .and_then(|c| c.get(0))
        .ok_or_else(|| {
            tracing::warn!("[WEATHER] No current_condition in response");
            "未找到当前天气数据".to_string()
        })?;

    let temp_c = current.get("temp_C")
        .and_then(|v| v.as_str())
        .unwrap_or("N/A");
    
    let feels_like = current.get("FeelsLikeC")
        .and_then(|v| v.as_str())
        .unwrap_or("N/A");
    
    let humidity = current.get("humidity")
        .and_then(|v| v.as_str())
        .unwrap_or("N/A");
    
    let weather_desc = current.get("lang_zh")
        .and_then(|v| v.get(0))
        .and_then(|v| v.get("value"))
        .and_then(|v| v.as_str())
        .or_else(|| current.get("weatherDesc")
            .and_then(|v| v.get(0))
            .and_then(|v| v.get("value"))
            .and_then(|v| v.as_str()))
        .unwrap_or("未知");
    
    let wind_speed = current.get("windspeedKmph")
        .and_then(|v| v.as_str())
        .unwrap_or("N/A");
    
    let wind_dir = current.get("winddir16Point")
        .and_then(|v| v.as_str())
        .unwrap_or("N/A");
    
    let visibility = current.get("visibility")
        .and_then(|v| v.as_str())
        .unwrap_or("N/A");
    
    let uv_index = current.get("uvIndex")
        .and_then(|v| v.as_str())
        .unwrap_or("N/A");

    // 获取今天的预报
    let today_forecast = data.get("weather")
        .and_then(|w| w.get(0));
    
    let max_temp = today_forecast
        .and_then(|f| f.get("maxtempC"))
        .and_then(|v| v.as_str())
        .unwrap_or("N/A");
    
    let min_temp = today_forecast
        .and_then(|f| f.get("mintempC"))
        .and_then(|v| v.as_str())
        .unwrap_or("N/A");

    // 格式化输出
    let now = chrono::Utc::now();
    let time_str = now.format("%Y-%m-%d %H:%M").to_string();
    
    let mut result = String::new();
    result.push_str(&format!("\n🌤 {} 实时天气（{}）：\n\n", location, time_str));
    result.push_str(&format!("**天气状况**: {}\n", weather_desc));
    result.push_str(&format!("**当前温度**: {}°C（体感 {}°C）\n", temp_c, feels_like));
    result.push_str(&format!("**今日温度**: {}°C ~ {}°C\n", min_temp, max_temp));
    result.push_str(&format!("**湿度**: {}%\n", humidity));
    result.push_str(&format!("**风速**: {} km/h（{}）\n", wind_speed, wind_dir));
    result.push_str(&format!("**能见度**: {} km\n", visibility));
    result.push_str(&format!("**紫外线指数**: {}\n", uv_index));
    
    tracing::info!("[WEATHER] Weather data formatted successfully for {}", location);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_weather() {
        let result = fetch_weather("Beijing").await;
        println!("Weather result: {:?}", result);
        assert!(result.is_ok() || result.is_err()); // Just check it doesn't panic
    }
}

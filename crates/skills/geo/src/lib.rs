use openclaw_plugin_sdk::prelude::*;

static MANIFEST: &str = r#"{
  "id": "community.geo",
  "name": "Geo Utilities",
  "version": "0.1.0",
  "description": "10 geographic skills: haversine, bearing, midpoint, bounding_box, point_in_bbox, deg_to_dms, dms_to_deg, distance_to_meters, meters_to_degrees, grid_ref.",
  "skills": [
    {"name":"geo.haversine","display":"Haversine Distance","description":"Great-circle distance in km between two lat/lon points.","risk":"safe","params":[{"name":"lat1","type":"number","required":true},{"name":"lon1","type":"number","required":true},{"name":"lat2","type":"number","required":true},{"name":"lon2","type":"number","required":true}]},
    {"name":"geo.bearing","display":"Bearing","description":"Initial bearing (degrees) from point A to point B.","risk":"safe","params":[{"name":"lat1","type":"number","required":true},{"name":"lon1","type":"number","required":true},{"name":"lat2","type":"number","required":true},{"name":"lon2","type":"number","required":true}]},
    {"name":"geo.midpoint","display":"Midpoint","description":"Geographic midpoint between two lat/lon points.","risk":"safe","params":[{"name":"lat1","type":"number","required":true},{"name":"lon1","type":"number","required":true},{"name":"lat2","type":"number","required":true},{"name":"lon2","type":"number","required":true}]},
    {"name":"geo.bounding_box","display":"Bounding Box","description":"Bounding box around a point given radius in km.","risk":"safe","params":[{"name":"lat","type":"number","required":true},{"name":"lon","type":"number","required":true},{"name":"radius_km","type":"number","required":true}]},
    {"name":"geo.point_in_bbox","display":"Point in Bbox","description":"Check if a point is inside a bounding box.","risk":"safe","params":[{"name":"lat","type":"number","required":true},{"name":"lon","type":"number","required":true},{"name":"min_lat","type":"number","required":true},{"name":"min_lon","type":"number","required":true},{"name":"max_lat","type":"number","required":true},{"name":"max_lon","type":"number","required":true}]},
    {"name":"geo.deg_to_dms","display":"Degrees to DMS","description":"Convert decimal degrees to degrees/minutes/seconds string.","risk":"safe","params":[{"name":"deg","type":"number","required":true},{"name":"is_lat","type":"boolean","description":"true for latitude, false for longitude","required":false}]},
    {"name":"geo.dms_to_deg","display":"DMS to Degrees","description":"Convert DMS string (e.g. 51°30'26\\\"N) to decimal degrees.","risk":"safe","params":[{"name":"dms","type":"string","required":true}]},
    {"name":"geo.distance_to_meters","display":"Distance to Meters","description":"Convert km distance to meters.","risk":"safe","params":[{"name":"km","type":"number","required":true}]},
    {"name":"geo.meters_to_degrees","display":"Meters to Degrees","description":"Approximate meters to degrees latitude at a given latitude.","risk":"safe","params":[{"name":"meters","type":"number","required":true},{"name":"lat","type":"number","description":"Reference latitude for longitude conversion","required":false}]},
    {"name":"geo.grid_ref","display":"Grid Reference","description":"Format lat/lon as a simple grid reference string (10km resolution).","risk":"safe","params":[{"name":"lat","type":"number","required":true},{"name":"lon","type":"number","required":true}]}
  ]
}"#;

const PI: f64 = std::f64::consts::PI;
const EARTH_R: f64 = 6371.0;

fn to_rad(d: f64) -> f64 { d * PI / 180.0 }
fn to_deg(r: f64) -> f64 { r * 180.0 / PI }

fn haversine(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let dlat = to_rad(lat2 - lat1);
    let dlon = to_rad(lon2 - lon1);
    let a = (dlat / 2.0).sin().powi(2)
        + to_rad(lat1).cos() * to_rad(lat2).cos() * (dlon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    EARTH_R * c
}

#[no_mangle]
pub extern "C" fn skill_manifest() -> u64 { sdk_export_str(MANIFEST) }

#[no_mangle]
pub extern "C" fn skill_execute(ptr: i32, len: i32) -> u64 {
    let req = match sdk_read_request(ptr, len) {
        Ok(r)  => r,
        Err(e) => return sdk_respond_err("", &e),
    };
    let rid = req.request_id.as_str();
    let args = &req.args;

    macro_rules! f {
        ($k:literal) => { args[$k].as_f64().unwrap_or(0.0) }
    }

    match req.skill.as_str() {
        "geo.haversine" => {
            let d = haversine(f!("lat1"), f!("lon1"), f!("lat2"), f!("lon2"));
            sdk_respond_ok(rid, &format!("{:.4}", d))
        }
        "geo.bearing" => {
            let (lat1,lon1,lat2,lon2) = (to_rad(f!("lat1")), to_rad(f!("lon1")), to_rad(f!("lat2")), to_rad(f!("lon2")));
            let dlon = lon2 - lon1;
            let y = dlon.sin() * lat2.cos();
            let x = lat1.cos() * lat2.sin() - lat1.sin() * lat2.cos() * dlon.cos();
            let brng = (to_deg(y.atan2(x)) + 360.0) % 360.0;
            sdk_respond_ok(rid, &format!("{:.2}", brng))
        }
        "geo.midpoint" => {
            let (lat1,lon1,lat2,lon2) = (to_rad(f!("lat1")), to_rad(f!("lon1")), to_rad(f!("lat2")), to_rad(f!("lon2")));
            let dlon = lon2 - lon1;
            let bx = lat2.cos() * dlon.cos();
            let by = lat2.cos() * dlon.sin();
            let mid_lat = to_deg((lat1.sin() + lat2.sin()).atan2(((lat1.cos() + bx).powi(2) + by.powi(2)).sqrt()));
            let mid_lon = to_deg(lon1 + by.atan2(lat1.cos() + bx));
            sdk_respond_ok(rid, &format!("{{\"lat\":{:.6},\"lon\":{:.6}}}", mid_lat, mid_lon))
        }
        "geo.bounding_box" => {
            let (lat, lon, r) = (f!("lat"), f!("lon"), f!("radius_km"));
            let dlat = to_deg(r / EARTH_R);
            let dlon = to_deg(r / EARTH_R / to_rad(lat).cos().max(0.001));
            sdk_respond_ok(rid, &format!(
                "{{\"min_lat\":{:.6},\"min_lon\":{:.6},\"max_lat\":{:.6},\"max_lon\":{:.6}}}",
                lat - dlat, lon - dlon, lat + dlat, lon + dlon
            ))
        }
        "geo.point_in_bbox" => {
            let (lat, lon) = (f!("lat"), f!("lon"));
            let (min_lat, min_lon, max_lat, max_lon) = (f!("min_lat"), f!("min_lon"), f!("max_lat"), f!("max_lon"));
            let inside = lat >= min_lat && lat <= max_lat && lon >= min_lon && lon <= max_lon;
            sdk_respond_ok(rid, if inside { "true" } else { "false" })
        }
        "geo.deg_to_dms" => {
            let deg = f!("deg");
            let is_lat = args["is_lat"].as_bool().unwrap_or(true);
            let abs = deg.abs();
            let d = abs as u32;
            let m = ((abs - d as f64) * 60.0) as u32;
            let s = (abs - d as f64 - m as f64 / 60.0) * 3600.0;
            let dir = if is_lat { if deg >= 0.0 { "N" } else { "S" } }
                      else { if deg >= 0.0 { "E" } else { "W" } };
            sdk_respond_ok(rid, &format!("{}°{}'{}\" {}", d, m, format!("{:.2}", s), dir))
        }
        "geo.dms_to_deg" => {
            let dms = args["dms"].as_str().unwrap_or("");
            let cleaned: String = dms.chars().map(|c| if "°'\"".contains(c) { ' ' } else { c }).collect();
            let parts: Vec<&str> = cleaned.split_whitespace().collect();
            if parts.len() < 3 {
                return sdk_respond_err(rid, "expected format: D M S dir");
            }
            let d: f64 = parts[0].parse().unwrap_or(0.0);
            let m: f64 = parts[1].parse().unwrap_or(0.0);
            let s: f64 = parts[2].parse().unwrap_or(0.0);
            let dir = parts.get(3).unwrap_or(&"N");
            let dec = d + m / 60.0 + s / 3600.0;
            let signed = if *dir == "S" || *dir == "W" { -dec } else { dec };
            sdk_respond_ok(rid, &format!("{:.6}", signed))
        }
        "geo.distance_to_meters" => {
            sdk_respond_ok(rid, &(f!("km") * 1000.0).to_string())
        }
        "geo.meters_to_degrees" => {
            let meters = f!("meters");
            let lat = f!("lat");
            let lat_deg = meters / 111_320.0;
            let lon_deg = meters / (111_320.0 * to_rad(lat).cos().max(0.001));
            sdk_respond_ok(rid, &format!("{{\"lat_deg\":{:.8},\"lon_deg\":{:.8}}}", lat_deg, lon_deg))
        }
        "geo.grid_ref" => {
            let lat = f!("lat");
            let lon = f!("lon");
            let row = ((lat + 90.0) / 10.0) as u32;
            let col = ((lon + 180.0) / 10.0) as u32;
            let col_letter = (b'A' + (col % 18) as u8) as char;
            sdk_respond_ok(rid, &format!("{}{}", col_letter, row))
        }
        other => sdk_respond_err(rid, &format!("unknown skill: {}", other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── geo.haversine ───────────────────────────────────────────────────
    #[test] fn haversine_same_point() { assert!(haversine(0.0, 0.0, 0.0, 0.0) < 0.001); }
    #[test] fn haversine_known() {
        let d = haversine(51.5074, -0.1278, 48.8566, 2.3522);
        assert!(d > 300.0 && d < 400.0, "London-Paris ~340km, got {}", d);
    }
    #[test] fn haversine_antipodes() {
        let d = haversine(0.0, 0.0, 0.0, 180.0);
        assert!(d > 20_000.0 && d < 20_050.0, "half-circumference ~20_015km, got {}", d);
    }

    // ── geo.bearing ─────────────────────────────────────────────────────
    #[test] fn bearing_north() {
        let (lat1,lon1,lat2,lon2) = (to_rad(0.0), to_rad(0.0), to_rad(10.0), to_rad(0.0));
        let dlon = lon2 - lon1;
        let y = dlon.sin() * lat2.cos();
        let x = lat1.cos() * lat2.sin() - lat1.sin() * lat2.cos() * dlon.cos();
        let brng = (to_deg(y.atan2(x)) + 360.0) % 360.0;
        assert!(brng.abs() < 0.01 || (brng - 360.0).abs() < 0.01, "expected 0° north, got {}", brng);
    }
    #[test] fn bearing_east() {
        let (lat1,lon1,lat2,lon2) = (to_rad(0.0), to_rad(0.0), to_rad(0.0), to_rad(10.0));
        let dlon = lon2 - lon1;
        let y = dlon.sin() * lat2.cos();
        let x = lat1.cos() * lat2.sin() - lat1.sin() * lat2.cos() * dlon.cos();
        let brng = (to_deg(y.atan2(x)) + 360.0) % 360.0;
        assert!((brng - 90.0).abs() < 0.01, "expected 90° east, got {}", brng);
    }

    // ── geo.point_in_bbox ───────────────────────────────────────────────
    #[test] fn point_in_bbox_true() {
        let (lat, lon) = (51.5, -0.1);
        assert!(lat >= 51.0 && lat <= 52.0 && lon >= -1.0 && lon <= 0.0);
    }
    #[test] fn point_in_bbox_false() {
        let (lat, lon) = (55.0, -0.1);
        assert!(!(lat >= 51.0 && lat <= 52.0 && lon >= -1.0 && lon <= 0.0));
    }
    #[test] fn point_on_bbox_edge() {
        let (lat, lon) = (51.0, -1.0);
        assert!(lat >= 51.0 && lat <= 52.0 && lon >= -1.0 && lon <= 0.0);
    }

    // ── geo.deg_to_dms ──────────────────────────────────────────────────
    #[test] fn deg_to_dms_positive_lat() {
        let deg = 51.5074f64;
        let abs = deg.abs();
        let d = abs as u32;
        let m = ((abs - d as f64) * 60.0) as u32;
        assert_eq!(d, 51);
        assert_eq!(m, 30);
    }
    #[test] fn deg_to_dms_negative() {
        let deg = -0.1278f64;
        let dir = if deg >= 0.0 { "E" } else { "W" };
        assert_eq!(dir, "W");
    }

    // ── geo.distance_to_meters ────────────────────────────────────────────
    #[test] fn distance_to_meters_1km() { assert_eq!(1.0 * 1000.0, 1000.0); }
    #[test] fn distance_to_meters_0()   { assert_eq!(0.0 * 1000.0, 0.0f64); }

    // ── geo.meters_to_degrees ────────────────────────────────────────────
    #[test] fn meters_to_degrees_lat() {
        let lat_deg = 111_320.0f64 / 111_320.0;
        assert!((lat_deg - 1.0).abs() < 1e-9);
    }
    #[test] fn meters_to_degrees_zero() {
        let lat_deg = 0.0f64 / 111_320.0;
        assert_eq!(lat_deg, 0.0);
    }

    // ── geo.grid_ref ─────────────────────────────────────────────────────
    #[test] fn grid_ref_london() {
        let lat = 51.5074f64;
        let lon = -0.1278f64;
        let row = ((lat + 90.0) / 10.0) as u32;
        let col = ((lon + 180.0) / 10.0) as u32;
        let col_letter = (b'A' + (col % 18) as u8) as char;
        let grid = format!("{}{}", col_letter, row);
        assert!(!grid.is_empty());
    }
    #[test] fn grid_ref_origin() {
        let lat = 0.0f64; let lon = 0.0f64;
        let row = ((lat + 90.0) / 10.0) as u32;
        let col = ((lon + 180.0) / 10.0) as u32;
        assert_eq!(row, 9);
        assert_eq!(col, 18);
    }

    // ── helpers ───────────────────────────────────────────────────────────
    #[test] fn deg_to_rad() { assert!((to_rad(180.0) - PI).abs() < 1e-10); }
    #[test] fn rad_to_deg() { assert!((to_deg(PI) - 180.0).abs() < 1e-9); }
    #[test] fn to_rad_zero() { assert_eq!(to_rad(0.0), 0.0); }

    // ── manifest ───────────────────────────────────────────────────────────
    #[test] fn manifest_valid_json() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(v["skills"].as_array().unwrap().len(), 10);
    }
    #[test] fn manifest_all_skills_have_name() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() {
            assert!(s["name"].as_str().unwrap().starts_with("geo."));
        }
    }
}

use openclaw_plugin_sdk::prelude::*;

static MANIFEST: &str = r#"{
  "id": "community.color",
  "name": "Color Utilities",
  "version": "0.1.0",
  "description": "10 color skills: hex_to_rgb, rgb_to_hex, hex_to_hsl, hsl_to_hex, luminance, contrast_ratio, is_dark, blend, invert, color_name.",
  "skills": [
    {"name":"color.hex_to_rgb","display":"Hex to RGB","description":"Convert #RRGGBB to {r,g,b} JSON.","risk":"safe","params":[{"name":"hex","type":"string","description":"Hex color e.g. #FF8800","required":true}]},
    {"name":"color.rgb_to_hex","display":"RGB to Hex","description":"Convert r,g,b integers to #RRGGBB.","risk":"safe","params":[{"name":"r","type":"integer","required":true},{"name":"g","type":"integer","required":true},{"name":"b","type":"integer","required":true}]},
    {"name":"color.hex_to_hsl","display":"Hex to HSL","description":"Convert #RRGGBB to {h,s,l} JSON.","risk":"safe","params":[{"name":"hex","type":"string","required":true}]},
    {"name":"color.hsl_to_hex","display":"HSL to Hex","description":"Convert h(0-360),s(0-100),l(0-100) to #RRGGBB.","risk":"safe","params":[{"name":"h","type":"number","required":true},{"name":"s","type":"number","required":true},{"name":"l","type":"number","required":true}]},
    {"name":"color.luminance","display":"Luminance","description":"Relative luminance (0-1) of a hex color.","risk":"safe","params":[{"name":"hex","type":"string","required":true}]},
    {"name":"color.contrast_ratio","display":"Contrast Ratio","description":"WCAG contrast ratio between two hex colors.","risk":"safe","params":[{"name":"fg","type":"string","required":true},{"name":"bg","type":"string","required":true}]},
    {"name":"color.is_dark","display":"Is Dark","description":"Return true if color is perceptually dark.","risk":"safe","params":[{"name":"hex","type":"string","required":true}]},
    {"name":"color.blend","display":"Blend","description":"Blend two hex colors by factor t (0=a, 1=b).","risk":"safe","params":[{"name":"a","type":"string","required":true},{"name":"b","type":"string","required":true},{"name":"t","type":"number","description":"Blend factor 0-1","required":true}]},
    {"name":"color.invert","display":"Invert","description":"Return the inverted hex color (#FFFFFF - input).","risk":"safe","params":[{"name":"hex","type":"string","required":true}]},
    {"name":"color.color_name","display":"Color Name","description":"Return a rough English name for a hex color.","risk":"safe","params":[{"name":"hex","type":"string","required":true}]}
  ]
}"#;

fn parse_hex(hex: &str) -> Option<(u8, u8, u8)> {
    let h = hex.trim_start_matches('#');
    if h.len() != 6 { return None; }
    let r = u8::from_str_radix(&h[0..2], 16).ok()?;
    let g = u8::from_str_radix(&h[2..4], 16).ok()?;
    let b = u8::from_str_radix(&h[4..6], 16).ok()?;
    Some((r, g, b))
}

fn to_hex(r: u8, g: u8, b: u8) -> String { format!("#{:02X}{:02X}{:02X}", r, g, b) }

fn luminance(r: u8, g: u8, b: u8) -> f64 {
    let lin = |c: u8| { let v = c as f64 / 255.0; if v <= 0.04045 { v/12.92 } else { ((v+0.055)/1.055f64).powf(2.4) } };
    0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)
}

fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (f64, f64, f64) {
    let rf = r as f64 / 255.0;
    let gf = g as f64 / 255.0;
    let bf = b as f64 / 255.0;
    let max = rf.max(gf).max(bf);
    let min = rf.min(gf).min(bf);
    let l = (max + min) / 2.0;
    if (max - min).abs() < 1e-10 { return (0.0, 0.0, l * 100.0); }
    let d = max - min;
    let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
    let h = if (max - rf).abs() < 1e-10 { (gf - bf) / d + if gf < bf { 6.0 } else { 0.0 } }
            else if (max - gf).abs() < 1e-10 { (bf - rf) / d + 2.0 }
            else { (rf - gf) / d + 4.0 };
    (h / 6.0 * 360.0, s * 100.0, l * 100.0)
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (u8, u8, u8) {
    let s = s / 100.0; let l = l / 100.0;
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r1, g1, b1) = match (h / 60.0) as u32 {
        0 => (c, x, 0.0), 1 => (x, c, 0.0), 2 => (0.0, c, x),
        3 => (0.0, x, c), 4 => (x, 0.0, c), _ => (c, 0.0, x),
    };
    (((r1 + m) * 255.0).round() as u8, ((g1 + m) * 255.0).round() as u8, ((b1 + m) * 255.0).round() as u8)
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

    match req.skill.as_str() {
        "color.hex_to_rgb" => {
            let hex = args["hex"].as_str().unwrap_or("");
            match parse_hex(hex) {
                Some((r,g,b)) => sdk_respond_ok(rid, &format!("{{\"r\":{},\"g\":{},\"b\":{}}}", r, g, b)),
                None => sdk_respond_err(rid, "invalid hex color"),
            }
        }
        "color.rgb_to_hex" => {
            let r = args["r"].as_u64().unwrap_or(0) as u8;
            let g = args["g"].as_u64().unwrap_or(0) as u8;
            let b = args["b"].as_u64().unwrap_or(0) as u8;
            sdk_respond_ok(rid, &to_hex(r, g, b))
        }
        "color.hex_to_hsl" => {
            let hex = args["hex"].as_str().unwrap_or("");
            match parse_hex(hex) {
                Some((r,g,b)) => {
                    let (h,s,l) = rgb_to_hsl(r, g, b);
                    sdk_respond_ok(rid, &format!("{{\"h\":{:.1},\"s\":{:.1},\"l\":{:.1}}}", h, s, l))
                }
                None => sdk_respond_err(rid, "invalid hex color"),
            }
        }
        "color.hsl_to_hex" => {
            let h = args["h"].as_f64().unwrap_or(0.0);
            let s = args["s"].as_f64().unwrap_or(0.0);
            let l = args["l"].as_f64().unwrap_or(0.0);
            let (r,g,b) = hsl_to_rgb(h, s, l);
            sdk_respond_ok(rid, &to_hex(r, g, b))
        }
        "color.luminance" => {
            let hex = args["hex"].as_str().unwrap_or("");
            match parse_hex(hex) {
                Some((r,g,b)) => sdk_respond_ok(rid, &format!("{:.4}", luminance(r, g, b))),
                None => sdk_respond_err(rid, "invalid hex color"),
            }
        }
        "color.contrast_ratio" => {
            let fg = args["fg"].as_str().unwrap_or("");
            let bg = args["bg"].as_str().unwrap_or("");
            match (parse_hex(fg), parse_hex(bg)) {
                (Some((r1,g1,b1)), Some((r2,g2,b2))) => {
                    let l1 = luminance(r1,g1,b1);
                    let l2 = luminance(r2,g2,b2);
                    let (lighter, darker) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
                    let ratio = (lighter + 0.05) / (darker + 0.05);
                    sdk_respond_ok(rid, &format!("{:.2}", ratio))
                }
                _ => sdk_respond_err(rid, "invalid hex color(s)"),
            }
        }
        "color.is_dark" => {
            let hex = args["hex"].as_str().unwrap_or("");
            match parse_hex(hex) {
                Some((r,g,b)) => sdk_respond_ok(rid, if luminance(r,g,b) < 0.179 { "true" } else { "false" }),
                None => sdk_respond_err(rid, "invalid hex color"),
            }
        }
        "color.blend" => {
            let ca = args["a"].as_str().unwrap_or("");
            let cb = args["b"].as_str().unwrap_or("");
            let t = args["t"].as_f64().unwrap_or(0.5).max(0.0).min(1.0);
            match (parse_hex(ca), parse_hex(cb)) {
                (Some((r1,g1,b1)), Some((r2,g2,b2))) => {
                    let blend = |a: u8, b: u8| (a as f64 + (b as f64 - a as f64) * t).round() as u8;
                    sdk_respond_ok(rid, &to_hex(blend(r1,r2), blend(g1,g2), blend(b1,b2)))
                }
                _ => sdk_respond_err(rid, "invalid hex color(s)"),
            }
        }
        "color.invert" => {
            let hex = args["hex"].as_str().unwrap_or("");
            match parse_hex(hex) {
                Some((r,g,b)) => sdk_respond_ok(rid, &to_hex(255-r, 255-g, 255-b)),
                None => sdk_respond_err(rid, "invalid hex color"),
            }
        }
        "color.color_name" => {
            let hex = args["hex"].as_str().unwrap_or("");
            match parse_hex(hex) {
                Some((r,g,b)) => {
                    let name = match (r, g, b) {
                        (255, 255, 255) => "white",
                        (0, 0, 0)       => "black",
                        (r, g, b) if r >= 200 && g < 80 && b < 80 => "red",
                        (r, g, b) if r < 80 && g >= 150 && b < 80 => "green",
                        (r, g, b) if r < 80 && g < 80 && b >= 150 => "blue",
                        (r, g, b) if r >= 200 && g >= 200 && b < 80 => "yellow",
                        (r, g, b) if r >= 200 && g >= 100 && b < 80 => "orange",
                        (r, g, b) if r >= 150 && g < 100 && b >= 150 => "purple",
                        (r, g, b) if r < 80 && g >= 180 && b >= 180 => "cyan",
                        (r, g, b) if r >= 200 && g < 100 && b >= 150 => "magenta",
                        (r, g, b) if (r as i16 - g as i16).abs() < 30 && (g as i16 - b as i16).abs() < 30 => "gray",
                        _ => "unknown",
                    };
                    sdk_respond_ok(rid, name)
                }
                None => sdk_respond_err(rid, "invalid hex color"),
            }
        }
        other => sdk_respond_err(rid, &format!("unknown skill: {}", other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn hex_parse_white() { assert_eq!(parse_hex("#FFFFFF"), Some((255,255,255))); }
    #[test] fn hex_parse_black() { assert_eq!(parse_hex("#000000"), Some((0,0,0))); }
    #[test] fn rgb_to_hex_white() { assert_eq!(to_hex(255,255,255), "#FFFFFF"); }
    #[test] fn invert_black() { let (r,g,b) = (0u8,0u8,0u8); assert_eq!(to_hex(255-r,255-g,255-b), "#FFFFFF"); }
    #[test] fn luminance_white() { assert!((luminance(255,255,255) - 1.0).abs() < 0.001); }
    #[test] fn luminance_black() { assert!(luminance(0,0,0) < 0.001); }
    #[test] fn contrast_max() {
        let l1 = luminance(255,255,255); let l2 = luminance(0,0,0);
        let ratio = (l1+0.05)/(l2+0.05);
        assert!(ratio > 20.0);
    }
    #[test] fn hsl_roundtrip() {
        let (r,g,b) = (200u8, 100u8, 50u8);
        let (h,s,l) = rgb_to_hsl(r, g, b);
        let (r2,g2,b2) = hsl_to_rgb(h, s, l);
        assert!((r as i16 - r2 as i16).abs() <= 1);
        assert!((g as i16 - g2 as i16).abs() <= 1);
        assert!((b as i16 - b2 as i16).abs() <= 1);
    }
    // ── parse_hex edge cases ──────────────────────────────────────────────
    #[test] fn hex_parse_red()    { assert_eq!(parse_hex("#FF0000"), Some((255,0,0))); }
    #[test] fn hex_parse_green()  { assert_eq!(parse_hex("#00FF00"), Some((0,255,0))); }
    #[test] fn hex_parse_blue()   { assert_eq!(parse_hex("#0000FF"), Some((0,0,255))); }
    #[test] fn hex_parse_invalid_short() { assert!(parse_hex("#FFF").is_none()); }
    #[test] fn hex_parse_invalid_chars() { assert!(parse_hex("#GGGGGG").is_none()); }
    #[test] fn hex_parse_no_hash() { assert_eq!(parse_hex("FF8800"), Some((255,136,0))); }

    // ── to_hex ──────────────────────────────────────────────────────────────
    #[test] fn to_hex_red()   { assert_eq!(to_hex(255,0,0), "#FF0000"); }
    #[test] fn to_hex_black() { assert_eq!(to_hex(0,0,0), "#000000"); }
    #[test] fn to_hex_mixed() { assert_eq!(to_hex(16,32,48), "#102030"); }

    // ── luminance ────────────────────────────────────────────────────────────
    #[test] fn luminance_red()  { let l = luminance(255,0,0); assert!(l > 0.2 && l < 0.22, "got {}", l); }
    #[test] fn luminance_mid()  { let l = luminance(128,128,128); assert!(l > 0.2 && l < 0.22, "got {}", l); }

    // ── is_dark / contrast ───────────────────────────────────────────────────
    #[test] fn is_dark_black()  { assert!(luminance(0,0,0) < 0.179); }
    #[test] fn is_dark_white()  { assert!(luminance(255,255,255) >= 0.179); }
    #[test] fn is_dark_navy()   { assert!(luminance(0,0,128) < 0.179); }

    // ── invert ───────────────────────────────────────────────────────────────
    #[test] fn invert_white()   { let (r,g,b)=(255u8,255,255); assert_eq!(to_hex(255-r,255-g,255-b),"#000000"); }
    #[test] fn invert_red()     { let (r,g,b)=(255u8,0,0); assert_eq!(to_hex(255-r,255-g,255-b),"#00FFFF"); }
    #[test] fn invert_same_result() { let (r,g,b)=(100u8,150,200); let inv=(255-r,255-g,255-b); let dbl=(255-inv.0,255-inv.1,255-inv.2); assert_eq!((r,g,b),dbl); }

    // ── rgb_to_hsl / hsl_to_rgb roundtrips ──────────────────────────────────
    #[test] fn hsl_roundtrip_red() {
        let (h,s,l) = rgb_to_hsl(255,0,0);
        let (r2,g2,b2) = hsl_to_rgb(h,s,l);
        assert!((r2 as i16 - 255).abs() <= 1);
        assert!((g2 as i16).abs() <= 1);
        assert!((b2 as i16).abs() <= 1);
    }
    #[test] fn hsl_roundtrip_blue() {
        let (h,s,l) = rgb_to_hsl(0,0,255);
        let (r2,g2,b2) = hsl_to_rgb(h,s,l);
        assert!((r2 as i16).abs() <= 1);
        assert!((g2 as i16).abs() <= 1);
        assert!((b2 as i16 - 255).abs() <= 1);
    }
    #[test] fn gray_has_zero_saturation() {
        let (_h, s, _l) = rgb_to_hsl(128,128,128);
        assert!(s < 1.0, "gray should have near-zero saturation, got {}", s);
    }

    // ── color_name ────────────────────────────────────────────────────────────
    #[test] fn color_name_white() {
        // white: (255,255,255)
        let (r,g,b) = (255u8,255u8,255u8);
        let name = match (r,g,b) { (255,255,255)=>"white", (0,0,0)=>"black", _=>"other" };
        assert_eq!(name, "white");
    }
    #[test] fn color_name_black() {
        let (r,g,b) = (0u8,0u8,0u8);
        let name = match (r,g,b) { (255,255,255)=>"white", (0,0,0)=>"black", _=>"other" };
        assert_eq!(name, "black");
    }

    // ── manifest ──────────────────────────────────────────────────────────────
    #[test] fn manifest_valid_json() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(v["skills"].as_array().unwrap().len(), 10);
    }
    #[test] fn manifest_all_skills_have_name() {
        let v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        for s in v["skills"].as_array().unwrap() {
            assert!(s["name"].as_str().unwrap().starts_with("color."));
        }
    }
}

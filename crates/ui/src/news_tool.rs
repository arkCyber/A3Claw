//! 新闻获取工具 - 为 Claw Terminal 提供实时新闻功能

use serde_json::json;

/// 获取实时新闻 - 支持多语言、多来源
pub async fn fetch_news(query: &str, count: usize) -> Result<String, String> {
    tracing::info!("[NEWS] Starting news fetch, query='{}', count={}", query, count);
    
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build() {
            Ok(c) => {
                tracing::debug!("[NEWS] HTTP client created successfully");
                c
            }
            Err(e) => {
                tracing::error!("[NEWS] Failed to create HTTP client: {}", e);
                return Err(format!("创建 HTTP 客户端失败: {}", e));
            }
        };

    // 根据查询内容智能选择新闻源
    let sources = select_news_sources(query);
    tracing::info!("[NEWS] Selected {} news sources based on query", sources.len());
    
    let mut all_news = Vec::new();
    let mut successful_sources = Vec::new();
    
    // 尝试从多个来源获取新闻
    for (url, name, lang) in sources {
        tracing::info!("[NEWS] Trying source: {} ({}) [{}]", name, url, lang);
        match fetch_news_from_url(&client, url, name, count).await {
            Ok(news) => {
                tracing::info!("[NEWS] Successfully fetched news from {}", name);
                all_news.push((name.to_string(), news, lang.to_string()));
                successful_sources.push(name);
                
                // 如果已经获取到足够的来源，可以提前返回
                if successful_sources.len() >= 2 {
                    break;
                }
            }
            Err(e) => {
                tracing::warn!("[NEWS] {} failed: {}", name, e);
                continue;
            }
        }
    }
    
    if all_news.is_empty() {
        tracing::error!("[NEWS] All news sources failed");
        return Err("所有新闻源都失败了，请稍后重试".to_string());
    }
    
    // 格式化多来源新闻
    format_multi_source_news(&all_news, query)
}

/// 根据查询内容智能选择新闻源
fn select_news_sources(query: &str) -> Vec<(&'static str, &'static str, &'static str)> {
    let query_lower = query.to_lowercase();
    let mut sources = Vec::new();
    
    // 检测语言和地区
    let is_chinese = query.chars().any(|c| (c as u32) > 0x4E00 && (c as u32) < 0x9FA5);
    let is_german = query_lower.contains("德国") || query_lower.contains("germany") || query_lower.contains("german");
    let is_us = query_lower.contains("美国") || query_lower.contains("us") || query_lower.contains("america");
    let is_uk = query_lower.contains("英国") || query_lower.contains("uk") || query_lower.contains("britain");
    
    // 优先添加相关地区的新闻源
    if is_german {
        sources.push(("https://www.dw.com/en/top-stories/s-9097", "Deutsche Welle (DW)", "en"));
        sources.push(("https://www.thelocal.de/", "The Local Germany", "en"));
    }
    
    if is_chinese || (!is_german && !is_us && !is_uk) {
        // 中文用户或默认情况，添加国际新闻源
        sources.push(("https://lite.cnn.com/", "CNN Lite", "en"));
        sources.push(("https://text.npr.org/", "NPR Text", "en"));
    }
    
    if is_us {
        sources.push(("https://lite.cnn.com/", "CNN Lite", "en"));
        sources.push(("https://text.npr.org/", "NPR Text", "en"));
    }
    
    if is_uk {
        sources.push(("https://www.bbc.com/news", "BBC News", "en"));
    }
    
    // 总是添加 Reuters 作为备选
    sources.push(("https://www.reuters.com/", "Reuters", "en"));
    
    // 如果没有匹配到特定来源，使用默认国际新闻源
    if sources.is_empty() {
        sources.push(("https://lite.cnn.com/", "CNN Lite", "en"));
        sources.push(("https://text.npr.org/", "NPR Text", "en"));
        sources.push(("https://www.reuters.com/", "Reuters", "en"));
    }
    
    sources
}

/// 格式化多来源新闻
fn format_multi_source_news(
    news_list: &[(String, String, String)],
    query: &str,
) -> Result<String, String> {
    let mut result = String::new();
    let now = chrono::Utc::now();
    let time_str = now.format("%Y-%m-%d %H:%M").to_string();
    
    // 添加标题，显示用户的查询
    result.push_str(&format!("\n📰 {} - 新闻汇总（{}）\n", query, time_str));
    result.push_str(&format!("\n✅ 成功获取 {} 个新闻源\n", news_list.len()));
    
    // 显示每个来源的新闻
    for (i, (source_name, news_content, _lang)) in news_list.iter().enumerate() {
        result.push_str(&format!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n"));
        result.push_str(&format!("📌 来源 {}: {}\n", i + 1, source_name));
        result.push_str(&format!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n"));
        result.push_str(news_content);
        result.push_str("\n");
    }
    
    Ok(result)
}

async fn fetch_news_from_url(
    client: &reqwest::Client,
    url: &str,
    source_name: &str,
    count: usize,
) -> Result<String, String> {
    tracing::debug!("[NEWS] Fetching from URL: {}", url);
    
    let resp = match client.get(url).send().await {
        Ok(r) => {
            tracing::debug!("[NEWS] HTTP request successful, status: {}", r.status());
            r
        }
        Err(e) => {
            tracing::error!("[NEWS] HTTP request failed for {}: {}", source_name, e);
            return Err(format!("HTTP 请求失败: {}", e));
        }
    };

    if !resp.status().is_success() {
        let status = resp.status();
        tracing::warn!("[NEWS] Non-success HTTP status from {}: {}", source_name, status);
        return Err(format!("HTTP 状态码: {}", status));
    }

    let body = match resp.text().await {
        Ok(b) => {
            tracing::debug!("[NEWS] Response body received, length: {} bytes", b.len());
            b
        }
        Err(e) => {
            tracing::error!("[NEWS] Failed to read response body from {}: {}", source_name, e);
            return Err(format!("读取响应失败: {}", e));
        }
    };

    parse_news_html(&body, source_name, count)
}

fn parse_news_html(html: &str, source_name: &str, count: usize) -> Result<String, String> {
    tracing::debug!("[NEWS] Parsing HTML from {}, length: {} bytes", source_name, html.len());
    
    let mut news_items = Vec::new();
    
    // 提取新闻项（标题 + 摘要）
    extract_news_items(html, &mut news_items);
    
    tracing::debug!("[NEWS] Extracted {} news items from {}", news_items.len(), source_name);
    
    if news_items.is_empty() {
        tracing::warn!("[NEWS] No news items extracted from {}", source_name);
        return Err("未能从 HTML 中提取到新闻".to_string());
    }
    
    // 格式化输出
    let mut result = String::new();
    let now = chrono::Utc::now();
    let time_str = now.format("%Y-%m-%d %H:%M").to_string();
    
    result.push_str(&format!("\n📰 来自 {} 的最新新闻（{}）：\n", source_name, time_str));
    
    let mut added = 0;
    for (title, summary) in news_items.iter().take(count) {
        added += 1;
        
        // 使用简单格式，不用 ** 包裹（避免 Markdown 解析问题）
        result.push_str(&format!(
            "\n{}. {}\n",
            added,
            title
        ));
        
        if !summary.is_empty() {
            result.push_str(&format!("   {}\n", summary));
        }
    }
    
    if result.is_empty() {
        Err(format!("从 {} 未能提取到有效新闻", source_name))
    } else {
        Ok(result)
    }
}

fn extract_news_items(html: &str, items: &mut Vec<(String, String)>) {
    // 尝试提取 <article> 标签（现代新闻网站常用）
    let mut pos = 0;
    while pos < html.len() && items.len() < 20 {
        if let Some(article_offset) = html.get(pos..).and_then(|s| s.find("<article")) {
            let article_start = match pos.checked_add(article_offset) {
                Some(start) if start < html.len() => start,
                _ => break,
            };
            
            if let Some(end_offset) = html.get(article_start..).and_then(|s| s.find("</article>")) {
                let article_end = match article_start.checked_add(end_offset).and_then(|e| e.checked_add(10)) {
                    Some(end) if end <= html.len() => end,
                    _ => break,
                };
                
                if let Some(article_html) = html.get(article_start..article_end) {
                    if let Some((title, summary)) = extract_article_content(article_html) {
                        items.push((title, summary));
                    }
                }
                
                pos = article_end;
            } else {
                break;
            }
        } else {
            break;
        }
    }
    
    // 如果没有找到 article，尝试提取 h1-h4 + 相邻的 p 标签
    if items.is_empty() {
        extract_headlines_with_paragraphs(html, items);
    }
}

fn extract_article_content(article_html: &str) -> Option<(String, String)> {
    // 提取标题（h1, h2, h3 或 a 标签）
    let title = extract_first_headline(article_html)?;
    
    // 提取摘要（p 标签）
    let summary = extract_first_paragraph(article_html).unwrap_or_default();
    
    Some((clean_html(&title), clean_html(&summary)))
}

fn extract_first_headline(html: &str) -> Option<String> {
    for tag in &["<h1", "<h2", "<h3", "<h4", "<a "] {
        let close_tag = if tag.starts_with("<a") { "</a>" } else { &tag.replace("<", "</") };
        
        if let Some(start) = html.find(tag) {
            if let Some(end_offset) = html.get(start..)?.find(close_tag) {
                let end = start.checked_add(end_offset)?;
                if end > html.len() {
                    continue;
                }
                let content = html.get(start..end)?;
                if let Some(text_start) = content.find('>') {
                    let text = content.get(text_start.checked_add(1)?..)?;
                    let cleaned = clean_html(text).trim().to_string();
                    if cleaned.len() > 10 && cleaned.len() < 200 {
                        return Some(cleaned);
                    }
                }
            }
        }
    }
    None
}

fn extract_first_paragraph(html: &str) -> Option<String> {
    let p_start = html.find("<p")?;
    let p_end_offset = html.get(p_start..)?.find("</p>")?;
    let p_end = p_start.checked_add(p_end_offset)?;
    
    if p_end > html.len() {
        return None;
    }
    
    let content = html.get(p_start..p_end)?;
    let text_start = content.find('>')?;
    let text = content.get(text_start.checked_add(1)?..)?;
    let cleaned = clean_html(text).trim().to_string();
    
    if cleaned.len() > 20 && cleaned.len() < 500 {
        Some(truncate_text(&cleaned, 200))
    } else {
        None
    }
}

fn extract_headlines_with_paragraphs(html: &str, items: &mut Vec<(String, String)>) {
    let mut titles = Vec::new();
    extract_headlines(html, &mut titles);
    
    for title in titles.iter().take(10) {
        let cleaned = clean_html(title).trim().to_string();
        
        if cleaned.len() < 15 || cleaned.len() > 200 {
            continue;
        }
        if cleaned.contains("http") || cleaned.contains("www.") {
            continue;
        }
        if cleaned.contains("©") || cleaned.contains("®") {
            continue;
        }
        
        items.push((cleaned, String::new()));
        
        if items.len() >= 10 {
            break;
        }
    }
}

fn extract_headlines(html: &str, titles: &mut Vec<String>) {
    // 提取 <h1>, <h2>, <h3> 标签
    for tag in &["<h1", "<h2", "<h3", "<h4"] {
        let close_tag = tag.replace("<", "</");
        let mut pos = 0;
        while pos < html.len() {
            if let Some(offset) = html.get(pos..).and_then(|s| s.find(tag)) {
                let start = match pos.checked_add(offset) {
                    Some(s) if s < html.len() => s,
                    _ => break,
                };
                
                if let Some(end_offset) = html.get(start..).and_then(|s| s.find(&close_tag)) {
                    let end = match start.checked_add(end_offset) {
                        Some(e) if e <= html.len() => e,
                        _ => break,
                    };
                    
                    if let Some(content) = html.get(start..end) {
                        if let Some(text_start) = content.find('>') {
                            if let Some(text) = content.get(text_start.checked_add(1).unwrap_or(content.len())..) {
                                let cleaned = clean_html(text).trim().to_string();
                                if !cleaned.is_empty() && cleaned.len() > 10 {
                                    titles.push(cleaned);
                                }
                            }
                        }
                    }
                    pos = end;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }
    
    // 提取 <a> 标签中的文本（作为备用）
    let mut pos = 0;
    while pos < html.len() && titles.len() < 50 {
        if let Some(offset) = html.get(pos..).and_then(|s| s.find("<a ")) {
            let a_start = match pos.checked_add(offset) {
                Some(s) if s < html.len() => s,
                _ => break,
            };
            
            if let Some(end_offset) = html.get(a_start..).and_then(|s| s.find("</a>")) {
                let a_end = match a_start.checked_add(end_offset) {
                    Some(e) if e <= html.len() => e,
                    _ => break,
                };
                
                if let Some(a_tag) = html.get(a_start..a_end) {
                    if let Some(text_start) = a_tag.rfind('>') {
                        if let Some(text) = a_tag.get(text_start.checked_add(1).unwrap_or(a_tag.len())..) {
                            let cleaned = clean_html(text).trim().to_string();
                            if cleaned.len() > 15 && !cleaned.contains("http") {
                                titles.push(cleaned);
                            }
                        }
                    }
                }
                pos = a_end;
            } else {
                break;
            }
        } else {
            break;
        }
    }
}

fn parse_and_format_rss(xml: &str, count: usize) -> Result<String, String> {
    let mut result = String::new();
    let mut found = 0;
    let mut pos = 0;

    while found < count && pos < xml.len() {
        if let Some(item_start) = xml[pos..].find("<item>").or_else(|| xml[pos..].find("<item ")) {
            let item_start = pos + item_start;
            if let Some(item_end) = xml[item_start..].find("</item>") {
                let item_end = item_start + item_end + 7;
                let item_xml = &xml[item_start..item_end];

                if let Some((title, pub_date, description)) = parse_rss_item(item_xml) {
                    found += 1;
                    result.push_str(&format!(
                        "\n{}. [{}] {}\n摘要：{}\n",
                        found,
                        pub_date,
                        title,
                        description
                    ));
                }

                pos = item_end;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    if result.is_empty() {
        Err("未能解析到任何新闻".to_string())
    } else {
        Ok(result)
    }
}

fn parse_rss_item(item_xml: &str) -> Option<(String, String, String)> {
    let title = extract_xml_tag(item_xml, "title")?;
    let description = extract_xml_tag(item_xml, "description")
        .or_else(|| extract_xml_tag(item_xml, "summary"))
        .unwrap_or_else(|| "无摘要".to_string());
    
    let pub_date = extract_xml_tag(item_xml, "pubDate")
        .or_else(|| extract_xml_tag(item_xml, "published"))
        .map(|d| format_date(&d))
        .unwrap_or_else(|| {
            let now = chrono::Utc::now();
            now.format("%Y-%m-%d %H:%M").to_string()
        });

    Some((
        clean_html(&title),
        pub_date,
        clean_html(&truncate_text(&description, 150))
    ))
}

fn extract_xml_tag(xml: &str, tag: &str) -> Option<String> {
    let open_tag = format!("<{}>", tag);
    let close_tag = format!("</{}>", tag);
    
    let start = xml.find(&open_tag)? + open_tag.len();
    let end = xml[start..].find(&close_tag)?;
    
    Some(xml[start..start + end].trim().to_string())
}

fn clean_html(text: &str) -> String {
    let no_cdata = text
        .replace("<![CDATA[", "")
        .replace("]]>", "");
    
    let mut result = String::new();
    let mut in_tag = false;
    for c in no_cdata.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(c),
            _ => {}
        }
    }
    
    result
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
        .trim()
        .to_string()
}

fn truncate_text(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        text.to_string()
    } else {
        let truncated: String = text.chars().take(max_chars).collect();
        format!("{}...", truncated)
    }
}

fn format_date(date_str: &str) -> String {
    use chrono::DateTime;
    
    if let Ok(dt) = DateTime::parse_from_rfc2822(date_str) {
        return dt.format("%Y-%m-%d %H:%M").to_string();
    }
    
    if let Ok(dt) = DateTime::parse_from_rfc3339(date_str) {
        return dt.format("%Y-%m-%d %H:%M").to_string();
    }
    
    date_str.to_string()
}

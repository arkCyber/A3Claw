//! Simple Markdown parser for Claw Terminal rich text display

/// Text span with formatting info
#[derive(Debug, Clone)]
pub struct TextSpan {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub code: bool,
    pub is_list_item: bool,
    pub indent_level: usize,
}

/// Parse a line of text with Markdown formatting
/// Supports: **bold**, *italic*, `code`, lists (*, +, -)
pub fn parse_markdown_line(line: &str) -> Vec<TextSpan> {
    // Safety check: empty or very long lines
    if line.is_empty() {
        return vec![TextSpan {
            text: String::new(),
            bold: false,
            italic: false,
            code: false,
            is_list_item: false,
            indent_level: 0,
        }];
    }
    
    if line.len() > 10000 {
        tracing::warn!("[MARKDOWN] Line too long ({} chars), truncating", line.len());
        return vec![TextSpan {
            text: line.chars().take(10000).collect(),
            bold: false,
            italic: false,
            code: false,
            is_list_item: false,
            indent_level: 0,
        }];
    }
    
    // Check for list items
    let trimmed = line.trim_start();
    let indent_level = (line.len() - trimmed.len()) / 2; // 2 spaces = 1 indent level
    
    // Check for bullet list (* + - •)
    let is_bullet_list = trimmed.starts_with("* ") || 
                         trimmed.starts_with("+ ") || 
                         trimmed.starts_with("- ") ||
                         trimmed.starts_with("• ");
    
    // Check for numbered list (1. 2. 3. etc.)
    let is_numbered_list = {
        let chars: Vec<char> = trimmed.chars().collect();
        if chars.is_empty() {
            false
        } else {
            let first_is_digit = chars[0].is_ascii_digit();
            if !first_is_digit {
                false
            } else {
                // Find where digits end
                let digit_count = chars.iter().take_while(|c| c.is_ascii_digit()).count();
                // Check if followed by ". "
                if digit_count > 0 && digit_count + 1 < chars.len() {
                    chars[digit_count] == '.' && 
                    (digit_count + 2 > chars.len() || chars.get(digit_count + 1) == Some(&' '))
                } else {
                    false
                }
            }
        }
    };
    
    let is_list_item = is_bullet_list || is_numbered_list;
    
    // Remove list marker if present
    let content = if is_bullet_list {
        trimmed.chars().skip(2).collect::<String>()
    } else if is_numbered_list {
        // Skip number and ". "
        let skip_count = trimmed.chars().take_while(|c| c.is_ascii_digit()).count() + 2;
        trimmed.chars().skip(skip_count).collect::<String>()
    } else {
        line.to_string()
    };
    
    let mut spans = Vec::new();
    let mut current_text = String::new();
    let mut in_bold = false;
    let mut in_italic = false;
    let mut in_code = false;
    
    let mut i = 0;
    let chars: Vec<char> = content.chars().collect();
    
    while i < chars.len() {
        if i > chars.len() {
            break;
        }
        
        // Check for ` (code)
        if chars[i] == '`' {
            if !current_text.is_empty() {
                spans.push(TextSpan {
                    text: current_text.clone(),
                    bold: in_bold,
                    italic: in_italic,
                    code: in_code,
                    is_list_item,
                    indent_level,
                });
                current_text.clear();
            }
            in_code = !in_code;
            i += 1;
            continue;
        }
        
        // Check for ** (bold)
        if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '*' {
            if !current_text.is_empty() {
                spans.push(TextSpan {
                    text: current_text.clone(),
                    bold: in_bold,
                    italic: in_italic,
                    code: in_code,
                    is_list_item,
                    indent_level,
                });
                current_text.clear();
            }
            in_bold = !in_bold;
            i += 2;
            continue;
        }
        
        // Check for * (italic) - only if not in bold
        if chars[i] == '*' && !in_bold {
            if !current_text.is_empty() {
                spans.push(TextSpan {
                    text: current_text.clone(),
                    bold: in_bold,
                    italic: in_italic,
                    code: in_code,
                    is_list_item,
                    indent_level,
                });
                current_text.clear();
            }
            in_italic = !in_italic;
            i += 1;
            continue;
        }
        
        current_text.push(chars[i]);
        i += 1;
    }
    
    // Add remaining text
    if !current_text.is_empty() {
        spans.push(TextSpan {
            text: current_text,
            bold: in_bold,
            italic: in_italic,
            code: in_code,
            is_list_item,
            indent_level,
        });
    }
    
    // If no formatting found, return single plain span
    if spans.is_empty() {
        spans.push(TextSpan {
            text: content,
            bold: false,
            italic: false,
            code: false,
            is_list_item,
            indent_level,
        });
    }
    
    // Debug logging
    tracing::debug!("[MARKDOWN] Parsed line: '{}'", line);
    tracing::debug!("[MARKDOWN] Result: {} spans (list: {}, indent: {})", 
                    spans.len(), is_list_item, indent_level);
    for (i, span) in spans.iter().enumerate() {
        tracing::debug!("[MARKDOWN]   Span {}: text='{}', bold={}, italic={}, code={}", 
                        i, span.text, span.bold, span.italic, span.code);
    }
    
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_formatting() {
        let spans = parse_markdown_line("Plain text");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "Plain text");
        assert!(!spans[0].bold);
        assert!(!spans[0].italic);
        assert!(!spans[0].code);
    }

    #[test]
    fn test_bold_text() {
        let spans = parse_markdown_line("This is **bold** text");
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].text, "This is ");
        assert!(!spans[0].bold);
        assert_eq!(spans[1].text, "bold");
        assert!(spans[1].bold);
        assert_eq!(spans[2].text, " text");
        assert!(!spans[2].bold);
    }

    #[test]
    fn test_list_item() {
        let spans = parse_markdown_line("* List item");
        assert!(spans[0].is_list_item);
        assert_eq!(spans[0].text, "List item");
    }

    #[test]
    fn test_code_text() {
        let spans = parse_markdown_line("This is `code` text");
        assert_eq!(spans.len(), 3);
        assert!(!spans[0].code);
        assert!(spans[1].code);
        assert_eq!(spans[1].text, "code");
    }

    #[test]
    fn test_italic_text() {
        let spans = parse_markdown_line("This is *italic* text");
        assert_eq!(spans.len(), 3);
        assert!(!spans[0].italic);
        assert!(spans[1].italic);
        assert_eq!(spans[1].text, "italic");
    }
}

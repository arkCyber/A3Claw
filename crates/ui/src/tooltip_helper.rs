use cosmic::iced::widget::{tooltip, container, Space};
use cosmic::iced::{Alignment, Length, Border, Background, Color};
use cosmic::widget;
use cosmic::Element;
use crate::app::AppMessage;
use crate::theme::Language;

/// Create a triangle arrow that looks like a proper speech-bubble pointer.
///
/// Strategy: render the Unicode solid triangle in `border_color` (golden).
/// The container behind it is `bg_color`, so the base of the triangle blends
/// seamlessly into the bubble when spacing(0) is used.
fn create_triangle_arrow<'a>(
    position: TooltipPosition,
    style: BubbleStyle,
) -> Element<'a, AppMessage> {
    // Use softer, rounder pointer glyphs: ◆ (diamond) is less sharp than ▲.
    // These feel gentler and blend better with rounded bubble corners.
    let (arrow_char, w, h, font_sz) = match position {
        TooltipPosition::Bottom => ("\u{25c6}", 18u16, 14u16, 12u16), // ◆ diamond above
        TooltipPosition::Top    => ("\u{25c6}", 18u16, 14u16, 12u16), // ◆ diamond below
        TooltipPosition::Left   => ("\u{25c6}", 14u16, 18u16, 12u16), // ◆ diamond right
        TooltipPosition::Right  => ("\u{25c6}", 14u16, 18u16, 12u16), // ◆ diamond left
        TooltipPosition::FollowCursor => return Space::new(0, 0).into(),
    };

    container(
        widget::text(arrow_char)
            .size(font_sz)
            .class(cosmic::theme::Text::Color(style.border_color))
    )
    .width(Length::Fixed(w as f32))
    .height(Length::Fixed(h as f32))
    .center_x(Length::Fixed(w as f32))
    .center_y(Length::Fixed(h as f32))
    .into()
}

/// Tooltip bubble style configuration
#[derive(Debug, Clone, Copy)]
pub struct BubbleStyle {
    /// Background color (RGBA)
    pub bg_color: Color,
    /// Border width in pixels
    pub border_width: f32,
    /// Border color (RGBA)
    pub border_color: Color,
    /// Border radius in pixels
    pub border_radius: f32,
    /// Shadow offset (x, y)
    pub shadow_offset: (f32, f32),
    /// Shadow blur radius
    pub shadow_blur: f32,
    /// Shadow color (RGBA)
    pub shadow_color: Color,
    /// Text color (RGB)
    pub text_color: Color,
    /// Icon color (RGB)
    pub icon_color: Color,
    /// Padding (vertical, horizontal)
    pub padding: (f32, f32),
}

impl BubbleStyle {
    /// Default style - soft rounded bubble with gentle golden glow
    pub const DEFAULT: Self = Self {
        bg_color: Color::from_rgba(0.13, 0.13, 0.16, 0.96),
        border_width: 1.0,
        border_color: Color::from_rgba(0.90, 0.74, 0.48, 0.92),
        border_radius: 10.0,
        shadow_offset: (0.0, 3.0),
        shadow_blur: 12.0,
        shadow_color: Color::from_rgba(0.0, 0.0, 0.0, 0.45),
        text_color: Color::from_rgb(0.96, 0.96, 0.98),
        icon_color: Color::from_rgb(0.96, 0.82, 0.54),
        padding: (8.0, 13.0),
    };

    /// Accent style - 1.0px warm amber border
    pub const ACCENT: Self = Self {
        bg_color: Color::from_rgba(0.12, 0.15, 0.22, 0.98),
        border_width: 1.0,
        border_color: Color::from_rgba(0.95, 0.75, 0.35, 0.95),
        border_radius: 10.0,
        shadow_offset: (0.0, 4.0),
        shadow_blur: 12.0,
        shadow_color: Color::from_rgba(0.0, 0.0, 0.0, 0.4),
        text_color: Color::from_rgb(0.98, 0.98, 1.0),
        icon_color: Color::from_rgb(1.0, 0.85, 0.45),
        padding: (10.0, 14.0),
    };

    /// Success style - 1.0px green border
    pub const SUCCESS: Self = Self {
        bg_color: Color::from_rgba(0.10, 0.18, 0.14, 0.98),
        border_width: 1.0,
        border_color: Color::from_rgba(0.22, 0.82, 0.46, 0.90),
        border_radius: 8.0,
        shadow_offset: (0.0, 3.0),
        shadow_blur: 10.0,
        shadow_color: Color::from_rgba(0.0, 0.0, 0.0, 0.35),
        text_color: Color::from_rgb(0.96, 0.98, 0.96),
        icon_color: Color::from_rgb(0.32, 0.92, 0.56),
        padding: (8.0, 12.0),
    };

    /// Warning style - 1.0px orange border
    pub const WARNING: Self = Self {
        bg_color: Color::from_rgba(0.20, 0.16, 0.10, 0.98),
        border_width: 1.0,
        border_color: Color::from_rgba(0.98, 0.72, 0.28, 0.90),
        border_radius: 8.0,
        shadow_offset: (0.0, 3.0),
        shadow_blur: 10.0,
        shadow_color: Color::from_rgba(0.0, 0.0, 0.0, 0.35),
        text_color: Color::from_rgb(0.98, 0.96, 0.94),
        icon_color: Color::from_rgb(1.0, 0.82, 0.38),
        padding: (8.0, 12.0),
    };

    /// Danger style - 1.0px red border
    pub const DANGER: Self = Self {
        bg_color: Color::from_rgba(0.20, 0.10, 0.12, 0.98),
        border_width: 1.0,
        border_color: Color::from_rgba(0.96, 0.28, 0.32, 0.95),
        border_radius: 8.0,
        shadow_offset: (0.0, 3.0),
        shadow_blur: 10.0,
        shadow_color: Color::from_rgba(0.0, 0.0, 0.0, 0.4),
        text_color: Color::from_rgb(0.98, 0.96, 0.96),
        icon_color: Color::from_rgb(1.0, 0.38, 0.42),
        padding: (8.0, 12.0),
    };

    /// Subtle style - minimal warm border
    pub const SUBTLE: Self = Self {
        bg_color: Color::from_rgba(0.18, 0.18, 0.20, 0.95),
        border_width: 1.0,
        border_color: Color::from_rgba(0.70, 0.60, 0.45, 0.65),
        border_radius: 6.0,
        shadow_offset: (0.0, 2.0),
        shadow_blur: 6.0,
        shadow_color: Color::from_rgba(0.0, 0.0, 0.0, 0.2),
        text_color: Color::from_rgb(0.92, 0.92, 0.94),
        icon_color: Color::from_rgb(0.88, 0.78, 0.62),
        padding: (6.0, 10.0),
    };
}

/// Tooltip position
#[derive(Debug, Clone, Copy)]
pub enum TooltipPosition {
    Top,
    Bottom,
    Left,
    Right,
    FollowCursor,
}

impl TooltipPosition {
    fn to_iced_position(self) -> tooltip::Position {
        match self {
            TooltipPosition::Top => tooltip::Position::Top,
            TooltipPosition::Bottom => tooltip::Position::Bottom,
            TooltipPosition::Left => tooltip::Position::Left,
            TooltipPosition::Right => tooltip::Position::Right,
            TooltipPosition::FollowCursor => tooltip::Position::FollowCursor,
        }
    }
}

/// Add tooltip to any widget (basic style)
pub fn with_tooltip<'a>(
    content: impl Into<Element<'a, AppMessage>>,
    tooltip_text: &'a str,
    position: TooltipPosition,
) -> Element<'a, AppMessage> {
    tooltip(
        content,
        widget::text(tooltip_text).size(12),
        position.to_iced_position(),
    )
    .gap(4)
    .into()
}

/// Add tooltip with bubble style (enhanced visual)
pub fn with_tooltip_bubble<'a>(
    content: impl Into<Element<'a, AppMessage>>,
    tooltip_text: &'a str,
    position: TooltipPosition,
) -> Element<'a, AppMessage> {
    with_tooltip_bubble_styled(content, tooltip_text, position, BubbleStyle::DEFAULT)
}

/// Add tooltip with bubble style and custom style configuration
pub fn with_tooltip_bubble_styled<'a>(
    content: impl Into<Element<'a, AppMessage>>,
    tooltip_text: &'a str,
    position: TooltipPosition,
    style: BubbleStyle,
) -> Element<'a, AppMessage> {
    with_tooltip_bubble_styled_arrow(content, tooltip_text, position, style, false)
}

/// Add tooltip with bubble style, custom style, and optional arrow indicator
pub fn with_tooltip_bubble_styled_arrow<'a>(
    content: impl Into<Element<'a, AppMessage>>,
    tooltip_text: &'a str,
    position: TooltipPosition,
    style: BubbleStyle,
    show_arrow: bool,
) -> Element<'a, AppMessage> {
    // Add directional arrow based on position
    let arrow = if show_arrow {
        match position {
            TooltipPosition::Top => "▼ ",
            TooltipPosition::Bottom => "▲ ",
            TooltipPosition::Left => "▶ ",
            TooltipPosition::Right => "◀ ",
            TooltipPosition::FollowCursor => "● ",
        }
    } else {
        ""
    };

    let text_with_arrow = if show_arrow {
        format!("{}{}", arrow, tooltip_text)
    } else {
        tooltip_text.to_string()
    };

    let bubble = container(
        widget::text(text_with_arrow)
            .size(13)
            .class(cosmic::theme::Text::Default)
    )
    .padding([style.padding.0 as u16, style.padding.1 as u16])
    .style(move |_theme: &cosmic::Theme| container::Style {
        background: Some(Background::Color(style.bg_color)),
        border: Border {
            radius: style.border_radius.into(),
            width: style.border_width,
            color: style.border_color,
        },
        shadow: cosmic::iced::Shadow {
            color: style.shadow_color,
            offset: cosmic::iced::Vector::new(style.shadow_offset.0, style.shadow_offset.1),
            blur_radius: style.shadow_blur,
        },
        text_color: Some(style.text_color),
        icon_color: Some(style.icon_color),
    });

    tooltip(
        content,
        bubble,
        position.to_iced_position(),
    )
    .gap(6)
    .into()
}

/// Add tooltip with bubble style and icon
pub fn with_tooltip_bubble_icon<'a>(
    content: impl Into<Element<'a, AppMessage>>,
    tooltip_text: &'a str,
    icon: &'a str,
    position: TooltipPosition,
) -> Element<'a, AppMessage> {
    with_tooltip_bubble_icon_styled(content, tooltip_text, icon, position, BubbleStyle::DEFAULT)
}

/// Add tooltip with bubble style and arrow indicator
pub fn with_tooltip_bubble_arrow<'a>(
    content: impl Into<Element<'a, AppMessage>>,
    tooltip_text: &'a str,
    position: TooltipPosition,
) -> Element<'a, AppMessage> {
    with_tooltip_bubble_styled_arrow(content, tooltip_text, position, BubbleStyle::DEFAULT, true)
}

/// Add tooltip with bubble style, icon, and custom style configuration
pub fn with_tooltip_bubble_icon_styled<'a>(
    content: impl Into<Element<'a, AppMessage>>,
    tooltip_text: &'a str,
    icon: &'a str,
    position: TooltipPosition,
    style: BubbleStyle,
) -> Element<'a, AppMessage> {
    let bubble_content = widget::row::with_children(vec![
        widget::text(icon)
            .size(14)
            .class(cosmic::theme::Text::Color(style.icon_color))
            .into(),
        widget::Space::new(6, 0).into(),
        widget::text(tooltip_text)
            .size(13)
            .class(cosmic::theme::Text::Default)
            .into(),
    ])
    .align_y(Alignment::Center);

    let bubble = container(bubble_content)
        .padding([style.padding.0 as u16, style.padding.1 as u16])
        .style(move |_theme: &cosmic::Theme| container::Style {
            background: Some(Background::Color(style.bg_color)),
            border: Border {
                radius: style.border_radius.into(),
                width: style.border_width,
                color: style.border_color,
            },
            shadow: cosmic::iced::Shadow {
                color: style.shadow_color,
                offset: cosmic::iced::Vector::new(style.shadow_offset.0, style.shadow_offset.1),
                blur_radius: style.shadow_blur,
            },
            text_color: Some(style.text_color),
            icon_color: Some(style.icon_color),
        });

    tooltip(
        content,
        bubble,
        position.to_iced_position(),
    )
    .gap(6)
    .into()
}

/// Add tooltip with multiline text support
pub fn with_tooltip_multiline<'a>(
    content: impl Into<Element<'a, AppMessage>>,
    lines: &'a [&'a str],
    position: TooltipPosition,
) -> Element<'a, AppMessage> {
    with_tooltip_multiline_styled(content, lines, position, BubbleStyle::DEFAULT)
}

/// Add tooltip with multiline text support and custom style
pub fn with_tooltip_multiline_styled<'a>(
    content: impl Into<Element<'a, AppMessage>>,
    lines: &'a [&'a str],
    position: TooltipPosition,
    style: BubbleStyle,
) -> Element<'a, AppMessage> {
    let text_elements: Vec<Element<'a, AppMessage>> = lines
        .iter()
        .map(|line| {
            widget::text(*line)
                .size(12)
                .class(cosmic::theme::Text::Default)
                .into()
        })
        .collect();

    let bubble = container(
        widget::column::with_children(text_elements)
            .spacing(4)
    )
    .padding([style.padding.0 as u16, style.padding.1 as u16])
    .style(move |_theme: &cosmic::Theme| container::Style {
        background: Some(Background::Color(style.bg_color)),
        border: Border {
            radius: style.border_radius.into(),
            width: style.border_width,
            color: style.border_color,
        },
        shadow: cosmic::iced::Shadow {
            color: style.shadow_color,
            offset: cosmic::iced::Vector::new(style.shadow_offset.0, style.shadow_offset.1),
            blur_radius: style.shadow_blur,
        },
        text_color: Some(style.text_color),
        icon_color: Some(style.icon_color),
    });

    tooltip(
        content,
        bubble,
        position.to_iced_position(),
    )
    .gap(6)
    .into()
}

/// Add bilingual tooltip (English / Chinese) - basic style
pub fn with_tooltip_i18n<'a>(
    content: impl Into<Element<'a, AppMessage>>,
    lang: Language,
    en_text: &'a str,
    zh_text: &'a str,
    position: TooltipPosition,
) -> Element<'a, AppMessage> {
    let text = match lang {
        Language::ZhCn | Language::ZhTw => zh_text,
        _ => en_text,
    };
    with_tooltip(content, text, position)
}

/// Add bilingual tooltip with bubble style
pub fn with_tooltip_bubble_i18n<'a>(
    content: impl Into<Element<'a, AppMessage>>,
    lang: Language,
    en_text: &'a str,
    zh_text: &'a str,
    position: TooltipPosition,
) -> Element<'a, AppMessage> {
    let text = match lang {
        Language::ZhCn | Language::ZhTw => zh_text,
        _ => en_text,
    };
    with_tooltip_bubble(content, text, position)
}

/// Add bilingual tooltip with bubble style and icon
pub fn with_tooltip_bubble_icon_i18n<'a>(
    content: impl Into<Element<'a, AppMessage>>,
    lang: Language,
    en_text: &'a str,
    zh_text: &'a str,
    icon: &'a str,
    position: TooltipPosition,
) -> Element<'a, AppMessage> {
    let text = match lang {
        Language::ZhCn | Language::ZhTw => zh_text,
        _ => en_text,
    };
    with_tooltip_bubble_icon(content, text, icon, position)
}

/// Add bilingual tooltip with bubble style, icon, and custom style
pub fn with_tooltip_bubble_icon_i18n_styled<'a>(
    content: impl Into<Element<'a, AppMessage>>,
    lang: Language,
    en_text: &'a str,
    zh_text: &'a str,
    icon: &'a str,
    position: TooltipPosition,
    style: BubbleStyle,
) -> Element<'a, AppMessage> {
    let text = match lang {
        Language::ZhCn | Language::ZhTw => zh_text,
        _ => en_text,
    };
    with_tooltip_bubble_icon_styled(content, text, icon, position, style)
}

/// Add bilingual tooltip with bubble style and arrow indicator
pub fn with_tooltip_bubble_arrow_i18n<'a>(
    content: impl Into<Element<'a, AppMessage>>,
    lang: Language,
    en_text: &'a str,
    zh_text: &'a str,
    position: TooltipPosition,
) -> Element<'a, AppMessage> {
    let text = match lang {
        Language::ZhCn | Language::ZhTw => zh_text,
        _ => en_text,
    };
    with_tooltip_bubble_styled_arrow(content, text, position, BubbleStyle::DEFAULT, true)
}

/// Add bilingual tooltip with bubble style, icon, and arrow indicator
pub fn with_tooltip_bubble_icon_arrow_i18n<'a>(
    content: impl Into<Element<'a, AppMessage>>,
    lang: Language,
    en_text: &'a str,
    zh_text: &'a str,
    icon: &'a str,
    position: TooltipPosition,
) -> Element<'a, AppMessage> {
    with_tooltip_bubble_icon_arrow_i18n_styled(
        content, lang, en_text, zh_text, icon, position, BubbleStyle::DEFAULT
    )
}

/// Add bilingual tooltip with bubble style, icon, arrow indicator, and custom style
pub fn with_tooltip_bubble_icon_arrow_i18n_styled<'a>(
    content: impl Into<Element<'a, AppMessage>>,
    lang: Language,
    en_text: &'a str,
    zh_text: &'a str,
    icon: &'a str,
    position: TooltipPosition,
    style: BubbleStyle,
) -> Element<'a, AppMessage> {
    let text = match lang {
        Language::ZhCn | Language::ZhTw => zh_text,
        _ => en_text,
    };
    
    // Create the main bubble content
    let bubble_content = widget::row::with_children(vec![
        widget::text(icon)
            .size(14)
            .class(cosmic::theme::Text::Color(style.icon_color))
            .into(),
        widget::Space::new(6, 0).into(),
        widget::text(text)
            .size(13)
            .class(cosmic::theme::Text::Default)
            .into(),
    ])
    .align_y(Alignment::Center);

    // Create the main bubble container
    let bubble_main = container(bubble_content)
        .padding([style.padding.0 as u16, style.padding.1 as u16])
        .style(move |_theme: &cosmic::Theme| container::Style {
            background: Some(Background::Color(style.bg_color)),
            border: Border {
                radius: style.border_radius.into(),
                width: style.border_width,
                color: style.border_color,
            },
            shadow: cosmic::iced::Shadow {
                color: style.shadow_color,
                offset: cosmic::iced::Vector::new(style.shadow_offset.0, style.shadow_offset.1),
                blur_radius: style.shadow_blur,
            },
            text_color: Some(style.text_color),
            icon_color: Some(style.icon_color),
        });

    // Create the arrow pointer (triangle)
    let arrow = create_triangle_arrow(position, style);

    // spacing(4) gives a clean gap between arrow tip and bubble edge
    let bubble_with_arrow: Element<'_, AppMessage> = match position {
        TooltipPosition::Bottom => {
            widget::column::with_children(vec![
                container(arrow)
                    .center_x(Length::Shrink)
                    .into(),
                bubble_main.into(),
            ])
            .spacing(4)
            .align_x(Alignment::Center)
            .into()
        },
        TooltipPosition::Top => {
            widget::column::with_children(vec![
                bubble_main.into(),
                container(arrow)
                    .center_x(Length::Shrink)
                    .into(),
            ])
            .spacing(4)
            .align_x(Alignment::Center)
            .into()
        },
        TooltipPosition::Left => {
            widget::row::with_children(vec![
                bubble_main.into(),
                container(arrow)
                    .center_y(Length::Shrink)
                    .into(),
            ])
            .spacing(4)
            .align_y(Alignment::Center)
            .into()
        },
        TooltipPosition::Right => {
            widget::row::with_children(vec![
                container(arrow)
                    .center_y(Length::Shrink)
                    .into(),
                bubble_main.into(),
            ])
            .spacing(4)
            .align_y(Alignment::Center)
            .into()
        },
        TooltipPosition::FollowCursor => {
            bubble_main.into()
        },
    };

    tooltip(
        content,
        bubble_with_arrow,
        position.to_iced_position(),
    )
    .gap(4)
    .into()
}

/// Tooltip texts for common UI elements
pub struct TooltipTexts;

impl TooltipTexts {
    // Assistant page tooltips
    pub const ASSISTANT_START_SANDBOX: (&'static str, &'static str) = 
        ("Start the sandbox environment to run agents", "启动沙箱环境以运行 Agent");
    
    pub const ASSISTANT_STOP_SANDBOX: (&'static str, &'static str) = 
        ("Stop the sandbox environment", "停止沙箱环境");
    
    pub const ASSISTANT_EMERGENCY_STOP: (&'static str, &'static str) = 
        ("Emergency stop - immediately halt all running agents", "紧急停止 - 立即停止所有运行中的 Agent");
    
    pub const ASSISTANT_CLEAR_LOG: (&'static str, &'static str) = 
        ("Clear all event logs", "清空所有事件日志");
    
    pub const ASSISTANT_SEND_QUERY: (&'static str, &'static str) = 
        ("Send query to Assistant (Enter)", "发送查询给 Assistant（回车）");
    
    pub const ASSISTANT_INPUT: (&'static str, &'static str) = 
        ("Type your question or command here", "在此输入您的问题或命令");

    // AI Chat page tooltips
    pub const AI_SEND_MESSAGE: (&'static str, &'static str) = 
        ("Send message to AI (Enter)", "发送消息给 AI（回车）");
    
    pub const AI_INPUT: (&'static str, &'static str) = 
        ("Ask the AI assistant anything", "向 AI 助手提问");
    
    pub const AI_ENDPOINT: (&'static str, &'static str) = 
        ("Ollama API endpoint URL", "Ollama API 接口地址");
    
    pub const AI_MODEL: (&'static str, &'static str) = 
        ("AI model name (e.g., qwen2.5:0.5b)", "AI 模型名称（如 qwen2.5:0.5b）");

    // Claw Terminal tooltips
    pub const CLAW_NL_MODE: (&'static str, &'static str) = 
        ("Toggle Natural Language mode - AI will plan and execute commands", "切换自然语言模式 - AI 将规划并执行命令");
    
    pub const CLAW_CLEAR: (&'static str, &'static str) = 
        ("Clear terminal history", "清空终端历史");
    
    pub const CLAW_VOICE: (&'static str, &'static str) = 
        ("Start/Stop voice recording", "开始/停止语音录制");
    
    pub const CLAW_IMAGE: (&'static str, &'static str) = 
        ("Attach an image to your command", "为命令附加图片");
    
    pub const CLAW_SEND: (&'static str, &'static str) = 
        ("Execute command (Enter)", "执行命令（回车）");
    
    pub const CLAW_GATEWAY: (&'static str, &'static str) = 
        ("Check Gateway connection status", "检查 Gateway 连接状态");
    
    pub const CLAW_TELEGRAM: (&'static str, &'static str) = 
        ("Start/Stop Telegram bot polling", "启动/停止 Telegram 机器人轮询");

    // Settings page tooltips
    pub const SETTINGS_MEMORY_LIMIT: (&'static str, &'static str) = 
        ("Maximum memory (MB) allowed for sandbox", "沙箱允许的最大内存（MB）");
    
    pub const SETTINGS_ENTRY_PATH: (&'static str, &'static str) = 
        ("Path to the OpenClaw entry script", "OpenClaw 入口脚本路径");
    
    pub const SETTINGS_WORKSPACE: (&'static str, &'static str) = 
        ("Workspace directory for agent execution", "Agent 执行的工作区目录");
    
    pub const SETTINGS_INTERCEPT_SHELL: (&'static str, &'static str) = 
        ("Intercept and confirm shell commands before execution", "执行前拦截并确认 Shell 命令");
    
    pub const SETTINGS_FILE_DELETE: (&'static str, &'static str) = 
        ("Confirm before deleting files", "删除文件前确认");
    
    pub const SETTINGS_NETWORK: (&'static str, &'static str) = 
        ("Confirm before network requests", "网络请求前确认");
    
    pub const SETTINGS_AI_ENDPOINT: (&'static str, &'static str) = 
        ("Ollama server endpoint for AI inference", "AI 推理的 Ollama 服务器接口");
    
    pub const SETTINGS_AI_MODEL: (&'static str, &'static str) = 
        ("Default AI model for inference", "默认 AI 推理模型");
    
    pub const SETTINGS_REFRESH_MODELS: (&'static str, &'static str) = 
        ("Refresh available models from Ollama", "从 Ollama 刷新可用模型");
    
    pub const SETTINGS_FILTER_MODELS: (&'static str, &'static str) = 
        ("Filter models by name or family", "按名称或系列筛选模型");

    // Dashboard tooltips
    pub const DASHBOARD_START: (&'static str, &'static str) = 
        ("Start sandbox environment", "启动沙箱环境");
    
    pub const DASHBOARD_STOP: (&'static str, &'static str) = 
        ("Stop sandbox environment", "停止沙箱环境");
    
    pub const DASHBOARD_EMERGENCY: (&'static str, &'static str) = 
        ("Emergency stop all operations", "紧急停止所有操作");
    
    pub const DASHBOARD_EMERGENCY_STOP: (&'static str, &'static str) = 
        ("Emergency stop - immediately halt all operations", "紧急停止 - 立即停止所有操作");
    
    pub const DASHBOARD_CLEAR: (&'static str, &'static str) = 
        ("Clear event log", "清空事件日志");
    
    pub const DASHBOARD_ALLOW: (&'static str, &'static str) = 
        ("Allow this operation", "允许此操作");
    
    pub const DASHBOARD_DENY: (&'static str, &'static str) = 
        ("Deny this operation", "拒绝此操作");
}

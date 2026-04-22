use std::io::{stdout, Write};
use crossterm::{
    style::{Color, Print, ResetColor, SetForegroundColor, Attribute, SetAttribute},
    QueueableCommand,
};

pub fn render_markdown(text: &str) {
    let mut out = stdout();
    let lines: Vec<&str> = text.lines().collect();
    let mut in_code_block = false;
    let mut code_content = String::new();
    let mut code_lang = String::new();

    for line in lines {
        let trimmed = line.trim();

        if trimmed.starts_with("```") {
            if in_code_block {
                let _ = out.queue(Print("┌─"));
                let _ = out.queue(SetForegroundColor(Color::DarkGrey));
                if !code_lang.is_empty() {
                    let _ = out.queue(Print(&format!(" {} ", code_lang)));
                } else {
                    let _ = out.queue(Print(" code "));
                }
                let _ = out.queue(ResetColor);
                let _ = out.queue(Print(&"─".repeat(60)));
                let _ = out.queue(Print("┐\n"));
                for code_line in code_content.lines() {
                    let _ = out.queue(SetForegroundColor(Color::Green));
                    let _ = out.queue(Print("│ "));
                    let _ = out.queue(Print(code_line));
                    let _ = out.queue(Print("\n"));
                }
                let _ = out.queue(ResetColor);
                let _ = out.queue(Print(&"─".repeat(70)));
                let _ = out.queue(Print("┘\n\n"));
                code_content.clear();
                code_lang.clear();
                in_code_block = false;
            } else {
                in_code_block = true;
                let lang = trimmed.trim_start_matches("```").trim();
                if !lang.is_empty() {
                    code_lang = lang.to_string();
                }
            }
            continue;
        }

        if in_code_block {
            code_content.push_str(line);
            code_content.push('\n');
            continue;
        }

        if trimmed.starts_with("### ") {
            let _ = out.queue(SetForegroundColor(Color::Cyan));
            let _ = out.queue(SetAttribute(Attribute::Bold));
            let _ = out.queue(Print("  "));
            let _ = out.queue(Print(&trimmed[4..]));
            let _ = out.queue(ResetColor);
            let _ = out.queue(Print("\n"));
        } else if trimmed.starts_with("## ") {
            let _ = out.queue(SetForegroundColor(Color::Cyan));
            let _ = out.queue(SetAttribute(Attribute::Bold));
            let _ = out.queue(Print(" "));
            let _ = out.queue(Print(&trimmed[3..]));
            let _ = out.queue(ResetColor);
            let _ = out.queue(Print("\n"));
        } else if trimmed.starts_with("# ") {
            let _ = out.queue(SetForegroundColor(Color::Cyan));
            let _ = out.queue(SetAttribute(Attribute::Bold));
            let _ = out.queue(Print(&trimmed[2..]));
            let _ = out.queue(ResetColor);
            let _ = out.queue(Print("\n"));
        } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            let content = &trimmed[2..];
            let _ = out.queue(Print("  • "));
            render_inline(&mut out, content);
            let _ = out.queue(Print("\n"));
        } else if trimmed.starts_with(|c: char| c.is_ascii_digit()) && trimmed.contains(". ") {
            if let Some(pos) = trimmed.find(". ") {
                let num = &trimmed[..=pos];
                let content = &trimmed[pos + 2..];
                let _ = out.queue(SetForegroundColor(Color::Yellow));
                let _ = out.queue(Print(num));
                let _ = out.queue(ResetColor);
                render_inline(&mut out, content);
                let _ = out.queue(Print("\n"));
            }
        } else if trimmed.starts_with("---") || trimmed.starts_with("***") {
            let _ = out.queue(Print("\n"));
            let _ = out.queue(SetForegroundColor(Color::DarkGrey));
            let _ = out.queue(Print(&"─".repeat(60)));
            let _ = out.queue(ResetColor);
            let _ = out.queue(Print("\n"));
        } else if trimmed.is_empty() {
            let _ = out.queue(Print("\n"));
        } else {
            render_inline(&mut out, trimmed);
            let _ = out.queue(Print("\n"));
        }
    }

    let _ = out.queue(ResetColor);
    let _ = out.flush();
}

fn render_inline(out: &mut std::io::Stdout, text: &str) {
    let mut current = String::new();
    let mut in_bold = false;
    let mut in_code = false;
    let mut in_italic = false;
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '*' if !in_code => {
                if chars.peek() == Some(&'*') {
                    chars.next();
                    if !current.is_empty() {
                        flush_inline(out, &current, in_bold, in_code, in_italic);
                        current.clear();
                    }
                    in_bold = !in_bold;
                } else {
                    if !current.is_empty() {
                        flush_inline(out, &current, in_bold, in_code, in_italic);
                        current.clear();
                    }
                    in_italic = !in_italic;
                }
            }
            '`' => {
                if !current.is_empty() {
                    flush_inline(out, &current, in_bold, in_code, in_italic);
                    current.clear();
                }
                in_code = !in_code;
            }
            '[' => {
                let mut link_text = String::new();
                let mut link_url = String::new();
                while let Some(nc) = chars.next() {
                    if nc == ']' {
                        break;
                    }
                    link_text.push(nc);
                }
                if chars.peek() == Some(&'(') {
                    chars.next();
                    while let Some(nc) = chars.next() {
                        if nc == ')' {
                            break;
                        }
                        link_url.push(nc);
                    }
                }
                let _ = out.queue(SetForegroundColor(Color::Blue));
                let _ = out.queue(SetAttribute(Attribute::Underlined));
                let _ = out.queue(Print(&link_text));
                let _ = out.queue(ResetColor);
                if in_bold {
                    let _ = out.queue(SetAttribute(Attribute::Bold));
                }
            }
            '\\' => {
                if let Some(nc) = chars.next() {
                    current.push(nc);
                }
            }
            _ => {
                current.push(c);
            }
        }
    }

    if !current.is_empty() {
        flush_inline(out, &current, in_bold, in_code, in_italic);
    }
}

fn flush_inline(out: &mut std::io::Stdout, text: &str, bold: bool, code: bool, italic: bool) {
    if code {
        let _ = out.queue(SetForegroundColor(Color::Green));
        let _ = out.queue(SetAttribute(Attribute::Bold));
        let _ = out.queue(Print(text));
        let _ = out.queue(ResetColor);
    } else if bold {
        let _ = out.queue(SetAttribute(Attribute::Bold));
        let _ = out.queue(Print(text));
        let _ = out.queue(ResetColor);
    } else if italic {
        let _ = out.queue(SetAttribute(Attribute::Italic));
        let _ = out.queue(Print(text));
        let _ = out.queue(ResetColor);
    } else {
        let _ = out.queue(Print(text));
    }
}

use pulldown_cmark::{Event, Parser, Tag};

#[derive(Debug)]
pub struct ParentBlock {
    pub header: String,
    pub payload: String,
    pub children: Vec<String>,
}

fn generate_sliding_windows(text: &str, window_size: usize, overlap: usize) -> Vec<String> {
    if text.trim().is_empty() {
        return vec![];
    }

    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= window_size {
        return vec![text.to_string()];
    }

    let mut windows = Vec::new();
    let mut start = 0;

    while start < chars.len() {
        let end = std::cmp::min(start + window_size, chars.len());
        let window_text: String = chars[start..end].iter().collect();
        windows.push(window_text);

        if end == chars.len() {
            break;
        }

        let step = if window_size > overlap {
            window_size - overlap
        } else {
            1
        };
        start += step;
    }

    windows
}

fn flush_block(header: &str, content: &mut String, blocks: &mut Vec<ParentBlock>) {
    let trimmed = content.trim();
    if !trimmed.is_empty() {
        let payload = trimmed.to_string();
        let children = generate_sliding_windows(&payload, 300, 50);
        blocks.push(ParentBlock {
            header: header.to_string(),
            payload,
            children,
        });
        content.clear();
    }
}

/// Parses Markdown into structural Parent Blocks with dense overlapping child vectors
pub fn chunk_markdown(markdown: &str) -> Vec<ParentBlock> {
    let parser = Parser::new(markdown);
    let mut blocks = Vec::new();

    let mut current_header = String::from("Document Root");
    let mut current_content = String::new();
    let mut in_heading = false;
    let mut heading_text = String::new();

    for event in parser {
        match event {
            Event::Start(Tag::Heading { .. }) => {
                in_heading = true;
                heading_text.clear();
                flush_block(&current_header, &mut current_content, &mut blocks);
            }
            Event::End(pulldown_cmark::TagEnd::Heading(..)) => {
                in_heading = false;
                current_header = heading_text.trim().to_string();
                current_content.push_str(&format!("# {}\n\n", current_header));
            }
            Event::End(pulldown_cmark::TagEnd::Paragraph) => {
                if !in_heading && !current_content.ends_with("\n\n") {
                    current_content.push_str("\n\n");
                }
            }
            Event::End(pulldown_cmark::TagEnd::Item) => {
                if !in_heading && !current_content.ends_with('\n') {
                    current_content.push('\n');
                }
            }
            Event::End(pulldown_cmark::TagEnd::CodeBlock) => {
                if !in_heading && !current_content.ends_with("\n\n") {
                    current_content.push_str("\n\n");
                }
            }
            Event::Text(text) | Event::Code(text) => {
                if in_heading {
                    heading_text.push_str(&text);
                } else {
                    current_content.push_str(&text);
                }
            }
            Event::SoftBreak | Event::HardBreak if !in_heading => {
                current_content.push('\n');
            }
            _ => {}
        }
    }

    flush_block(&current_header, &mut current_content, &mut blocks);

    blocks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_heading() {
        let md = "# Introduction\n\nThis is a test document.";
        let blocks = chunk_markdown(md);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].header, "Introduction");
        assert!(blocks[0].payload.contains("This is a test document."));
        assert_eq!(blocks[0].children.len(), 1);
    }

    #[test]
    fn test_multiple_hierarchical_headings() {
        let md = r#"
# Architecture

Core system design.

## Database

LanceDB vector storage.

### Indexing

Full-text search with Tantivy.
"#;
        let blocks = chunk_markdown(md);
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].header, "Architecture");
        assert_eq!(blocks[1].header, "Database");
        assert_eq!(blocks[2].header, "Indexing");
    }

    #[test]
    fn test_root_fallback_when_no_headers() {
        let md = "Just a raw paragraph without any markdown headings.";
        let blocks = chunk_markdown(md);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].header, "Document Root");
        assert!(blocks[0].payload.contains("Just a raw paragraph"));
    }

    #[test]
    fn test_code_block_with_hash_is_not_heading() {
        let md = r#"
# Code Sample

```python
# This is a Python comment, not a markdown heading
def hello():
    return "world"
```
"#;
        let blocks = chunk_markdown(md);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].header, "Code Sample");
        assert!(blocks[0].payload.contains("# This is a Python comment"));
    }

    #[test]
    fn test_empty_and_whitespace_documents() {
        assert!(chunk_markdown("").is_empty());
        assert!(chunk_markdown("   \n\n  \t ").is_empty());
    }

    #[test]
    fn test_paragraph_separation() {
        let md = "# Title\n\nFirst paragraph.\n\nSecond paragraph.";
        let blocks = chunk_markdown(md);
        assert_eq!(blocks.len(), 1);
        assert!(blocks[0]
            .payload
            .contains("First paragraph.\n\nSecond paragraph."));
    }

    #[test]
    fn test_sliding_window_multibyte_unicode() {
        let text = "こんにちは世界！".repeat(30); // multibyte characters
        let windows = generate_sliding_windows(&text, 100, 20);
        assert!(windows.len() > 1);
        for w in &windows {
            assert!(!w.is_empty());
            assert!(w.chars().count() <= 100);
        }
    }

    #[test]
    fn test_sliding_window_shorter_than_window_size() {
        let text = "Short text";
        let windows = generate_sliding_windows(text, 300, 50);
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0], "Short text");
    }

    #[test]
    fn test_sliding_window_overlap_step() {
        let text = "abcdefghij".repeat(50); // 500 chars
        let windows = generate_sliding_windows(&text, 100, 20);
        assert_eq!(windows.len(), 6);
        assert_eq!(windows[0].chars().count(), 100);
    }

    #[test]
    fn test_sliding_window_exact_boundaries_and_zero_step_protection() {
        // Text of exact window length (300 chars) must produce exactly 1 chunk, no empty tail
        let exact_300 = "a".repeat(300);
        let win_300 = generate_sliding_windows(&exact_300, 300, 50);
        assert_eq!(win_300.len(), 1);
        assert_eq!(win_300[0].len(), 300);

        // Text of 301 chars must produce 2 chunks
        let win_301 = generate_sliding_windows(&"b".repeat(301), 300, 50);
        assert_eq!(win_301.len(), 2);

        // Pathological case: overlap >= window_size must not loop infinitely
        let safe_win = generate_sliding_windows("test text", 10, 10);
        assert_eq!(safe_win.len(), 1);
    }

    #[test]
    fn test_consecutive_headings_without_body() {
        let md = r#"
# System Design
## Database Layer
LanceDB storage engine.
"#;
        let blocks = chunk_markdown(md);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].header, "System Design");
        assert_eq!(blocks[1].header, "Database Layer");
        assert!(blocks[1].payload.contains("LanceDB storage engine."));
    }
}

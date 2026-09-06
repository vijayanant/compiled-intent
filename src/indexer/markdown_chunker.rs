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
    
    if text.len() <= window_size {
        return vec![text.to_string()];
    }

    let chars: Vec<char> = text.chars().collect();
    let mut windows = Vec::new();
    let mut start = 0;

    while start < chars.len() {
        let end = std::cmp::min(start + window_size, chars.len());
        let window_text: String = chars[start..end].iter().collect();
        windows.push(window_text);

        if end == chars.len() {
            break;
        }
        
        let step = if window_size > overlap { window_size - overlap } else { 1 };
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

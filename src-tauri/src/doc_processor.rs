use std::fs;
use std::path::Path;

/// Parsed document content
pub struct ParsedDocument {
    pub content: String,
    pub file_type: String,
}

/// Parse a document file into plain text
pub fn parse_file(path: &Path) -> Result<ParsedDocument, String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "txt" => {
            let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
            Ok(ParsedDocument {
                content,
                file_type: "txt".into(),
            })
        }
        "md" | "markdown" => {
            let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
            Ok(ParsedDocument {
                content,
                file_type: "md".into(),
            })
        }
        "pdf" => {
            let bytes = fs::read(path).map_err(|e| e.to_string())?;
            let content = pdf_extract::extract_text_from_mem(&bytes)
                .map_err(|e| format!("PDF parse error: {}", e))?;
            Ok(ParsedDocument {
                content,
                file_type: "pdf".into(),
            })
        }
        _ => Err(format!("Unsupported file type: .{}", ext)),
    }
}

/// Split text into overlapping chunks for embedding
pub fn chunk_text(text: &str, chunk_size: usize, overlap: usize) -> Vec<String> {
    let text = text.trim();
    if text.is_empty() {
        return vec![];
    }
    if text.len() <= chunk_size {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut start = 0;

    while start < chars.len() {
        let end = (start + chunk_size).min(chars.len());
        let chunk: String = chars[start..end].iter().collect();
        let chunk = chunk.trim().to_string();
        if !chunk.is_empty() {
            chunks.push(chunk);
        }
        if end >= chars.len() {
            break;
        }
        start += chunk_size - overlap;
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_text_short() {
        let chunks = chunk_text("Hello world", 100, 20);
        assert_eq!(chunks, vec!["Hello world"]);
    }

    #[test]
    fn test_chunk_text_overlap() {
        let text = "a".repeat(100);
        let chunks = chunk_text(&text, 40, 10);
        assert!(chunks.len() >= 3);
        // Check overlap exists
        assert_eq!(chunks[0].len(), 40);
    }
}

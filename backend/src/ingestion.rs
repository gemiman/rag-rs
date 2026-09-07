use anyhow::{Context, Result};

/// 根据文件名扩展名，把原始字节解析成纯文本
pub fn parse_document(filename: &str, bytes: &[u8]) -> Result<String> {
    let ext = std::path::Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "txt" | "md" | "markdown" => Ok(String::from_utf8_lossy(bytes).to_string()),
        "csv" => parse_csv(bytes),
        "xlsx" | "xls" => parse_xlsx(bytes),
        "pdf" => parse_pdf(bytes),
        "docx" => parse_docx(bytes),
        other => Err(anyhow::anyhow!("暂不支持的文件格式: {other}")),
    }
}

/// 解析 CSV
fn parse_csv(bytes: &[u8]) -> Result<String> {
    let mut reader = csv::Reader::from_reader(bytes);
    let headers = reader.headers()?.clone();
    let mut out = String::new();
    for record in reader.records() {
        let record = record?;
        let mut parts = Vec::new();
        for (i, field) in record.iter().enumerate() {
            let h = headers.get(i).unwrap_or("");
            parts.push(format!("{h}: {field}"));
        }
        out.push_str(&parts.join(" | "));
        out.push('\n');
    }
    Ok(out)
}

/// 解析 Excel（xlsx/xls）
fn parse_xlsx(bytes: &[u8]) -> Result<String> {
    use calamine::{Data, Reader};

    let cursor = std::io::Cursor::new(bytes);
    let mut workbook =
        calamine::open_workbook_auto_from_rs(cursor).context("打开 Excel 失败")?;
    let mut out = String::new();

    for sheet_name in workbook.sheet_names().to_vec() {
        if let Ok(range) = workbook.worksheet_range(&sheet_name) {
            out.push_str(&format!("【工作表 {sheet_name}】\n"));
            for row in range.rows() {
                let cells: Vec<String> = row
                    .iter()
                    .map(|cell| match cell {
                        Data::String(s) => s.clone(),
                        Data::Float(f) => f.to_string(),
                        Data::Int(i) => i.to_string(),
                        Data::Bool(b) => b.to_string(),
                        _ => String::new(),
                    })
                    .collect();
                if cells.iter().any(|c| !c.is_empty()) {
                    out.push_str(&cells.join(" | "));
                    out.push('\n');
                }
            }
        }
    }
    Ok(out)
}

/// 解析 PDF（仅支持文本型 PDF，扫描件无法提取）
fn parse_pdf(bytes: &[u8]) -> Result<String> {
    let doc = lopdf::Document::load_mem(bytes).context("解析 PDF 失败")?;
    let mut out = String::new();
    let pages = doc.get_pages();
    for (page_num, _) in pages {
        if let Ok(content) = doc.extract_text(&[page_num]) {
            out.push_str(&content);
            out.push('\n');
        }
    }
    Ok(out)
}

/// 解析 Word（docx）：解压 ZIP，提取 document.xml 里的文本
fn parse_docx(bytes: &[u8]) -> Result<String> {
    let cursor = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor).context("打开 docx（ZIP）失败")?;

    let mut xml = String::new();
    {
        let mut f = archive
            .by_name("word/document.xml")
            .context("docx 中找不到 document.xml")?;
        std::io::Read::read_to_string(&mut f, &mut xml)?;
    }

    let mut reader = quick_xml::Reader::from_str(&xml);
    reader.config_mut().trim_text(false);

    let mut out = String::new();
    let mut in_text = false;
    loop {
        match reader.read_event() {
            Ok(quick_xml::events::Event::Start(e)) => {
                if e.name().as_ref() == b"w:t" {
                    in_text = true;
                }
            }
            Ok(quick_xml::events::Event::End(e)) => {
                if e.name().as_ref() == b"w:t" {
                    in_text = false;
                } else if e.name().as_ref() == b"w:p" {
                    out.push('\n'); // 段落结束换行
                }
            }
            Ok(quick_xml::events::Event::Text(t)) => {
                if in_text {
                    out.push_str(&t.decode().unwrap_or_default());
                }
            }
            Ok(quick_xml::events::Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }
    Ok(out)
}

/// 把长文本切成若干片段（chunk），保留少量重叠，便于检索
pub fn split_text(text: &str, chunk_size: usize, overlap: usize) -> Vec<String> {
    // 先按段落（换行）切分
    let paragraphs: Vec<&str> = text
        .split('\n')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    let mut chunks: Vec<String> = Vec::new();
    let mut current = String::new();

    for p in paragraphs {
        // 段落太长，先按句子边界拆
        for piece in split_long(p, chunk_size, overlap) {
            if current.len() + piece.len() + 1 > chunk_size && !current.is_empty() {
                chunks.push(std::mem::take(&mut current));
            }
            if !current.is_empty() {
                current.push('\n');
            }
            current.push_str(&piece);
        }
    }

    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

/// 把超长单段文本按句子边界拆成多个不超过 chunk_size 的小段
fn split_long(text: &str, chunk_size: usize, overlap: usize) -> Vec<String> {
    if text.chars().count() <= chunk_size {
        return vec![text.to_string()];
    }

    let mut out = Vec::new();
    let mut rest = text.to_string();
    while rest.chars().count() > chunk_size {
        // 取前 chunk_size 个字符，找最后一个句子边界
        let head: String = rest.chars().take(chunk_size).collect();
        let boundary = head
            .char_indices()
            .rev()
            .find(|(_, c)| matches!(c, '。' | '！' | '？' | '；' | '，' | ' '))
            .map(|(i, _)| i + 1)
            .unwrap_or(chunk_size);
        let piece: String = head.chars().take(boundary).collect();
        out.push(piece);

        // 前进 chunk_size - overlap 个字符，保留重叠
        let skip = chunk_size.saturating_sub(overlap).max(1);
        rest = rest.chars().skip(skip).collect::<String>();
    }
    if !rest.trim().is_empty() {
        out.push(rest);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_text_short_is_single_chunk() {
        let chunks = split_text("短短的一句话", 100, 20);
        assert_eq!(chunks, vec!["短短的一句话".to_string()]);
    }

    #[test]
    fn test_split_text_long_is_multiple_chunks_within_size() {
        let text = "这是一个比较长的文本，用来测试切块逻辑是否能够正确地把内容切成多个片段。";
        let chunks = split_text(text, 12, 2);
        assert!(chunks.len() > 1, "应切成多块，实际: {:?}", chunks);
        for c in &chunks {
            assert!(c.chars().count() <= 12, "块超长: {:?}", c);
        }
    }

    #[test]
    fn test_split_text_empty_is_empty() {
        assert!(split_text("", 100, 20).is_empty());
    }

    #[test]
    fn test_parse_document_txt() {
        let out = parse_document("a.txt", "hello".as_bytes()).unwrap();
        assert_eq!(out, "hello");
    }

    #[test]
    fn test_parse_document_md() {
        let out = parse_document("a.md", "# 标题".as_bytes()).unwrap();
        assert_eq!(out, "# 标题");
    }

    #[test]
    fn test_parse_document_csv() {
        let csv = "名称,价格\n手机,4999\n电脑,8999\n";
        let out = parse_document("a.csv", csv.as_bytes()).unwrap();
        assert!(out.contains("名称: 手机"));
        assert!(out.contains("价格: 4999"));
        assert!(out.contains("名称: 电脑"));
    }

    #[test]
    fn test_parse_document_unsupported_extension() {
        assert!(parse_document("a.xyz", b"data").is_err());
    }
}

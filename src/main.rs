use std::{
    env,
    error::Error,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use html2md::parse_html;

#[derive(Debug)]
enum EpubToMdError {
    InputError(String),
    PandocError(String),
    FileIOError(String),
    UsageError,
    PandocCheckError(String),
}

impl std::fmt::Display for EpubToMdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EpubToMdError::InputError(msg) => write!(f, "Input Error: {}", msg),
            EpubToMdError::PandocError(msg) => write!(f, "Pandoc Error: {}", msg),
            EpubToMdError::FileIOError(msg) => write!(f, "File IO Error: {}", msg),
            EpubToMdError::UsageError => write!(f, "Usage: epub2md <input_epub> [output_md]"),
            EpubToMdError::PandocCheckError(msg) => write!(f, "Pandoc Check Error: {}", msg),
        }
    }
}

impl Error for EpubToMdError {}

// 检查 pandoc 是否安装
fn check_pandoc() -> Result<(), EpubToMdError> {
    match Command::new("pandoc")
        .arg("--version")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(mut child) => {
            let status = child.wait().map_err(|e| EpubToMdError::PandocCheckError(format!("Failed to wait for pandoc process: {}", e)))?;
            if status.success() {
                Ok(())
            } else {
                Err(EpubToMdError::PandocCheckError("Pandoc command failed to execute.".to_string()))
            }
        }
        Err(_) => Err(EpubToMdError::PandocCheckError(
            "Pandoc is not installed or not in PATH. Please ensure pandoc is installed and accessible.".to_string(),
        )),
    }
}


fn convert_epub_to_md(epub_path_str: &str, md_path_str: Option<&str>) -> Result<(), EpubToMdError> {
    if let Err(e) = check_pandoc() {
        eprintln!("{}", e);
        return Err(e); // 如果 pandoc 未安装，直接返回错误
    }

    let epub_path = Path::new(epub_path_str);

    // 检查输入文件是否为 EPUB 格式
    if epub_path.extension().and_then(|s| s.to_str()) != Some("epub") {
        return Err(EpubToMdError::InputError("Input file must be an EPUB file.".to_string()));
    }

    let current_dir = env::current_dir().map_err(|e| EpubToMdError::FileIOError(format!("Failed to get current directory: {}", e)))?;
    let html_path = current_dir.join("temp_epub.html");
    let md_path = match md_path_str {
        Some(p) => PathBuf::from(p),
        None => {
            let file_name = epub_path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| EpubToMdError::InputError("Invalid input file name.".to_string()))?;
            let md_file_name = file_name.trim_end_matches(".epub").to_string() + ".md";
            current_dir.join(md_file_name)
        }
    };

    // 执行 pandoc 命令将 EPUB 转换为 HTML
    let pandoc_output = Command::new("pandoc")
        .arg(epub_path)
        .arg("-o")
        .arg(&html_path)
        .output()
        .map_err(|e| EpubToMdError::PandocError(format!("Failed to execute pandoc: {}", e)))?;

    if !pandoc_output.status.success() {
        let error_message = String::from_utf8_lossy(&pandoc_output.stderr);
        return Err(EpubToMdError::PandocError(format!("pandoc command failed: {}", error_message)));
    }

    // 读取 HTML 文件内容
    let html_content = fs::read_to_string(&html_path)
        .map_err(|e| EpubToMdError::FileIOError(format!("Failed to read HTML file: {}", e)))?;

    // 使用 html2md 转换为 Markdown
    let markdown_content = parse_html(&html_content);

    // 写入 Markdown 文件
    fs::write(&md_path, markdown_content.as_bytes())
        .map_err(|e| EpubToMdError::FileIOError(format!("Failed to write Markdown file: {}", e)))?;

    // 删除临时 HTML 文件
    fs::remove_file(&html_path).map_err(|e| EpubToMdError::FileIOError(format!("Failed to remove temporary HTML file: {}", e)))?;

    Ok(())
}

fn main() -> Result<(), EpubToMdError> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("{}", EpubToMdError::UsageError);
        return Err(EpubToMdError::UsageError);
    }

    let epub_path = &args[1];
    let md_path = args.get(2).map(|s| s.as_str());

    if let Err(e) = convert_epub_to_md(epub_path, md_path) {
        eprintln!("Error: {}", e); // 打印详细错误信息
        return Err(e);
    }

    println!("EPUB to Markdown conversion successful!");
    Ok(())
}
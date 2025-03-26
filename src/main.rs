/// 导入所需的标准库和第三方库
use std::{
    env,                            // 环境变量操作
    error::Error,                   // 错误处理trait
    fs,                            // 文件系统操作
    path::{Path, PathBuf},         // 路径处理
    process::{Command, Stdio},     // 进程控制
};
use html2md::parse_html;           // HTML转Markdown库

/// 自定义错误类型，用于处理转换过程中可能出现的各种错误
#[derive(Debug)]
enum EpubToMdError {
    InputError(String),            // 输入错误
    PandocError(String),          // Pandoc相关错误
    FileIOError(String),          // 文件IO错误
    UsageError,                   // 使用方法错误
    PandocCheckError(String),     // Pandoc检查错误
}

/// 为自定义错误类型实现Display trait，用于格式化错误信息
impl std::fmt::Display for EpubToMdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EpubToMdError::InputError(msg) => write!(f, "输入错误: {}", msg),
            EpubToMdError::PandocError(msg) => write!(f, "Pandoc错误: {}", msg),
            EpubToMdError::FileIOError(msg) => write!(f, "文件IO错误: {}", msg),
            EpubToMdError::UsageError => write!(f, "使用方法: epub2md <输入epub文件> [输出md文件]"),
            EpubToMdError::PandocCheckError(msg) => write!(f, "Pandoc检查错误: {}", msg),
        }
    }
}

/// 实现Error trait，使其成为标准错误类型
impl Error for EpubToMdError {}

/// 检查系统中是否安装了Pandoc
/// 
/// 返回值:
/// - Ok(()): Pandoc已正确安装
/// - Err(EpubToMdError): Pandoc未安装或出现错误
fn check_pandoc() -> Result<(), EpubToMdError> {
    match Command::new("pandoc")
        .arg("--version")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(mut child) => {
            let status = child.wait().map_err(|e| 
                EpubToMdError::PandocCheckError(format!("等待Pandoc进程失败: {}", e)))?;
            if status.success() {
                Ok(())
            } else {
                Err(EpubToMdError::PandocCheckError("Pandoc命令执行失败".to_string()))
            }
        }
        Err(_) => Err(EpubToMdError::PandocCheckError(
            "Pandoc未安装或不在PATH中。请确保Pandoc已正确安装".to_string(),
        )),
    }
}

/// EPUB文件转换为Markdown的核心函数
/// 
/// 参数:
/// - epub_path_str: EPUB文件路径
/// - md_path_str: 可选的输出Markdown文件路径
/// 
/// 返回值:
/// - Ok(()): 转换成功
/// - Err(EpubToMdError): 转换过程中出现错误
fn convert_epub_to_md(epub_path_str: &str, md_path_str: Option<&str>) -> Result<(), EpubToMdError> {
    // 检查Pandoc是否安装
    if let Err(e) = check_pandoc() {
        eprintln!("{}", e);
        return Err(e);
    }

    let epub_path = Path::new(epub_path_str);

    // 验证输入文件扩展名
    if epub_path.extension().and_then(|s| s.to_str()) != Some("epub") {
        return Err(EpubToMdError::InputError("输入文件必须是EPUB格式".to_string()));
    }

    // 获取当前工作目录
    let current_dir = env::current_dir()
        .map_err(|e| EpubToMdError::FileIOError(format!("获取当前目录失败: {}", e)))?;
    
    // 创建临时HTML文件路径
    let html_path = current_dir.join("temp_epub.html");
    
    // 确定输出Markdown文件路径
    let md_path = match md_path_str {
        Some(p) => PathBuf::from(p),
        None => {
            let file_name = epub_path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| EpubToMdError::InputError("无效的输入文件名".to_string()))?;
            let md_file_name = file_name.trim_end_matches(".epub").to_string() + ".md";
            current_dir.join(md_file_name)
        }
    };

    // 使用Pandoc将EPUB转换为HTML
    let pandoc_output = Command::new("pandoc")
        .arg(epub_path)
        .arg("-o")
        .arg(&html_path)
        .output()
        .map_err(|e| EpubToMdError::PandocError(format!("执行Pandoc失败: {}", e)))?;

    // 检查Pandoc转换是否成功
    if !pandoc_output.status.success() {
        let error_message = String::from_utf8_lossy(&pandoc_output.stderr);
        return Err(EpubToMdError::PandocError(format!("Pandoc命令失败: {}", error_message)));
    }

    // 读取生成的HTML文件
    let html_content = fs::read_to_string(&html_path)
        .map_err(|e| EpubToMdError::FileIOError(format!("读取HTML文件失败: {}", e)))?;

    // 将HTML转换为Markdown
    let markdown_content = parse_html(&html_content);

    // 将Markdown内容写入文件
    fs::write(&md_path, markdown_content.as_bytes())
        .map_err(|e| EpubToMdError::FileIOError(format!("写入Markdown文件失败: {}", e)))?;

    // 清理临时文件
    fs::remove_file(&html_path)
        .map_err(|e| EpubToMdError::FileIOError(format!("删除临时HTML文件失败: {}", e)))?;

    Ok(())
}

/// 主函数：处理命令行参数并执行转换
fn main() -> Result<(), EpubToMdError> {
    // 获取命令行参数
    let args: Vec<String> = env::args().collect();

    // 检查参数数量
    if args.len() < 2 {
        eprintln!("{}", EpubToMdError::UsageError);
        return Err(EpubToMdError::UsageError);
    }

    // 获取输入和输出文件路径
    let epub_path = &args[1];
    let md_path = args.get(2).map(|s| s.as_str());

    // 执行转换
    if let Err(e) = convert_epub_to_md(epub_path, md_path) {
        eprintln!("错误: {}", e);
        return Err(e);
    }

    println!("EPUB转Markdown成功！");
    Ok(())
}
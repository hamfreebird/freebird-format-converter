#[cfg(windows)]
use std::env;
#[cfg(windows)]
use std::fs::{self, File};
#[cfg(windows)]
use std::io::{self, Write, BufRead, Read};
#[cfg(windows)]
use std::path::{Path, PathBuf};
#[cfg(windows)]
use std::process::Command;
#[cfg(windows)]
use which::which;
#[cfg(windows)]
use indicatif::{ProgressBar, ProgressStyle};

#[cfg(windows)]
const FFMPEG_DOWNLOAD_URL: &str =
    "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip";

/// 主入口点，处理命令行参数分发
#[cfg(windows)]
pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 && args[1] == "--install-ffmpeg" {
        // 新窗口执行安装逻辑
        if let Err(e) = install_ffmpeg() {
            eprintln!("Installation failed: {}", e);
            println!("\nPress any key to close this window...");
            io::stdin().read_line(&mut String::new())?;
            return Err(e);
        }
        println!("\nInstallation completed successfully.");
        println!("Press any key to close this window...");
        io::stdin().read_line(&mut String::new())?;
        return Ok(());
    }

    // 普通调用路径：检查并确保 ffmpeg 可用（可能启动安装窗口）
    ensure_ffmpeg()
}

/// 在 Windows 上检查 ffmpeg 是否可用，若不可用则引导用户在新窗口中下载安装。
#[cfg(windows)]
pub fn ensure_ffmpeg() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 检查 ffmpeg 是否已在 PATH 中
    if which("ffmpeg").is_ok() {
        println!("ffmpeg is already installed and found in PATH.");
        return Ok(());
    }

    println!("ffmpeg not found in PATH.");

    // 2. 询问用户是否要下载安装（在主窗口中进行）
    print!("Download and install ffmpeg? (Y/N): ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    if input != "y" && input != "yes" {
        println!("Installation cancelled.");
        return Ok(());
    }

    // 3. 检查是否已经安装但未加入 PATH（避免重复下载）
    let local_app_data = env::var("LOCALAPPDATA")
        .map_err(|_| "Unable to retrieve LOCALAPPDATA environment variable")?;
    let install_dir = PathBuf::from(local_app_data).join("ffmpeg");
    let bin_dir = install_dir.join("bin");
    let ffmpeg_exe = bin_dir.join("ffmpeg.exe");

    if ffmpeg_exe.exists() {
        println!("ffmpeg already exists at {}, adding to PATH...", install_dir.display());
        add_to_user_path(&bin_dir)?;
        println!("Added {} to user PATH.", bin_dir.display());
        println!("Please reopen your command prompt for PATH changes to take effect.");
        return Ok(());
    }

    // 4. 启动新窗口执行安装过程
    let exe = env::current_exe()?;
    let exe_str = exe.to_str().ok_or("Executable path contains invalid UTF-8")?;

    println!("Starting installation in a new window...");
    println!("You may close this window if you wish; the installation will continue.");

    // 使用 cmd /c start "title" 启动新窗口，注意用引号包裹路径以防空格
    let status = Command::new("cmd")
        .args(&[
            "/C",
            "start",
            "\"Installing ffmpeg\"",
            exe_str,
            "--install-ffmpeg",
        ])
        .spawn(); // 不等待子进程结束

    match status {
        Ok(_) => {
            println!("Installation window launched successfully.");
        }
        Err(e) => {
            eprintln!("Failed to launch installation window: {}", e);
            eprintln!("Falling back to installation in current window...");
            // 回退：在当前窗口直接执行安装
            install_ffmpeg()?;
        }
    }

    Ok(())
}

/// 执行实际的下载、解压、安装和 PATH 设置（在独立窗口中运行）
#[cfg(windows)]
fn install_ffmpeg() -> Result<(), Box<dyn std::error::Error>> {
    // 确定安装目录：%LOCALAPPDATA%\ffmpeg
    let local_app_data = env::var("LOCALAPPDATA")
        .map_err(|_| "Unable to retrieve LOCALAPPDATA environment variable")?;
    let install_dir = PathBuf::from(local_app_data).join("ffmpeg");
    let bin_dir = install_dir.join("bin");
    let ffmpeg_exe = bin_dir.join("ffmpeg.exe");

    // 如果已存在，直接设置 PATH 并返回
    if ffmpeg_exe.exists() {
        println!("ffmpeg already exists at {}, skipping download.", install_dir.display());
        add_to_user_path(&bin_dir)?;
        println!("Added {} to user PATH.", bin_dir.display());
        println!("Please reopen your command prompt for PATH changes to take effect.");
        return Ok(());
    }

    // 下载 ffmpeg 压缩包（带进度条）
    println!("Downloading ffmpeg, please wait...");
    let temp_dir = env::temp_dir().join("ffmpeg_install");
    fs::create_dir_all(&temp_dir)?;
    let zip_path = temp_dir.join("ffmpeg.zip");

    let client = reqwest::blocking::Client::new();
    let mut response = client
        .get(FFMPEG_DOWNLOAD_URL)
        .send()?
        .error_for_status()?;

    // 获取文件总大小用于进度条
    let total_size = response.content_length().unwrap_or(0);
    let pb = ProgressBar::new(total_size);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
        .unwrap()
        .progress_chars("#>-"));

    let mut file = File::create(&zip_path)?;
    let mut downloaded: u64 = 0;
    let mut buffer = [0; 8192];

    loop {
        let bytes_read = response.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        file.write_all(&buffer[..bytes_read])?;
        downloaded += bytes_read as u64;
        pb.set_position(downloaded);
    }
    pb.finish_with_message("Download completed");
    println!();

    // 解压到临时目录（带进度条）
    println!("Extracting...");
    let extract_dir = temp_dir.join("extract");
    fs::create_dir_all(&extract_dir)?;

    let zip_file = File::open(&zip_path)?;
    let mut archive = zip::ZipArchive::new(zip_file)?;
    let total_files = archive.len() as u64;

    let pb_extract = ProgressBar::new(total_files);
    pb_extract.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} files")
        .unwrap()
        .progress_chars("#>-"));

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = match file.enclosed_name() {
            Some(path) => extract_dir.join(path),
            None => continue,
        };

        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
            }
            let mut outfile = File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }

        // 设置文件权限（Unix），Windows 下忽略
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = file.unix_mode() {
                fs::set_permissions(&outpath, fs::Permissions::from_mode(mode))?;
            }
        }

        pb_extract.inc(1);
    }
    pb_extract.finish_with_message("Extraction completed");
    println!();

    // 查找解压后的 ffmpeg 目录
    let extracted_content: Vec<_> = fs::read_dir(&extract_dir)?
        .filter_map(Result::ok)
        .collect();
    if extracted_content.is_empty() {
        return Err("No files found after extraction".into());
    }

    let ffmpeg_root = if extracted_content.len() == 1 && extracted_content[0].path().is_dir() {
        extracted_content[0].path()
    } else {
        extracted_content
            .iter()
            .find(|entry| entry.path().join("bin").exists())
            .map(|e| e.path())
            .ok_or("Could not locate ffmpeg directory in extracted contents")?
    };

    // 移动到安装目录
    if install_dir.exists() {
        fs::remove_dir_all(&install_dir)?;
    }
    fs::create_dir_all(&install_dir)?;

    copy_dir_all(&ffmpeg_root, &install_dir)?;

    // 清理临时文件
    let _ = fs::remove_dir_all(&temp_dir);

    println!("ffmpeg has been installed to {}", install_dir.display());

    // 将 bin 目录添加到用户 PATH
    add_to_user_path(&bin_dir)?;
    println!("Added {} to user PATH.", bin_dir.display());
    println!("Please reopen your command prompt for PATH changes to take effect.");

    Ok(())
}

/// 将一个目录路径追加到当前用户的 PATH 环境变量中。
#[cfg(windows)]
fn add_to_user_path(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // 获取当前用户 PATH
    let output = Command::new("cmd")
        .args(&["/C", "reg", "query", "HKCU\\Environment", "/v", "PATH"])
        .output()?;

    let current_path = if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout
            .lines()
            .find(|line| line.contains("PATH"))
            .and_then(|line| line.split_whitespace().last())
            .map(|s| s.to_string())
            .unwrap_or_default()
    } else {
        String::new()
    };

    let dir_str = dir.to_string_lossy().to_string();
    let paths: Vec<&str> = current_path.split(';').collect();
    if paths.iter().any(|p| Path::new(p) == dir) {
        println!("Directory already present in PATH, skipping.");
        return Ok(());
    }

    let new_path = if current_path.is_empty() {
        dir_str
    } else {
        format!("{};{}", current_path, dir_str)
    };

    let status = Command::new("setx")
        .args(&["PATH", &new_path])
        .status()?;

    if !status.success() {
        return Err("Failed to set PATH environment variable".into());
    }
    Ok(())
}

/// 递归复制目录内容。
#[cfg(windows)]
fn copy_dir_all(src: &Path, dst: &Path) -> io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

// 非 Windows 平台提供一个空实现或 panic
#[cfg(not(windows))]
pub fn ensure_ffmpeg() -> Result<(), Box<dyn std::error::Error>> {
    panic!("This function is only supported on Windows.");
}
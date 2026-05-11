use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set("FileDescription", env!("CARGO_PKG_DESCRIPTION"));
        res.set("ProductName", "Rust MCP Server");
        res.set("FileVersion", env!("CARGO_PKG_VERSION"));
        res.set("ProductVersion", env!("CARGO_PKG_VERSION"));
        res.set("OriginalFilename", "rust-mcp-server.exe");
        res.set("InternalName", "rust-mcp-server");
        res.set("LegalCopyright", "Copyright (c) MCP Server Team");
        res.set_icon("assets/icon.ico");
        res.compile().expect("Failed to compile Windows resources");

        let target = std::env::var("TARGET").unwrap_or_default();
        if target.contains("win7") {
            inject_yy_thunks(&target);
        }
    }

    prepare_pdfium_assets()?;

    Ok(())
}

#[cfg(windows)]
fn inject_yy_thunks(target: &str) {
    // ── Architecture selection ──
    let (arch, platform_dir, yy_obj) = if target.contains("x86_64") {
        ("x64", "x64", "YY_Thunks_for_Win7.obj")
    } else {
        ("x86", "x86", "YY_Thunks_for_Win7_x86.obj")
    };

    // ── 1. VC-LTL5 v5.3.1: CRT replacement (ucrt/vcruntime → msvcrt.dll) ──
    // Embedded at assets/vc-ltl/{platform}/ — release libs only (no debug).
    // 6.0.6000.0 (Vista-level CRT), compatible with Win7+.
    println!(
        "cargo:rustc-link-search=native=assets/vc-ltl/{platform_dir}",
        platform_dir = platform_dir
    );

    // ── 2. YY-Thunks v1.2.1: Win8+ Windows API stubs ──
    // Embedded at assets/yy-thunks/
    println!("cargo:rustc-link-search=native=assets/yy-thunks");
    println!("cargo:rustc-link-arg={obj}", obj = yy_obj);

    // Suppress default kernel32.lib then re-link it AFTER YY-Thunks obj
    // so the thunked symbols take precedence.
    println!("cargo:rustc-link-arg=/NODEFAULTLIB:kernel32.lib");
    println!("cargo:rustc-link-arg=kernel32.lib");

    println!("cargo:warning=VC-LTL5 v5.3.1 + YY-Thunks v1.2.1 ({arch}) injected for Win7 compatibility");
}

fn prepare_pdfium_assets() -> Result<(), Box<dyn std::error::Error>> {
    let target = std::env::var("TARGET").unwrap_or_default();
    let (lib_name, archive_name) = get_pdfium_filename(&target);
    let assets_dir = Path::new("assets/pdfium");
    let lib_path = assets_dir.join(&lib_name);
    let zst_path = assets_dir.join(format!("{}.zst", lib_name));
    let archive_path = assets_dir.join(&archive_name);

    // Already have the library — compress to .zst for embedding
    if lib_path.exists() {
        compress_if_needed(&lib_path, &zst_path)?;
        return Ok(());
    }

    // Already have the .zst (e.g. from a previous build or manual placement)
    if zst_path.exists() {
        return Ok(());
    }

    // Try extracting from archive if present
    if archive_path.exists() {
        println!("cargo:warning=Extracting {lib_name} from {archive_name}...");
        extract_pdfium(&archive_path, assets_dir, &target, &lib_name)?;
        if lib_path.exists() {
            compress_if_needed(&lib_path, &zst_path)?;
            return Ok(());
        }
    }

    // Try downloading
    println!("cargo:warning={archive_name} not found, attempting auto-download...");
    if download_pdfium(&archive_path, &target, &archive_name).is_ok() && archive_path.exists() {
        extract_pdfium(&archive_path, assets_dir, &target, &lib_name)?;
    }

    if lib_path.exists() {
        compress_if_needed(&lib_path, &zst_path)?;
    } else {
        let display = assets_dir.display();
        println!("cargo:warning====================================================================");
        println!("cargo:warning=  {lib_name} NOT FOUND — PDF page rendering will be limited.");
        println!("cargo:warning=  The fallback extracts embedded images from PDFs instead.");
        println!("cargo:warning=");
        println!("cargo:warning=  To enable full PDF rendering, place the library in assets/pdfium/:");
        println!("cargo:warning=    Windows : {display}\\pdfium.dll");
        println!("cargo:warning=    Linux   : {display}/libpdfium.so");
        println!("cargo:warning=    macOS   : {display}/libpdfium.dylib");
        println!("cargo:warning=");
        println!("cargo:warning=  Or place the .tgz archive: {display}\\{archive_name}");
        println!("cargo:warning=");
        println!("cargo:warning=  Download: https://github.com/bblanchon/pdfium-binaries/releases");
        println!("cargo:warning====================================================================");
    }

    Ok(())
}

fn get_pdfium_filename(target: &str) -> (String, String) {
    if target.contains("windows") {
        ("pdfium.dll".to_string(), "pdfium-v8-win-x64.tgz".to_string())
    } else if target.contains("linux") {
        ("libpdfium.so".to_string(), "pdfium-v8-linux-x64.tgz".to_string())
    } else if target.contains("apple") {
        ("libpdfium.dylib".to_string(), "pdfium-mac-x64.tgz".to_string())
    } else {
        ("libpdfium.so".to_string(), "pdfium-v8-linux-x64.tgz".to_string())
    }
}

fn find_file_in_dir(dir: &Path, name: &str) -> Option<std::path::PathBuf> {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(found) = find_file_in_dir(&path, name) {
                    return Some(found);
                }
            } else if path.file_name().map(|n| n == name).unwrap_or(false) {
                return Some(path);
            }
        }
    }
    None
}

fn compress_if_needed(lib_path: &Path, zst_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Only recompress if .zst is missing or older than source
    let need_compress = !zst_path.exists()
        || lib_path.metadata()?.modified()? > zst_path.metadata()?.modified()?;

    if need_compress {
        let data = std::fs::read(lib_path)?;
        let compressed = zstd::encode_all(&data[..], 22)?;
        std::fs::write(zst_path, compressed)?;
        println!("cargo:warning=Compressed {} -> {} ({:.1}%)",
            lib_path.file_name().unwrap().to_string_lossy(),
            zst_path.file_name().unwrap().to_string_lossy(),
            100.0 * zst_path.metadata()?.len() as f64 / lib_path.metadata()?.len() as f64
        );
    }

    Ok(())
}

fn download_pdfium(
    archive_path: &Path,
    target: &str,
    archive_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let version = "7763";
    let url = format!(
        "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F{version}/{archive_name}"
    );

    // Try curl first (available on most platforms)
    let ok = std::process::Command::new("curl")
        .args([
            "-L", "-f", "-o", archive_path.to_str().unwrap(),
            "--connect-timeout", "30", "--max-time", "300",
            &url,
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if ok {
        return Ok(());
    }

    // Windows: fall back to PowerShell
    if target.contains("windows") {
        let _ = std::fs::remove_file(archive_path);
        let ok = std::process::Command::new("powershell")
            .args([
                "-NoProfile", "-Command",
                &format!(
                    "try {{ Invoke-WebRequest -Uri '{}' -OutFile '{}' -UseBasicParsing; exit 0 }} catch {{ exit 1 }}",
                    url, archive_path.display()
                ),
            ])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok {
            return Ok(());
        }
    }

    // Linux: fall back to wget
    if target.contains("linux") {
        let _ = std::fs::remove_file(archive_path);
        let ok = std::process::Command::new("wget")
            .args(["-q", "-O", archive_path.to_str().unwrap(), &url])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok {
            return Ok(());
        }
    }

    let _ = std::fs::remove_file(archive_path);
    Err("Download failed — check network connection or download manually".into())
}

fn extract_pdfium(
    archive_path: &Path,
    assets_dir: &Path,
    target: &str,
    lib_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let temp_extract = tempfile::tempdir()?;
    let extract_dir = temp_extract.path();

    let ok = std::process::Command::new("tar")
        .args(["-xzf", archive_path.to_str().unwrap(), "-C", extract_dir.to_str().unwrap()])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if !ok {
        return Ok(());
    }

    let lib_in_archive = if target.contains("windows") {
        extract_dir.join("bin/pdfium.dll")
    } else {
        extract_dir.join("lib/libpdfium.so")
    };

    let lib_path = if lib_in_archive.exists() {
        Some(lib_in_archive)
    } else {
        find_file_in_dir(extract_dir, lib_name)
    };

    if let Some(path) = lib_path {
        let dest = assets_dir.join(lib_name);
        std::fs::copy(&path, &dest)?;
        println!("cargo:warning=Extracted {} -> {}", lib_name, dest.display());
        let zst_path = assets_dir.join(format!("{}.zst", lib_name));
        compress_if_needed(&dest, &zst_path)?;
    }

    Ok(())
}

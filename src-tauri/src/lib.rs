use std::path::{Path, PathBuf};

use tauri::ipc::Channel;
use tauri::Manager;

const SUPPORTED_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "bmp", "tiff", "tif", "webp", "ico", "pnm", "pbm", "pgm",
    "ppm", "pam", "tga", "dds", "ff", "qoi",
];

#[derive(serde::Serialize)]
pub struct ImageInfo {
    path: String,
    name: String,
    size: u64,
    width: u32,
    height: u32,
    format: String,
}

fn has_supported_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| SUPPORTED_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

fn read_image_info(path: &Path) -> Result<ImageInfo, String> {
    let metadata = std::fs::metadata(path)
        .map_err(|e| format!("Failed to read metadata for {}: {}", path.display(), e))?;

    let reader = image::ImageReader::open(path)
        .map_err(|e| format!("Failed to open {}: {}", path.display(), e))?
        .with_guessed_format()
        .map_err(|e| format!("Failed to detect format for {}: {}", path.display(), e))?;

    let format = reader
        .format()
        .map(|f| format!("{:?}", f))
        .unwrap_or_else(|| "Unknown".to_string());

    let (width, height) = reader
        .into_dimensions()
        .map_err(|e| format!("Failed to read dimensions for {}: {}", path.display(), e))?;

    Ok(ImageInfo {
        path: path.to_string_lossy().into_owned(),
        name: path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        size: metadata.len(),
        width,
        height,
        format,
    })
}

fn scan_folder(folder: &Path) -> Result<Vec<ImageInfo>, String> {
    let entries =
        std::fs::read_dir(folder).map_err(|e| format!("Failed to read folder: {}", e))?;

    let mut images = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();

        // Nested folders are not traversed
        if path.is_dir() {
            continue;
        }

        if !has_supported_extension(&path) {
            continue;
        }

        // Silently skip files that fail to load (corrupted, misnamed, etc.)
        if let Ok(info) = read_image_info(&path) {
            images.push(info);
        }
    }

    Ok(images)
}

#[tauri::command]
fn load_image_file(path: String) -> Result<ImageInfo, String> {
    let path = Path::new(&path);

    if !path.exists() {
        return Err("File does not exist".to_string());
    }
    if !path.is_file() {
        return Err("Path is not a file".to_string());
    }
    if !has_supported_extension(path) {
        return Err(format!(
            "Unsupported file type. Supported extensions: {}",
            SUPPORTED_EXTENSIONS.join(", ")
        ));
    }

    read_image_info(path)
}

#[tauri::command]
fn load_images_from_folder(folder_path: String) -> Result<Vec<ImageInfo>, String> {
    let folder = Path::new(&folder_path);

    if !folder.exists() {
        return Err("Folder does not exist".to_string());
    }
    if !folder.is_dir() {
        return Err("Path is not a folder".to_string());
    }

    let mut images = scan_folder(folder)?;

    if images.is_empty() {
        return Err("No compatible images found in the selected folder".to_string());
    }

    images.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    Ok(images)
}

#[tauri::command]
fn load_dropped_paths(paths: Vec<String>) -> Result<Vec<ImageInfo>, String> {
    let mut images: Vec<ImageInfo> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for p in &paths {
        let path = Path::new(p);

        let found = if path.is_dir() {
            scan_folder(path)?
        } else if has_supported_extension(path) {
            match read_image_info(path) {
                Ok(info) => vec![info],
                Err(_) => Vec::new(),
            }
        } else {
            Vec::new()
        };

        for info in found {
            if seen.insert(info.path.clone()) {
                images.push(info);
            }
        }
    }

    if images.is_empty() {
        return Err("No compatible images in the dropped items".to_string());
    }

    images.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    Ok(images)
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ResizeSettings {
    width: u32,
    height: u32,
}

#[derive(serde::Serialize)]
struct ResizeProgress {
    current: usize,
    total: usize,
    info: ImageInfo,
}

fn settings_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to resolve config dir: {}", e))?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create config dir: {}", e))?;
    Ok(dir.join("resize-settings.json"))
}

#[tauri::command]
fn load_resize_settings(app: tauri::AppHandle) -> Result<Option<ResizeSettings>, String> {
    let path = settings_path(&app)?;
    if !path.exists() {
        return Ok(None);
    }
    let data = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read settings: {}", e))?;
    serde_json::from_str(&data)
        .map(Some)
        .map_err(|e| format!("Failed to parse settings: {}", e))
}

#[tauri::command]
fn save_resize_settings(app: tauri::AppHandle, settings: ResizeSettings) -> Result<(), String> {
    let path = settings_path(&app)?;
    let data = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;
    std::fs::write(&path, data).map_err(|e| format!("Failed to write settings: {}", e))
}

fn backup_original(path: &Path) -> Result<PathBuf, String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("No parent folder for {}", path.display()))?;
    let name = path
        .file_name()
        .ok_or_else(|| format!("No file name for {}", path.display()))?;

    let original_dir = parent.join("original");
    std::fs::create_dir_all(&original_dir)
        .map_err(|e| format!("Failed to create original folder: {}", e))?;

    let backup = original_dir.join(name);
    if backup.exists() {
        return Err(format!(
            "{} already exists in the original folder — refusing to overwrite a backup",
            backup.display()
        ));
    }
    std::fs::rename(path, &backup)
        .map_err(|e| format!("Failed to move {} to original folder: {}", path.display(), e))?;

    Ok(backup)
}

fn resize_from_backup(backup: &Path, dest: &Path, width: u32, height: u32) -> Result<(), String> {
    let img = image::open(backup)
        .map_err(|e| format!("Failed to open {}: {}", backup.display(), e))?;
    img.resize_exact(width, height, image::imageops::FilterType::Triangle)
        .save(dest)
        .map_err(|e| format!("Failed to save resized {}: {}", dest.display(), e))
}

#[tauri::command]
async fn resize_images(
    paths: Vec<String>,
    width: u32,
    height: u32,
    on_progress: Channel<ResizeProgress>,
) -> Result<(), String> {
    if width == 0 || height == 0 {
        return Err("Width and height must be greater than zero".to_string());
    }
    if paths.is_empty() {
        return Err("No images to resize".to_string());
    }

    tauri::async_runtime::spawn_blocking(move || {
        // Move every original into original/ first — renames are cheap, and it
        // guarantees all sources are safe before any resized file is written
        let mut jobs = Vec::with_capacity(paths.len());
        for p in &paths {
            let dest = PathBuf::from(p);
            let backup = backup_original(&dest)?;
            jobs.push((backup, dest));
        }

        let total = jobs.len();
        for (i, (backup, dest)) in jobs.iter().enumerate() {
            resize_from_backup(backup, dest, width, height)?;

            // Re-read the written file so the frontend gets the real new size on disk
            let info = read_image_info(dest)?;
            let _ = on_progress.send(ResizeProgress {
                current: i + 1,
                total,
                info,
            });
        }
        Ok(())
    })
    .await
    .map_err(|e| format!("Resize task failed: {}", e))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_then_resize_preserves_original_and_writes_resized() {
        let dir = std::env::temp_dir().join(format!("resize-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.png");

        image::RgbImage::new(100, 50).save(&path).unwrap();

        let backup = backup_original(&path).unwrap();
        assert_eq!(backup, dir.join("original").join("test.png"));
        assert!(!path.exists());
        assert_eq!(image::image_dimensions(&backup).unwrap(), (100, 50));

        resize_from_backup(&backup, &path, 50, 25).unwrap();
        assert_eq!(image::image_dimensions(&path).unwrap(), (50, 25));
        assert_eq!(image::image_dimensions(&backup).unwrap(), (100, 50));

        // A second backup of the same name must refuse to overwrite the first
        assert!(backup_original(&path).is_err());
        assert!(backup.exists());

        std::fs::remove_dir_all(&dir).unwrap();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            load_image_file,
            load_images_from_folder,
            load_dropped_paths,
            resize_images,
            load_resize_settings,
            save_resize_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

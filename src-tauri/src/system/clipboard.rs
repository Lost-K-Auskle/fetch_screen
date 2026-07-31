use std::path::Path;
use arboard::Clipboard;

/// 将图像文件复制到剪贴板
pub fn copy_image_to_clipboard(image_path: &Path) -> Result<(), String> {
    let img = image::open(image_path)
        .map_err(|e| format!("加载图片失败: {}", e))?;

    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    let raw = rgba.into_raw();

    // Use arboard crate for clipboard
    let mut clipboard = Clipboard::new()
        .map_err(|e| format!("打开剪贴板失败: {}", e))?;

    let img_data = arboard::ImageData {
        width: width as usize,
        height: height as usize,
        bytes: raw.into(),
    };

    clipboard.set_image(img_data)
        .map_err(|e| format!("复制图片到剪贴板失败: {}", e))?;

    Ok(())
}

/// 将文本复制到剪贴板
pub fn copy_text(text: &str) -> Result<(), String> {
    let mut clipboard = Clipboard::new()
        .map_err(|e| format!("打开剪贴板失败: {}", e))?;

    clipboard.set_text(text.to_string())
        .map_err(|e| format!("复制文本到剪贴板失败: {}", e))?;

    Ok(())
}

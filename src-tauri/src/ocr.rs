//! Optional OCR — read text out of an image using the OS OCR engine. On Windows
//! this drives the built-in Windows.Media.Ocr engine through PowerShell/WinRT, so
//! there's no extra Rust dependency to ship. Gated behind the "OCR" extension
//! toggle; failures degrade to a friendly message rather than a crash.

use anyhow::{anyhow, Result};

#[cfg(windows)]
const OCR_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
$Path = $env:PEBBLE_OCR_PATH
$null = [Windows.Media.Ocr.OcrEngine, Windows.Foundation, ContentType = WindowsRuntime]
$null = [Windows.Graphics.Imaging.BitmapDecoder, Windows.Foundation, ContentType = WindowsRuntime]
$null = [Windows.Storage.StorageFile, Windows.Foundation, ContentType = WindowsRuntime]
Add-Type -AssemblyName System.Runtime.WindowsRuntime | Out-Null
$asTask = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object { $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' })[0]
function Await($op, $t) { $task = $asTask.MakeGenericMethod($t).Invoke($null, @($op)); $task.Wait(-1) | Out-Null; $task.Result }
$file = Await ([Windows.Storage.StorageFile]::GetFileFromPathAsync($Path)) ([Windows.Storage.StorageFile])
$stream = Await ($file.OpenReadAsync()) ([Windows.Storage.Streams.IRandomAccessStreamWithContentType])
$decoder = Await ([Windows.Graphics.Imaging.BitmapDecoder]::CreateAsync($stream)) ([Windows.Graphics.Imaging.BitmapDecoder])
$bitmap = Await ($decoder.GetSoftwareBitmapAsync()) ([Windows.Graphics.Imaging.SoftwareBitmap])
$engine = [Windows.Media.Ocr.OcrEngine]::TryCreateFromUserProfileLanguages()
if ($null -eq $engine) { [Console]::Error.WriteLine('no-ocr-language'); exit 2 }
$result = Await ($engine.RecognizeAsync($bitmap)) ([Windows.Media.Ocr.OcrResult])
[Console]::Out.Write($result.Text)
"#;

/// Extract text from an image at `path`. Read-only.
pub fn image_text(path: &str) -> Result<String> {
    #[cfg(windows)]
    {
        let out = std::process::Command::new("powershell")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", OCR_SCRIPT])
            .env("PEBBLE_OCR_PATH", path)
            .output()
            .map_err(|e| anyhow!("couldn't run OCR: {e}"))?;
        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if text.is_empty() {
                Ok("(no readable text found in this image)".to_string())
            } else {
                Ok(text)
            }
        } else {
            let err = String::from_utf8_lossy(&out.stderr);
            if err.contains("no-ocr-language") {
                Err(anyhow!(
                    "Windows has no OCR language pack for this. Add one in Windows Settings → \
                     Time & language → Language → (your language) → Optional features → OCR."
                ))
            } else {
                Err(anyhow!(
                    "couldn't read text from that image (Windows OCR needs Windows 10/11)."
                ))
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        Err(anyhow!("OCR is only available on Windows."))
    }
}

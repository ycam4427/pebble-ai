//! Best-effort hardware capability detection, used to recommend a model that
//! suits the user's PC. Never fatal — falls back to "light" if detection fails.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Default)]
pub struct SystemInfo {
    pub ram_gb: f64,
    pub cpu_cores: u32,
    pub gpu_name: String,
    pub vram_gb: f64,
    /// "light" | "balanced" | "powerful"
    pub tier: String,
    pub recommended_model: String,
    pub reason: String,
}

#[derive(Deserialize, Default)]
struct RawHw {
    #[serde(default)]
    ram_gb: f64,
    #[serde(default)]
    cpu_cores: u32,
    #[serde(default)]
    gpu_name: Option<String>,
    #[serde(default)]
    vram_gb: f64,
}

pub fn gather() -> SystemInfo {
    let raw = query_hw().unwrap_or_default();
    let gpu = raw.gpu_name.unwrap_or_default();
    let (tier, model, reason) = recommend(raw.vram_gb, raw.ram_gb, &gpu);
    SystemInfo {
        ram_gb: raw.ram_gb,
        cpu_cores: raw.cpu_cores,
        gpu_name: gpu,
        vram_gb: raw.vram_gb,
        tier: tier.to_string(),
        recommended_model: model.to_string(),
        reason,
    }
}

#[cfg(windows)]
fn query_hw() -> Option<RawHw> {
    // Reads true VRAM from the GPU driver registry key (Win32_VideoController
    // caps AdapterRAM at ~4 GB, so prefer qwMemorySize and fall back if needed).
    let script = r#"
$cs = Get-CimInstance Win32_ComputerSystem
$ramGB = [math]::Round($cs.TotalPhysicalMemory/1GB,1)
$cores = [int]$env:NUMBER_OF_PROCESSORS
$best=$null;$bestVram=0
try {
  Get-ChildItem 'HKLM:\SYSTEM\CurrentControlSet\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}' -ErrorAction Stop | ForEach-Object {
    $name=(Get-ItemProperty $_.PSPath -Name 'DriverDesc' -ErrorAction SilentlyContinue).DriverDesc
    $mem=(Get-ItemProperty $_.PSPath -Name 'HardwareInformation.qwMemorySize' -ErrorAction SilentlyContinue).'HardwareInformation.qwMemorySize'
    if ($mem -and [double]$mem -gt $bestVram) { $bestVram=[double]$mem; $best=$name }
  }
} catch {}
if (-not $best) {
  $vc = Get-CimInstance Win32_VideoController | Sort-Object AdapterRAM -Descending | Select-Object -First 1
  if ($vc) { $best=$vc.Name; $bestVram=[double]$vc.AdapterRAM }
}
$vramGB = if ($bestVram -gt 0) { [math]::Round($bestVram/1GB,1) } else { 0 }
[pscustomobject]@{ ram_gb=$ramGB; cpu_cores=$cores; gpu_name=$best; vram_gb=$vramGB } | ConvertTo-Json -Compress
"#;
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    serde_json::from_str::<RawHw>(text.trim()).ok()
}

#[cfg(not(windows))]
fn query_hw() -> Option<RawHw> {
    None
}

/// Pick a tier + a sensible model. LLMs run best on VRAM; CPU-only falls back to RAM.
fn recommend(vram: f64, ram: f64, gpu: &str) -> (&'static str, &'static str, String) {
    let gpu_label = if gpu.trim().is_empty() {
        "Your GPU".to_string()
    } else {
        gpu.trim().to_string()
    };
    if vram >= 11.0 {
        (
            "powerful",
            "qwen2.5:14b",
            format!("{gpu_label} has ~{vram:.0} GB VRAM — plenty for a big, capable model."),
        )
    } else if vram >= 7.0 {
        (
            "powerful",
            "llama3.1:8b",
            format!("{gpu_label} (~{vram:.0} GB VRAM) can comfortably run an 8B model — much smarter than the tiny default."),
        )
    } else if vram >= 5.0 {
        (
            "balanced",
            "qwen2.5:7b",
            format!("{gpu_label} (~{vram:.0} GB VRAM) can run a 7B model — a nice step up."),
        )
    } else if vram >= 3.5 {
        (
            "balanced",
            "llama3.1:8b",
            format!("{gpu_label} (~{vram:.0} GB VRAM) can likely handle an 8B model (might be a touch tight)."),
        )
    } else if ram >= 16.0 {
        (
            "balanced",
            "llama3.1:8b",
            format!("No big GPU detected, but with ~{ram:.0} GB RAM you can run an 8B model on the CPU — smarter, just slower."),
        )
    } else {
        (
            "light",
            "llama3.2:3b",
            "Your PC runs best on the lightweight default for snappy replies.".to_string(),
        )
    }
}

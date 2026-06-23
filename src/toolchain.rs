use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy)]
pub struct ToolchainSpec {
    pub key: &'static str,
    pub fallback: &'static str,
    pub candidates: &'static [&'static str],
}

const TOOLCHAIN_SPECS: &[ToolchainSpec] = &[
    ToolchainSpec {
        key: "cargo",
        fallback: "cargo",
        candidates: &["cargo"],
    },
    ToolchainSpec {
        key: "npm",
        fallback: "npm",
        candidates: &["npm"],
    },
    ToolchainSpec {
        key: "python",
        fallback: "python",
        candidates: &["python", "python3", "py"],
    },
    ToolchainSpec {
        key: "java",
        fallback: "java",
        candidates: &["java"],
    },
    ToolchainSpec {
        key: "javac",
        fallback: "javac",
        candidates: &["javac"],
    },
    ToolchainSpec {
        key: "c",
        fallback: "gcc",
        candidates: &["gcc", "clang"],
    },
    ToolchainSpec {
        key: "cpp",
        fallback: "g++",
        candidates: &["g++", "clang++"],
    },
    ToolchainSpec {
        key: "go",
        fallback: "go",
        candidates: &["go"],
    },
    ToolchainSpec {
        key: "dotnet",
        fallback: "dotnet",
        candidates: &["dotnet"],
    },
    ToolchainSpec {
        key: "julia",
        fallback: "julia",
        candidates: &["julia"],
    },
    ToolchainSpec {
        key: "rscript",
        fallback: "Rscript",
        candidates: &["Rscript"],
    },
    ToolchainSpec {
        key: "pdflatex",
        fallback: "pdflatex",
        candidates: &["pdflatex"],
    },
    ToolchainSpec {
        key: "tectonic",
        fallback: "tectonic",
        candidates: &["tectonic"],
    },
];

fn spec_for_key(key: &str) -> Option<&'static ToolchainSpec> {
    TOOLCHAIN_SPECS.iter().find(|spec| spec.key == key)
}

fn looks_like_path(value: &str) -> bool {
    let path = Path::new(value);
    path.is_absolute() || value.contains('\\') || value.contains('/')
}

fn normalize_existing_path(path: &Path) -> Option<String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().ok()?.join(path)
    };
    if !absolute.exists() {
        return None;
    }
    let canonical = fs::canonicalize(&absolute).unwrap_or(absolute);
    Some(canonical.to_string_lossy().to_string())
}

fn detect_from_candidates<'a>(candidates: impl IntoIterator<Item = &'a str>) -> Option<String> {
    for candidate in candidates {
        if candidate.trim().is_empty() {
            continue;
        }
        if looks_like_path(candidate) {
            if let Some(resolved) = normalize_existing_path(Path::new(candidate)) {
                return Some(resolved);
            }
            continue;
        }
        if let Ok(path) = which::which(candidate) {
            return Some(path.to_string_lossy().to_string());
        }
    }
    None
}

pub fn auto_detect_toolchain_paths() -> BTreeMap<String, String> {
    TOOLCHAIN_SPECS
        .iter()
        .map(|spec| {
            let value = detect_from_candidates(spec.candidates.iter().copied())
                .unwrap_or_else(|| spec.fallback.to_string());
            (spec.key.to_string(), value)
        })
        .collect()
}

pub fn default_toolchain_command(key: &str) -> String {
    spec_for_key(key)
        .map(|spec| spec.fallback.to_string())
        .unwrap_or_else(|| key.to_string())
}

pub fn resolve_toolchain_value(key: &str, value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    if looks_like_path(trimmed) {
        return normalize_existing_path(Path::new(trimmed));
    }

    let mut candidates = vec![trimmed];
    if let Some(spec) = spec_for_key(key) {
        let can_expand = spec.fallback.eq_ignore_ascii_case(trimmed)
            || spec
                .candidates
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(trimmed));
        if can_expand {
            for candidate in spec.candidates {
                if !candidate.eq_ignore_ascii_case(trimmed) {
                    candidates.push(candidate);
                }
            }
        }
    }

    detect_from_candidates(candidates.into_iter())
}

pub fn normalize_toolchain_paths(mut incoming: BTreeMap<String, String>) -> BTreeMap<String, String> {
    let mut defaults = auto_detect_toolchain_paths();
    for (key, value) in incoming.iter_mut() {
        let normalized_key = key.trim().to_ascii_lowercase();
        if normalized_key.is_empty() {
            continue;
        }
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }
        let normalized_value = if let Some(resolved) = resolve_toolchain_value(&normalized_key, trimmed) {
            resolved
        } else if let Some(detected) = defaults.get(&normalized_key).cloned() {
            detected
        } else {
            trimmed.to_string()
        };
        defaults.insert(normalized_key, normalized_value);
    }
    defaults
}

pub fn command_is_available(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return false;
    }
    if looks_like_path(trimmed) {
        return normalize_existing_path(Path::new(trimmed)).is_some();
    }
    which::which(trimmed).is_ok()
}

pub fn detect_toolchain_executable(key: &str) -> Option<String> {
    spec_for_key(key).and_then(|spec| detect_from_candidates(spec.candidates.iter().copied()))
}

pub(crate) fn package_target_path(package_path: &str, target: &str) -> Option<String> {
    let target = target.trim();
    if target.is_empty() || target.contains('*') {
        return None;
    }
    let target = target.replace('\\', "/");
    if target.starts_with('/') || windows_absolute_path(&target) {
        return None;
    }

    let base = normalize_rel_path(package_path);
    let mut parts = if base == "." || base.is_empty() {
        Vec::new()
    } else {
        base.split('/').map(str::to_string).collect::<Vec<_>>()
    };
    let min_depth = parts.len();

    for part in target.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.len() <= min_depth {
                    return None;
                }
                parts.pop();
            }
            other => parts.push(other.to_string()),
        }
    }

    if parts.is_empty() {
        Some(".".to_string())
    } else {
        Some(parts.join("/"))
    }
}

fn windows_absolute_path(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && bytes[2] == b'/'
}

pub(crate) fn package_public_target_candidates(package_path: &str, target: &str) -> Vec<String> {
    let Some(base) = package_target_path(package_path, target) else {
        return Vec::new();
    };
    let mut out = vec![base.clone()];
    if Path::new(&base).extension().is_none() {
        for ext in ["ts", "tsx", "js", "jsx", "mjs", "cjs", "d.ts"] {
            out.push(format!("{base}.{ext}"));
        }
        for index in ["index.ts", "index.tsx", "index.js", "index.jsx"] {
            out.push(normalize_rel_path(&format!("{base}/{index}")));
        }
    }
    out
}

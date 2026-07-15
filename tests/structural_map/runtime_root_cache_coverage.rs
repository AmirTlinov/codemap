// Responsibility: runtime-root-cache-coverage-integrity
#[test]
fn runtime_root_cache_preserves_the_cold_route_horizon_on_warm_read() {
    let (repo, cache) = runtime_root_cache_fixture();

    let cold = run_markdown(repo.path(), cache.path(), &["runtime", "."]);
    let cold_artifact = runtime_root_cache_json(cache.path());
    let warm = run_markdown(repo.path(), cache.path(), &["runtime", "."]);
    let warm_artifact = runtime_root_cache_json(cache.path());

    assert_lens_markdown_eq(
        &cold,
        &warm,
        "the warm runtime root must preserve the exact cold route horizon",
    );
    assert_eq!(
        warm_artifact["report"]["observations"], cold_artifact["report"]["observations"],
        "the complete persisted runtime ledger must survive a warm cache read unchanged",
    );
    assert_eq!(
        cold_artifact["report"]["observations"]["horizons"]
            .as_array()
            .expect("runtime group horizons")
            .len(),
        9,
        "the root cache must persist every runtime group horizon"
    );
}

#[test]
fn runtime_root_cache_body_hash_is_canonical_across_json_object_order() {
    let (repo, cache) = runtime_root_cache_fixture();
    let expected_output = run_markdown(repo.path(), cache.path(), &["runtime", "."]);
    let path = lens_artifact_path(cache.path(), "runtime-root.json");
    let original = fs::read_to_string(&path).expect("runtime root cache");
    let value: Value = serde_json::from_str(&original).expect("runtime root cache json");
    let reordered = format!(
        "{}\n",
        serde_json::to_string_pretty(&value).expect("reordered runtime cache")
    );
    assert_ne!(
        reordered, original,
        "the test must actually reorder JSON object keys"
    );
    assert_eq!(
        runtime_report_digest(&reordered),
        value["report_sha256"]
            .as_str()
            .expect("runtime report sha256"),
        "the body hash must bind semantic JSON independently of object order"
    );
    fs::write(&path, &reordered).expect("write reordered runtime root cache");

    let warm_output = run_markdown(repo.path(), cache.path(), &["runtime", "."]);
    assert_lens_markdown_eq(
        &expected_output,
        &warm_output,
        "an order-only JSON rewrite must remain a valid warm cache hit",
    );
    assert_eq!(
        fs::read_to_string(&path).expect("runtime root cache after warm read"),
        reordered,
        "a valid reordered artifact must not be repaired as corruption"
    );
}

#[test]
fn runtime_root_cache_dangling_ledger_with_a_coherent_body_hash_misses_and_rebuilds() {
    assert_runtime_root_corruption_rebuilds(
        corrupt_runtime_horizon_certificate_reference,
        true,
        "a dangling route certificate must fail semantic cache validation",
    );
}

#[test]
fn runtime_root_cache_route_list_without_a_matching_horizon_misses_and_rebuilds() {
    assert_runtime_root_corruption_rebuilds(
        remove_cached_runtime_routes,
        true,
        "a route list/horizon mismatch must fail semantic cache validation",
    );
}

#[test]
fn runtime_root_cache_coherent_route_mutation_with_a_stale_body_hash_misses_and_rebuilds() {
    assert_runtime_root_corruption_rebuilds(
        forge_cached_runtime_route,
        false,
        "a coherent route/count mutation must still fail the report body hash",
    );
}

#[test]
fn runtime_root_readable_honors_no_cache_without_creating_an_external_artifact() {
    let (repo, cache_parent) = runtime_root_cache_fixture();
    let absent_cache = cache_parent.path().join("disabled");
    assert!(
        !absent_cache.exists(),
        "disabled cache fixture starts absent"
    );

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", &absent_cache)
        .env("CODEMAP_NO_CACHE", "1")
        .args(["runtime", "."])
        .output()
        .expect("no-cache runtime should run");
    assert!(
        output.status.success(),
        "no-cache runtime failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let markdown = String::from_utf8(output.stdout).expect("runtime markdown");
    assert!(
        markdown.contains("- routes: counted(1); shown=1 hidden=0"),
        "the no-cache path must still return the live route horizon: {markdown}"
    );
    assert!(
        !absent_cache.exists(),
        "CODEMAP_NO_CACHE must suppress the runtime-root artifact directory"
    );
}

#[test]
fn runtime_root_no_cache_bypasses_an_existing_warm_artifact() {
    let (repo, cache) = runtime_root_cache_fixture();
    run_markdown(repo.path(), cache.path(), &["runtime", "."]);
    let path = lens_artifact_path(cache.path(), "runtime-root.json");
    let poisoned = with_current_runtime_report_hash(forge_cached_runtime_route(
        fs::read_to_string(&path).expect("runtime root cache"),
    ));
    fs::write(&path, poisoned).expect("poison runtime root cache");

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .env("CODEMAP_NO_CACHE", "1")
        .args(["runtime", "."])
        .output()
        .expect("no-cache runtime should bypass warm artifact");
    assert!(
        output.status.success(),
        "no-cache runtime failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let markdown = String::from_utf8(output.stdout).expect("runtime markdown");
    assert!(
        markdown.contains("GET /health") && !markdown.contains("/forged"),
        "CODEMAP_NO_CACHE must return live runtime truth instead of the warm artifact: {markdown}"
    );
}

fn assert_runtime_root_corruption_rebuilds(
    corrupt: fn(String) -> String,
    refresh_report_hash: bool,
    message: &str,
) {
    let (repo, cache) = runtime_root_cache_fixture();
    let expected_output = run_markdown(repo.path(), cache.path(), &["runtime", "."]);
    let expected_artifact = runtime_root_cache_json(cache.path());
    let path = lens_artifact_path(cache.path(), "runtime-root.json");
    let original = fs::read_to_string(&path).expect("runtime root cache");
    assert_eq!(
        runtime_report_digest(&original),
        expected_artifact["report_sha256"]
            .as_str()
            .expect("runtime report sha256"),
        "the test digest must match the production runtime report hash",
    );
    let mut corrupted = corrupt(original);
    if refresh_report_hash {
        corrupted = with_current_runtime_report_hash(corrupted);
    }
    assert_ne!(
        serde_json::from_str::<Value>(&corrupted).expect("corrupted runtime cache"),
        expected_artifact,
        "the cache mutation must change persisted truth",
    );
    fs::write(&path, corrupted).expect("write corrupted runtime root cache");

    let rebuilt_output = run_markdown(repo.path(), cache.path(), &["runtime", "."]);
    assert_lens_markdown_eq(&expected_output, &rebuilt_output, message);
    assert_eq!(
        runtime_root_cache_json(cache.path()),
        expected_artifact,
        "{message}; the fallback must repair the poisoned runtime-root artifact"
    );
    let repeated_output = run_markdown(repo.path(), cache.path(), &["runtime", "."]);
    assert_lens_markdown_eq(
        &expected_output,
        &repeated_output,
        "{message}; repeated reads must serve the repaired truth",
    );
}

fn runtime_root_cache_fixture() -> (TempDir, TempDir) {
    let repo = TempDir::new().expect("runtime root cache repo");
    let cache = TempDir::new().expect("runtime root cache dir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"runtime-root-cache-fixture","private":true,"scripts":{"test":"vitest"}}"#,
    );
    write(
        &repo.path().join("src/routes.ts"),
        "router.get('/health', health);\n",
    );
    write(
        &repo.path().join("__main__.py"),
        "def main():\n    return True\n",
    );
    write(
        &repo.path().join("workers/job.ts"),
        "export async function run() { return true; }\n",
    );
    git(repo.path(), &["add", "."]);
    git(
        repo.path(),
        &["commit", "-qm", "runtime root cache fixture"],
    );
    (repo, cache)
}

fn runtime_root_cache_json(cache: &Path) -> Value {
    let path = lens_artifact_path(cache, "runtime-root.json");
    serde_json::from_str(&fs::read_to_string(path).expect("runtime root cache"))
        .expect("runtime root cache json")
}

fn corrupt_runtime_horizon_certificate_reference(text: String) -> String {
    let marker = "\"certificate_id\": \"";
    let start = text.find(marker).expect("runtime certificate reference") + marker.len();
    let end = start
        + text[start..]
            .find('"')
            .expect("runtime certificate reference terminator");
    format!(
        "{}coverage-v1:{}{}",
        &text[..start],
        "0".repeat(64),
        &text[end..]
    )
}

fn remove_cached_runtime_routes(text: String) -> String {
    replace_json_collection(&text, "\n    \"routes\": ", '[', ']', "[]")
}

fn forge_cached_runtime_route(text: String) -> String {
    let forged = text.replacen("\"path\": \"/health\"", "\"path\": \"/forged\"", 1);
    assert_ne!(forged, text, "fixture route must be present in the cache");
    forged
}

fn replace_json_collection(
    text: &str,
    marker: &str,
    open: char,
    close: char,
    replacement: &str,
) -> String {
    let start = text.find(marker).expect("cached JSON collection") + marker.len();
    let end = matching_json_delimiter(text, start, open, close);
    format!("{}{}{}", &text[..start], replacement, &text[end + 1..])
}

fn matching_json_delimiter(text: &str, start: usize, open: char, close: char) -> usize {
    assert_eq!(text[start..].chars().next(), Some(open));
    let mut depth = 0_u32;
    let mut quoted = false;
    let mut escaped = false;
    for (offset, character) in text[start..].char_indices() {
        if quoted {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                quoted = false;
            }
            continue;
        }
        if character == '"' {
            quoted = true;
        } else if character == open {
            depth += 1;
        } else if character == close {
            depth -= 1;
            if depth == 0 {
                return start + offset;
            }
        }
    }
    panic!("unterminated JSON collection after byte {start}");
}

fn with_current_runtime_report_hash(text: String) -> String {
    let digest = runtime_report_digest(&text);
    let parsed: Value = serde_json::from_str(&text).expect("runtime cache json");
    let previous = parsed["report_sha256"]
        .as_str()
        .expect("runtime report sha256");
    text.replacen(previous, &digest, 1)
}

fn runtime_report_digest(text: &str) -> String {
    use sha2::{Digest, Sha256};

    let parsed: Value = serde_json::from_str(text).expect("runtime cache json");
    let canonical = serde_json::to_vec(&parsed["report"]).expect("canonical runtime report");
    format!("{:x}", Sha256::digest(canonical))
}

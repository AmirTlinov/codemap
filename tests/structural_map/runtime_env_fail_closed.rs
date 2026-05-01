#[test]
fn runtime_lens_env_surfaces_ignore_cross_line_literals_and_comments() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/.env.example"),
        "REAL_ENV=\nPY_REAL=\n",
    );
    write(
        &repo.path().join("packages/app/src/env-noise.ts"),
        "/*\nprocess.env.BLOCK_COMMENT_ENV;\nprocess.env[blockCommentKey];\n*/\nconst docs = `\nprocess.env.TEMPLATE_ENV;\nprocess.env[templateKey];\n`;\nconst sameLine = \"process.env.STRING_ENV\";\nconst ru = \"пппппп\";\nexport const real = process.env.REAL_ENV;\nconst realKey = 'REAL_DYNAMIC';\nexport const dynamic = process.env[realKey];\n",
    );
    write(
        &repo.path().join("packages/app/src/env_doc.py"),
        "import os\n\"\"\"\nos.getenv(\"DOC_TOKEN\")\nos.environ['DOC_OTHER']\nos.getenv(dynamic_doc)\n\"\"\"\nREAL = os.getenv(\"PY_REAL\")\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "env noise fixture"]);

    let runtime = run_json(
        repo.path(),
        cache.path(),
        &["runtime", "packages/app/src", "--format", "json"],
    );
    assert_schema("schemas/runtime.schema.json", &runtime);
    let env = runtime["env"].as_array().expect("env surfaces");
    for expected in ["REAL_ENV", "PY_REAL"] {
        assert!(
            env.iter().any(|surface| surface["name"] == expected
                && surface["declaration"] == "packages/app/.env.example"),
            "runtime lens should keep real static env surface `{expected}`: {runtime:#}"
        );
    }
    for rejected in [
        "BLOCK_COMMENT_ENV",
        "TEMPLATE_ENV",
        "STRING_ENV",
        "DOC_TOKEN",
        "DOC_OTHER",
    ] {
        assert!(
            env.iter().all(|surface| surface["name"] != rejected),
            "env-looking text inside comments, templates, strings, or docstrings must not become a runtime env surface ({rejected}): {runtime:#}"
        );
    }
    let dynamic_env_unknowns = runtime["unknowns"]
        .as_array()
        .expect("runtime unknowns")
        .iter()
        .filter(|unknown| unknown["kind"] == "env_dynamic_lookup")
        .collect::<Vec<_>>();
    assert_eq!(
        dynamic_env_unknowns.len(),
        1,
        "only the real dynamic env lookup should remain as a typed unknown: {runtime:#}"
    );
    assert_eq!(
        dynamic_env_unknowns[0]["path"],
        "packages/app/src/env-noise.ts"
    );
}

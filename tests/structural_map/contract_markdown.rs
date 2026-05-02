#[test]
fn contract_markdown_groups_exported_symbols_under_file() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/replay/src/multi-contract.ts"),
        "export interface AlphaDto { id: string }\nexport type BetaDto = { id: string };\nexport function makeGamma(): AlphaDto { return { id: 'g' }; }\n",
    );

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["contract", "packages/replay/src/multi-contract.ts"])
        .output()
        .expect("contract markdown should run");
    assert!(output.status.success());
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("- `packages/replay/src/multi-contract.ts`"),
        "contract markdown should group exported symbols under the owning file: {markdown}"
    );
    assert_eq!(
        markdown
            .matches("- `packages/replay/src/multi-contract.ts`")
            .count(),
        1,
        "contract markdown should print the owning file heading once: {markdown}"
    );
    assert!(
        markdown.contains("  - `AlphaDto`")
            && markdown.contains("  - `BetaDto`")
            && markdown.contains("  - `makeGamma`"),
        "contract markdown should list symbol names without repeating the full anchor: {markdown}"
    );
    assert_eq!(
        markdown
            .matches("packages/replay/src/multi-contract.ts#")
            .count(),
        0,
        "contract markdown should not repeat full file#symbol paths for every export: {markdown}"
    );
}

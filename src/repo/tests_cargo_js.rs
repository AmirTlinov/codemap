    #[test]
    fn cargo_workspace_table_and_dotted_forms_parse() {
        let workspace = r#"[workspace]
members = [
  "crates/app",
  "crates/renderer",
]
exclude = ["crates/ignored"]
dependencies.codemap_fixture_tools = { path = "crates/tools" }
dependencies.codemap_fixture_extra.path = "crates/extra"
dependencies.codemap_fixture_inline = { path = "crates/inline" }
dependencies.codemap_fixture_quoted = { version = "0.1, still a string", path = "crates/quoted,comma" }

[workspace.dependencies.codemap_fixture_replay]
path = "crates/replay"
"#;
        assert_eq!(
            cargo_workspace_array_values(workspace, "members"),
            vec!["crates/app".to_string(), "crates/renderer".to_string()]
        );
        assert_eq!(
            cargo_workspace_array_values(workspace, "exclude"),
            vec!["crates/ignored".to_string()]
        );
        let deps = cargo_workspace_path_dependencies(workspace);
        assert_eq!(
            deps.get("codemap_fixture_replay").map(String::as_str),
            Some("crates/replay")
        );
        assert_eq!(
            deps.get("codemap_fixture_tools").map(String::as_str),
            Some("crates/tools")
        );
        assert_eq!(
            deps.get("codemap_fixture_extra").map(String::as_str),
            Some("crates/extra")
        );
        assert_eq!(
            deps.get("codemap_fixture_inline").map(String::as_str),
            Some("crates/inline")
        );
        assert_eq!(
            deps.get("codemap_fixture_quoted").map(String::as_str),
            Some("crates/quoted,comma")
        );
        let root_dotted = r#"workspace.members = ["crates/app", "crates/replay"]
workspace.dependencies.codemap_fixture_root = { path = "crates/root" }
"#;
        assert_eq!(
            cargo_workspace_array_values(root_dotted, "members"),
            vec!["crates/app".to_string(), "crates/replay".to_string()]
        );
        assert_eq!(
            cargo_workspace_path_dependencies(root_dotted)
                .get("codemap_fixture_root")
                .map(String::as_str),
            Some("crates/root")
        );

        let package = r#"[dependencies]
codemap_fixture_replay.workspace = true
codemap_fixture_tools.workspace = true
codemap_fixture_inline = { version = "0.1, still a string", path = "crates/inline,comma" }

[dependencies.codemap_fixture_table]
workspace = true

[dev-dependencies]
codemap_fixture_test = { path = "crates/test" }

[build-dependencies]
codemap_fixture_build.workspace = true

[target.'cfg(unix)'.dependencies.codemap_fixture_target]
path = "crates/target"

[package.metadata.fake.dependencies.codemap_fixture_ignored]
path = "crates/ignored"
"#;
        assert_eq!(
            cargo_workspace_dependency_names(package),
            vec![
                ("codemap_fixture_replay".to_string(), "runtime".to_string()),
                ("codemap_fixture_table".to_string(), "runtime".to_string()),
                ("codemap_fixture_tools".to_string(), "runtime".to_string()),
                ("codemap_fixture_build".to_string(), "build".to_string())
            ]
        );
        let path_deps = cargo_path_dependencies(package);
        assert!(path_deps.iter().any(|(name, path, kind)| {
            name == "codemap_fixture_inline" && path == "crates/inline,comma" && kind == "runtime"
        }));
        assert!(path_deps.iter().any(|(name, path, kind)| {
            name == "codemap_fixture_test" && path == "crates/test" && kind == "dev"
        }));
        assert!(path_deps.iter().any(|(name, path, kind)| {
            name == "codemap_fixture_target" && path == "crates/target" && kind == "runtime"
        }));
        assert!(
            path_deps
                .iter()
                .all(|(name, _, _)| name != "codemap_fixture_ignored")
        );
        assert!(cargo_workspace_member_pattern_matches(
            "crates/renderer",
            "crates/renderer"
        ));
        assert!(cargo_workspace_member_pattern_matches(
            "crates/group/app",
            "crates/*/app"
        ));
    }

    #[test]
    fn cargo_package_name_uses_structural_toml() {
        assert_eq!(
            cargo_package_name(
                r#"[package]
name = "codemap_fixture_renderer"
version = "0.1.0"
edition = "2024"
"#
            )
            .as_deref(),
            Some("codemap_fixture_renderer")
        );
    }

    #[test]
    fn javascript_local_dependency_specs_parse_only_relative_paths() {
        assert_eq!(
            js_local_dependency_path("file:../renderer").as_deref(),
            Some("../renderer")
        );
        assert_eq!(
            js_local_dependency_path("link:./packages/replay").as_deref(),
            Some("./packages/replay")
        );
        assert_eq!(js_local_dependency_path("portal:..").as_deref(), Some(".."));
        assert_eq!(
            js_local_dependency_path("workspace:../renderer").as_deref(),
            Some("../renderer")
        );
        assert_eq!(js_local_dependency_path("workspace:*"), None);
        assert_eq!(js_local_dependency_path("^1.2.3"), None);
        assert_eq!(js_local_dependency_path("file:/tmp/renderer"), None);
        assert!(js_dependency_spec_is_local_protocol(
            "file:../../../external"
        ));
        assert!(js_dependency_spec_is_local_protocol(
            "workspace:/tmp/renderer"
        ));
        assert!(js_dependency_spec_is_local_protocol(
            "workspace:../../../external"
        ));
        assert!(!js_dependency_spec_is_local_protocol("workspace:"));
        assert!(!js_dependency_spec_is_local_protocol("workspace:*"));
        assert!(!js_dependency_spec_is_local_protocol("workspace:^1.2.3"));
    }

    #[test]
    fn javascript_import_specs_ignore_import_text_inside_strings() {
        let text = r#"const docs = "import { ShellHint } from './shell-hint';";
const tmpl = `require('./shadow')`;
const importPattern = /import { RegexHint } from '.\/regex-hint'/;
const exportPattern = /export { RegexOther } from '.\/regex-other'/;
import { Real as LocalReal } from './real';
export { Other } from './other';
export { /* Real is only a comment */ Other as PublicOther } from './commented';
export { CommentedGap as PublicGap } /* valid comment gap */ from './comment-gap';
export {
  Dialog,
  type ToastData,
} from './primitives'
const lazy = import('./lazy');
const required = require('./required');
"#;

        let specs = extract_js_import_specs(text);

        assert!(specs.contains("./real"));
        assert!(specs.contains("./other"));
        assert!(specs.contains("./commented"));
        assert!(specs.contains("./comment-gap"));
        assert!(specs.contains("./primitives"));
        assert!(specs.contains("./lazy"));
        assert!(specs.contains("./required"));
        assert!(!specs.contains("./shell-hint"));
        assert!(!specs.contains("./shadow"));
        assert!(!specs.contains("./regex-hint"));
        assert!(!specs.contains("./regex-other"));

        let bindings = extract_js_import_bindings(text);
        assert_eq!(
            bindings
                .get("./real")
                .and_then(|map| map.get("LocalReal"))
                .map(String::as_str),
            Some("Real")
        );
        assert_eq!(
            bindings
                .get("./commented")
                .and_then(|map| map.get("export:PublicOther"))
                .map(String::as_str),
            Some("Other")
        );
        assert_eq!(
            bindings
                .get("./comment-gap")
                .and_then(|map| map.get("export:PublicGap"))
                .map(String::as_str),
            Some("CommentedGap")
        );
        assert_eq!(
            bindings
                .get("./primitives")
                .and_then(|map| map.get("export:Dialog"))
                .map(String::as_str),
            Some("Dialog")
        );
        assert!(!bindings.contains_key("./shell-hint"));
        assert!(!bindings.contains_key("./regex-hint"));
        assert!(!bindings.contains_key("./regex-other"));
    }

    #[test]
    fn javascript_local_bindings_capture_function_and_destructured_params() {
        let text = r#"import { ShellHint } from './shell-hint';

export function ShellParamShadowView({ ShellHint }: Props) {
  return <ShellHint />;
}

const Arrow = ({ CanvasShellHint }: Props) => <CanvasShellHint />;
const Single = ShellAction => <ShellAction />;
export default function({ DefaultShellHint }: Props) {
  return <DefaultShellHint />;
}
export function FunctionTypedParam(
  makeHint: (id: string) => string,
  LaterShellHint: (id: string) => string
) {
  return LaterShellHint('x');
}
const ArrowTypedParam = (
  makeHint: (id: string) => string,
  LaterArrowHint: (id: string) => string
) => LaterArrowHint('x');
const methods = {
  render({ MethodShellHint }: Props) {
    return <MethodShellHint />;
  },
};
function Destructure(props) {
  const { LocalHint } = props;
  let {
    MultilineHint,
  } = props;
  const { hint: AliasHint = FallbackHint } = props;
  const [ArrayHint = FallbackArrayHint] = props.items;
  return <LocalHint />;
}
function LoopAndCatch() {
  for (const LoopHint of hints) {
    return <LoopHint />;
  }
  for await (const AwaitLoopHint of hints) {
    return <AwaitLoopHint />;
  }
  try {
    run();
  } catch (CatchHint) {
    return <CatchHint />;
  }
}
"#;

        let bindings = extract_local_bindings(text, "tsx");

        assert!(bindings.contains("ShellHint"));
        assert!(bindings.contains("CanvasShellHint"));
        assert!(bindings.contains("ShellAction"));
        assert!(bindings.contains("DefaultShellHint"));
        assert!(bindings.contains("LaterShellHint"));
        assert!(bindings.contains("LaterArrowHint"));
        assert!(bindings.contains("MethodShellHint"));
        assert!(bindings.contains("LocalHint"));
        assert!(bindings.contains("MultilineHint"));
        assert!(bindings.contains("AliasHint"));
        assert!(bindings.contains("ArrayHint"));
        assert!(bindings.contains("LoopHint"));
        assert!(bindings.contains("AwaitLoopHint"));
        assert!(bindings.contains("CatchHint"));
    }

    #[test]
    fn javascript_jsx_tags_ignore_type_generic_arguments() {
        let generic_only = r#"import { GroupCard } from './card';

const value = identity<GroupCard | null>(null);
type Factory = <GroupCard>() => void;
const make = <GroupCard extends object>() => null;
"#;
        assert!(!extract_jsx_tags(generic_only, "tsx").contains("GroupCard"));

        let text = r#"import { GroupCard } from './card';

export function View() {
  return <GroupCard title="real" />;
}
"#;

        let tags = extract_jsx_tags(text, "tsx");

        assert!(tags.contains("GroupCard"));
        assert_eq!(tags.len(), 1);
    }

    #[test]
    fn javascript_symbol_ranges_skip_multiline_destructured_params() {
        let source = r#"export function PanelView({
  Header,
  Body,
}: Props) {
  return (
    <section>
      <Header />
      <Body />
    </section>
  );
}
"#;

        let symbols = extract_symbols(source, "tsx");

        assert_symbol(&symbols, "PanelView", "component", true, 1, 11);
    }

    #[test]
    fn javascript_regex_keyword_probe_handles_unicode_prefix() {
        assert!(!previous_word_is("навигации", "return"));
        assert!(previous_word_is("навигации return", "return"));
    }

    #[test]
    fn cargo_workspace_edges_use_workspace_dependency_tables() {
        let repo = tempfile::TempDir::new().expect("temp repo");
        write_test_file(
            &repo.path().join("Cargo.toml"),
            r#"[workspace]
members = [
  "crates/renderer",
  "crates/replay",
]

[workspace.dependencies.codemap_fixture_replay]
path = "crates/replay"
"#,
        );
        write_test_file(
            &repo.path().join("crates/renderer/Cargo.toml"),
            r#"[package]
name = "codemap_fixture_renderer"
version = "0.1.0"
edition = "2024"

[dependencies.codemap_fixture_replay]
workspace = true
"#,
        );
        write_test_file(
            &repo.path().join("crates/replay/Cargo.toml"),
            r#"[package]
name = "codemap_fixture_replay"
version = "0.1.0"
edition = "2024"
"#,
        );
        let project = load_project_with_cache(
            RootSelection::Exact(repo.path().to_path_buf()),
            CacheWriteMode::ReadOnly,
        )
        .expect("load project");
        let by_path: BTreeMap<String, &PackageInfo> = project
            .packages
            .iter()
            .map(|package| (package.path.clone(), package))
            .collect();
        let workspaces =
            cargo_workspace_infos(repo.path(), &project.files, &project.packages, &by_path);
        assert!(
            project.package_edges.iter().any(|edge| {
                edge.from == "crates/renderer"
                    && edge.to == "crates/replay"
                    && edge.source == "Cargo.toml workspace dependency"
            }),
            "files: {:#?}; packages: {:#?}; workspaces: {:#?}; package edges: {:#?}",
            project.files.keys().collect::<Vec<_>>(),
            project.packages,
            workspaces,
            project.package_edges
        );
    }

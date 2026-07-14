// Responsibility: repo-tests-js-modules
use crate::repo::tests::playwright::assert_symbol;
use crate::repo::{
    extract_js_import_bindings, extract_js_import_specs, extract_jsx_tags, extract_local_bindings,
    extract_symbols, js_dependency_spec_is_local_protocol, js_local_dependency_path,
    previous_word_is,
};

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
pub(crate) const lazy = import('./lazy');
pub(crate) const required = require('./required');
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
pub(crate) const methods = {
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

pub(crate) const value = identity<GroupCard | null>(null);
type Factory = <GroupCard>() => void;
pub(crate) const make = <GroupCard extends object>() => null;
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

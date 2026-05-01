fn symbol_import_fixture() -> (TempDir, TempDir) {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "symbol-anchor-fixture",
  "private": true,
  "scripts": { "test": "vitest run" }
}
"#,
    );
    write(
        &repo.path().join("src/card.tsx"),
        "export function GroupCard() {\n  return <section>Group</section>;\n}\n\nexport function AdminCard() {\n  return <section>Admin</section>;\n}\n",
    );
    write(
        &repo.path().join("src/home.tsx"),
        "import { GroupCard as Card } from './card';\n\nexport function HomePage() {\n  return <Card />;\n}\n",
    );
    write(
        &repo.path().join("src/two-cards.tsx"),
        "import { AdminCard, GroupCard } from './card';\n\nexport function TwoCards() {\n  return <><GroupCard /><AdminCard /></>;\n}\n",
    );
    write(
        &repo.path().join("src/panel-parts.tsx"),
        "export function PanelHeader() {\n  return <header>Panel</header>;\n}\n\nexport function PanelBody() {\n  return <main>Body</main>;\n}\n",
    );
    write(
        &repo.path().join("src/panel-view.tsx"),
        "import { PanelBody, PanelHeader } from './panel-parts';\n\ntype Props = {\n  title: string;\n};\n\nexport function PanelView({\n  title,\n}: Props) {\n  return (\n    <section aria-label={title}>\n      <PanelHeader />\n      <PanelBody />\n    </section>\n  );\n}\n",
    );
    write(
        &repo.path().join("src/helpers.tsx"),
        "export function custom() {\n  return null;\n}\n",
    );
    write(
        &repo.path().join("src/lowercase-jsx.tsx"),
        "import { custom } from './helpers';\n\nexport function LowercaseView() {\n  return <custom />;\n}\n",
    );
    write(
        &repo.path().join("src/admin.tsx"),
        "import { AdminCard } from './card';\n\nexport function AdminPage() {\n  return <AdminCard />;\n}\n",
    );
    write(
        &repo.path().join("src/unused.tsx"),
        "import { GroupCard } from './card';\n\nexport const unused = true;\n",
    );
    write(
        &repo.path().join("src/side-effect.tsx"),
        "import { GroupCard } from './card'\nimport './setup'\n\nexport function SideEffectPage() {\n  return <GroupCard />;\n}\n",
    );
    write(
        &repo.path().join("src/setup.ts"),
        "export const setup = true;\n",
    );
    write(
        &repo.path().join("src/string-only.tsx"),
        "import { GroupCard } from './card';\n\nexport const fixture = 'GroupCard';\n",
    );
    write(
        &repo.path().join("src/card.test.tsx"),
        "import { GroupCard } from './card';\n\ntest('group card export stays usable', () => {\n  expect(GroupCard).toBeDefined();\n});\n",
    );
    write(
        &repo.path().join("src/type-only-consumer.test.tsx"),
        "import { GroupCard } from './card';\n\ntype Props = {\n  id: string;\n  component: typeof GroupCard;\n};\n\ntest('type-only mention does not prove runtime behavior', () => {\n  const props: Props | null = null;\n  expect(props).toBeNull();\n});\n",
    );
    write(
        &repo.path().join("src/type-annotation-consumer.test.tsx"),
        "import { GroupCard } from './card';\n\ntest('typeof annotation does not prove runtime behavior', () => {\n  let component: typeof GroupCard | null = null;\n  expect(component).toBeNull();\n});\n",
    );
    write(
        &repo.path().join("src/type-assertion-consumer.test.tsx"),
        "import { GroupCard } from './card';\n\ntest('typeof assertion does not prove runtime behavior', () => {\n  const value = null as unknown as typeof GroupCard;\n  expect(value).toBeNull();\n});\n",
    );
    write(
        &repo.path().join("src/implements-only.test.tsx"),
        "import { GroupCard } from './card';\n\nclass Fake implements GroupCard {\n  value = 1;\n}\n\ntest('implements mention does not prove runtime behavior', () => {\n  expect(new Fake().value).toBe(1);\n});\n",
    );
    write(
        &repo.path().join("src/object-key.test.tsx"),
        "import { GroupCard } from './card';\n\ntest('object key does not prove runtime behavior', () => {\n  const metadata = { GroupCard: true };\n  expect(metadata.GroupCard).toBe(true);\n});\n",
    );
    write(
        &repo.path().join("src/regex-only.test.tsx"),
        "import { GroupCard } from './card';\n\ntest('regex mention does not prove runtime behavior', () => {\n  expect(/GroupCard/.test('GroupCard')).toBe(true);\n});\n",
    );
    write(
        &repo.path().join("src/regex-angle.test.tsx"),
        "import { GroupCard } from './card';\n\ntest('regex markup mention does not prove runtime behavior', () => {\n  expect(/<GroupCard>/.test('<GroupCard>')).toBe(true);\n});\n",
    );
    write(
        &repo.path().join("src/regex-group.test.tsx"),
        "import { GroupCard } from './card';\n\ntest('regex group mention does not prove runtime behavior', () => {\n  expect(/foo (GroupCard) bar/.test('GroupCard')).toBe(true);\n});\n",
    );
    write(
        &repo.path().join("src/arrow-regex-group.test.tsx"),
        "import { GroupCard } from './card';\n\ntest('arrow regex group mention does not prove runtime behavior', () => {\n  const matcher = () => /foo (GroupCard) bar/.test('GroupCard');\n  expect(matcher()).toBe(true);\n});\n",
    );
    write(
        &repo.path().join("src/await-regex-consumer.ts"),
        "import { GroupCard } from './card';\n\nexport async function regexConsumer(value: string) {\n  return await /foo (GroupCard) bar/.test(value);\n}\n",
    );
    write(
        &repo.path().join("src/if-regex-consumer.tsx"),
        "import { GroupCard } from './card';\n\nexport function regexConsumer(enabled: boolean, value: string) {\n  if (enabled) /foo (GroupCard) bar/.test(value);\n  if (enabled) /<GroupCard>/.test(value);\n}\n",
    );
    write(
        &repo.path().join("src/else-regex-consumer.tsx"),
        "import { GroupCard } from './card';\n\nexport function regexConsumer(enabled: boolean, value: string) {\n  if (enabled) return;\n  else /foo (GroupCard) bar/.test(value);\n}\n",
    );
    write(
        &repo.path().join("src/type-generic-consumer.tsx"),
        "import { GroupCard } from './card';\n\nfunction identity<T>(value: T) {\n  return value;\n}\n\nexport const value = identity<GroupCard | null>(null);\n",
    );
    write(
        &repo.path().join("src/template-consumer.tsx"),
        "import { GroupCard } from './card';\n\nexport const snippet = `\n  GroupCard()\n`;\n",
    );
    write(
        &repo.path().join("src/generic-arrow.tsx"),
        "import { GroupCard } from './card';\n\nexport const make = <GroupCard extends object>() => null;\n",
    );
    write(
        &repo.path().join("src/angle-assertion.ts"),
        "import { GroupCard } from './card';\n\nexport function cast(value: unknown) {\n  return <GroupCard>value;\n}\n",
    );
    write(
        &repo.path().join("src/await-regex.test.tsx"),
        "import { GroupCard } from './card';\n\ntest('await regex mention does not prove runtime behavior', async () => {\n  const matched = await /foo (GroupCard) bar/.test('GroupCard');\n  expect(matched).toBe(true);\n});\n",
    );
    write(
        &repo.path().join("src/if-regex.test.tsx"),
        "import { GroupCard } from './card';\n\ntest('if regex mention does not prove runtime behavior', () => {\n  if (true) /foo (GroupCard) bar/.test('GroupCard');\n  if (true) /<GroupCard>/.test('GroupCard');\n});\n",
    );
    write(
        &repo.path().join("src/else-regex.test.tsx"),
        "import { GroupCard } from './card';\n\ntest('else regex mention does not prove runtime behavior', () => {\n  if (false) return;\n  else /foo (GroupCard) bar/.test('GroupCard');\n});\n",
    );
    write(
        &repo.path().join("src/throw-regex.test.tsx"),
        "import { GroupCard } from './card';\n\ntest('throw regex mention does not prove runtime behavior', () => {\n  try {\n    throw /foo (GroupCard) bar/;\n  } catch (pattern) {\n    expect(pattern.test('GroupCard')).toBe(true);\n  }\n});\n",
    );
    write(
        &repo.path().join("src/type-generic.test.tsx"),
        "import { GroupCard } from './card';\n\nfunction identity<T>(value: T) {\n  return value;\n}\n\ntest('generic type argument does not prove runtime behavior', () => {\n  expect(identity<GroupCard | null>(null)).toBeNull();\n});\n",
    );
    write(
        &repo.path().join("src/type-factory.test.tsx"),
        "import { GroupCard } from './card';\n\ntype Factory = <GroupCard>() => void;\n\ntest('generic type parameter does not prove runtime behavior', () => {\n  const noop: Factory | null = null;\n  expect(noop).toBeNull();\n});\n",
    );
    write(
        &repo.path().join("src/generic-arrow.test.tsx"),
        "import { GroupCard } from './card';\n\ntest('generic arrow type parameter does not prove runtime behavior', () => {\n  const make = <GroupCard extends object>() => null;\n  expect(make()).toBeNull();\n});\n",
    );
    write(
        &repo.path().join("src/template-only.test.tsx"),
        "import { GroupCard } from './card';\n\ntest('template snippet does not prove runtime behavior', () => {\n  const snippet = `\n    GroupCard()\n  `;\n  expect(snippet).toContain('GroupCard');\n});\n",
    );
    write(
        &repo.path().join("src/admin.test.tsx"),
        "import { AdminCard } from './card';\n\ntest('admin card export stays usable', () => {\n  expect(AdminCard).toBeDefined();\n});\n",
    );
    write(
        &repo.path().join("src/local-shadow.tsx"),
        "import { GroupCard } from './card';\n\nexport function ShadowPage() {\n  const GroupCard = () => <section>Local</section>;\n  return <GroupCard />;\n}\n",
    );
    write(
        &repo.path().join("src/for-shadow.tsx"),
        "import { GroupCard } from './card';\n\nconst cards = [() => <section>Local</section>];\n\nexport function ForShadowPage() {\n  for (const GroupCard of cards) {\n    return <GroupCard />;\n  }\n  return null;\n}\n",
    );
    write(
        &repo.path().join("src/for-await-shadow.tsx"),
        "import { GroupCard } from './card';\n\nasync function* cards() {\n  yield () => <section>Local</section>;\n}\n\nexport async function ForAwaitShadowPage() {\n  for await (const GroupCard of cards()) {\n    return <GroupCard />;\n  }\n  return null;\n}\n",
    );
    write(
        &repo.path().join("src/catch-shadow.tsx"),
        "import { GroupCard } from './card';\n\nexport function CatchShadowPage() {\n  try {\n    throw new Error('x');\n  } catch (GroupCard) {\n    return <GroupCard />;\n  }\n}\n",
    );
    write(
        &repo.path().join("src/for-await-shadow.test.tsx"),
        "import { GroupCard } from './card';\n\nasync function* cards() {\n  yield () => <section>Local</section>;\n}\n\ntest('for await shadow does not prove runtime behavior', async () => {\n  for await (const GroupCard of cards()) {\n    expect(GroupCard).toBeDefined();\n  }\n});\n",
    );
    write(
        &repo.path().join("src/default-card.tsx"),
        "export default function DefaultCard() {\n  return <section>Default</section>;\n}\n",
    );
    write(
        &repo.path().join("src/default-consumer.tsx"),
        "import RenamedCard from './default-card';\n\nexport function DefaultPage() {\n  return <RenamedCard />;\n}\n",
    );
    write(
        &repo.path().join("src/default-card.test.tsx"),
        "import RenamedCard from './default-card';\n\ntest('default card export stays usable', () => {\n  expect(RenamedCard).toBeDefined();\n});\n",
    );
    write(
        &repo.path().join("src/default-const-card.tsx"),
        "const DefaultConstCard = () => <section>Default const</section>;\n\nexport default DefaultConstCard;\n",
    );
    write(
        &repo.path().join("src/default-const-consumer.tsx"),
        "import RenamedConstCard from './default-const-card';\n\nexport function DefaultConstPage() {\n  return <RenamedConstCard />;\n}\n",
    );
    write(
        &repo.path().join("src/default-const-card.test.tsx"),
        "import RenamedConstCard from './default-const-card';\n\ntest('default const card export stays usable', () => {\n  expect(RenamedConstCard).toBeDefined();\n});\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    (repo, cache)
}

// 从 TypeScript 侧启动 Rust 根组件，验证工作台外壳的核心结构都在。
import { expect, open, test } from "nsh/test";

test("renders the Keel workbench shell", async () => {
  const app = await open(".");
  const texts = app.texts();
  expect(texts).toContain("Keel");
  expect(texts).toContain("文件");
  expect(texts).toContain("视图");
  expect(texts).toContain("导航");
  expect(texts).toContain("帮助");
  expect(texts).toContain("项目");
  expect(texts).toContain("⌕");
  expect(texts).toContain("⚙");
  expect(texts).toContain("UTF-8");
});

test("opens the View menu from the action registry", async () => {
  const app = await open(".");
  await app.press("text:视图");
  await app.waitFor("text:工具窗口");
  const texts = app.texts();
  expect(texts).toContain("工具窗口");
  expect(texts).toContain("底部工具窗口");
  expect(texts).toContain("外观");
});

test("opens the command palette and filters actions", async () => {
  const app = await open(".");
  await app.keyboard.press("mod+shift+a");
  await app.waitFor("text:关闭标签页");
  const texts = app.texts();
  expect(texts).toContain("打开项目…");
  await app.type("text:搜索操作…", "保存");
  expect(app.texts()).toContain("保存");
  await app.keyboard.press("escape");
  await app.waitFor("text:保存", { state: "gone" });
});

test("opens the project tree context menu", async () => {
  const app = await open(".");
  // 没有打开示例项目时跳过。
  if (app.count("text:pom.xml") === 0) {
    return;
  }
  await app.rightClick("text:pom.xml");
  await app.waitFor("text:复制路径");
  const texts = app.texts();
  expect(texts).toContain("打开");
  expect(texts).toContain("在项目中定位");
  await app.press("text:复制路径");
  await app.waitFor("text:已复制路径");
});

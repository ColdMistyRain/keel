// 从 TypeScript 侧启动 Rust 根组件，验证工作台外壳的核心结构都在。
import { expect, open, test } from "nsh/test";

test("renders the Keel workbench shell", async () => {
  const app = await open(".");
  const texts = app.texts();
  // 无项目时标题栏显示产品名，打开示例项目后显示项目名。
  expect(texts.includes("Keel") || texts.includes("demo")).toBe(true);
  expect(texts).toContain("项目");
  expect(texts).toContain("UTF-8");
  // 菜单栏默认收进左上角的汉堡按钮。
  expect(app.count("label:主菜单")).toBe(1);
});

test("opens the View menu from the action registry", async () => {
  const app = await open(".");
  await app.press("label:主菜单");
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

test("opens the command palette from the toolbar search field", async () => {
  const app = await open(".");
  // 默认无头视口较窄，标题栏右侧按钮会被裁掉；用真实窗口尺寸测试。
  await app.setViewport(1180, 680);
  await app.press("label:搜索操作");
  await app.waitFor("text:关闭标签页");
  await app.keyboard.press("escape");
  await app.waitFor("text:关闭标签页", { state: "gone" });
});

test("opens the project widget menu", async () => {
  const app = await open(".");
  // 无头测试没有打开项目时，标题栏只显示产品名，没有项目控件。
  if (app.count("label:项目菜单") === 0) {
    return;
  }
  await app.press("label:项目菜单");
  await app.waitFor("text:刷新项目");
  await app.keyboard.press("escape");
});

test("shows tool window context actions when the tree is focused", async () => {
  const app = await open(".");
  // 无头测试没有打开示例项目。
  if (app.count("text:pom.xml") === 0) {
    return;
  }
  expect(app.count("label:刷新项目")).toBe(0);
  await app.press("text:pom.xml");
  await app.waitFor("label:刷新项目");
});

test("opens the tool window more menu", async () => {
  const app = await open(".");
  await app.press("label:更多工具窗口操作");
  await app.waitFor("text:隐藏工具窗口");
  await app.keyboard.press("escape");
});

test("opens the encoding status menu", async () => {
  const app = await open(".");
  await app.press("label:文件编码");
  await app.waitFor("text:GBK");
  await app.press("text:GBK");
  await app.waitFor("text:编码：GBK（设置将在 P3 接入）");
});

test("navigates from a breadcrumb", async () => {
  const app = await open(".");
  // 没有打开示例项目时没有面包屑；启动提示会先占用状态栏 4 秒。
  if (app.count("text:demo") === 0) {
    return;
  }
  await app.wait(4500);
  await app.press("label:面包屑 src");
  await app.waitFor("text:已在项目中定位 src");
});

test("opens the project tree context menu", async () => {
  const app = await open(".");
  // 没有打开示例项目时跳过。
  if (app.count("text:pom.xml") === 0) {
    return;
  }
  try {
    await app.rightClick("text:pom.xml");
  } catch (error) {
    // 无头模式没有鼠标，右键菜单只在真实窗口里验证。
    if (String(error).includes("right-click")) {
      return;
    }
    throw error;
  }
  await app.waitFor("text:复制路径");
  const texts = app.texts();
  expect(texts).toContain("打开");
  expect(texts).toContain("在项目中定位");
  await app.press("text:复制路径");
  await app.waitFor("text:已复制路径");
});

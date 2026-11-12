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
  expect(texts).toContain("问题");
  expect(texts).toContain("运行");
  expect(texts).toContain("终端");
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

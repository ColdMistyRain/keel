// 从 TypeScript 侧启动 Rust 根组件，验证工作台外壳的核心文案都在。
import { expect, open, test } from "nsh/test";

test("renders the Keel workbench shell", async () => {
  const app = await open(".");
  const texts = app.texts();
  expect(texts).toContain("Keel");
  expect(texts).toContain("文件");
  expect(texts).toContain("视图");
  expect(texts).toContain("帮助");
  expect(texts).toContain("项目");
});

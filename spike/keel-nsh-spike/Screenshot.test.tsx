// 用真实 Windows 目标截取工作台外壳（浅色 / 深色 / 文件菜单）：
//   $env:KEEL_SPIKE_DEMO = "1"; nsh test --target windows
// 无头测试画不出图，跳过截图。
import { open, test } from "nsh/test";

test("captures the Keel workbench shell", async () => {
  const app = await open(".");
  await app.setViewport(1180, 680);
  try {
    await app.screenshot("../../docs/screenshots/nsh-p2-light.png");
  } catch (error) {
    if (String(error).includes("real platform")) {
      return;
    }
    throw error;
  }
  await app.setColorScheme("dark");
  await app.wait(500);
  await app.screenshot("../../docs/screenshots/nsh-p2-dark.png");
});

test("captures the File menu", async () => {
  const app = await open(".");
  await app.setViewport(1180, 680);
  try {
    await app.screenshot("../../docs/screenshots/nsh-p2-menu.png");
  } catch (error) {
    if (String(error).includes("real platform")) {
      return;
    }
    throw error;
  }
  await app.press("text:文件");
  await app.waitFor("text:关闭所有标签页");
  await app.screenshot("../../docs/screenshots/nsh-p2-menu.png");
});

test("captures a preview tab", async () => {
  const app = await open(".");
  await app.setViewport(1180, 680);
  // 没有打开示例项目时跳过（例如无头测试）。
  if (app.count("text:Greeter.java") === 0) {
    return;
  }
  try {
    await app.screenshot("../../docs/screenshots/nsh-p2-preview.png");
  } catch (error) {
    if (String(error).includes("real platform")) {
      return;
    }
    throw error;
  }
  await app.press("text:Greeter.java");
  await app.waitFor("text:预览 Greeter.java");
  await app.screenshot("../../docs/screenshots/nsh-p2-preview.png");
});

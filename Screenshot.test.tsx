// 用真实 Windows 目标截取工作台外壳（浅色 / 深色 / 文件菜单）：
//   $env:KEEL_DEMO = "1"; nsh test --target windows
// 无头测试画不出图，跳过截图。
import { open, test } from "nsh/test";

test("captures the Keel workbench shell", async () => {
  const app = await open(".");
  await app.setViewport(1180, 680);
  // 等启动提示自动清除，截到状态栏左侧的面包屑。
  if (app.count("text:demo") > 0) {
    await app.wait(4500);
  }
  try {
    await app.screenshot("docs/screenshots/nsh-p2-light.png");
  } catch (error) {
    if (String(error).includes("real platform")) {
      return;
    }
    throw error;
  }
  await app.setColorScheme("dark");
  await app.wait(500);
  await app.screenshot("docs/screenshots/nsh-p2-dark.png");
});

test("captures the File menu", async () => {
  const app = await open(".");
  await app.setViewport(1180, 680);
  try {
    await app.screenshot("docs/screenshots/nsh-p2-menu.png");
  } catch (error) {
    if (String(error).includes("real platform")) {
      return;
    }
    throw error;
  }
  await app.press("label:主菜单");
  await app.press("text:文件");
  await app.waitFor("text:关闭所有标签页");
  await app.screenshot("docs/screenshots/nsh-p2-menu.png");
});

test("captures a preview tab", async () => {
  const app = await open(".");
  await app.setViewport(1180, 680);
  // 没有打开示例项目时跳过（例如无头测试）。
  if (app.count("text:Greeter.java") === 0) {
    return;
  }
  try {
    await app.screenshot("docs/screenshots/nsh-p2-preview.png");
  } catch (error) {
    if (String(error).includes("real platform")) {
      return;
    }
    throw error;
  }
  await app.press("text:Greeter.java");
  await app.waitFor("text:预览 Greeter.java");
  // 等图标在下一帧重绘完成，避免截到只画了文字的中间帧。
  await app.wait(300);
  await app.screenshot("docs/screenshots/nsh-p2-preview.png");
});

test("captures the command palette", async () => {
  const app = await open(".");
  await app.setViewport(1180, 680);
  try {
    await app.screenshot("docs/screenshots/nsh-p2-palette.png");
  } catch (error) {
    if (String(error).includes("real platform")) {
      return;
    }
    throw error;
  }
  await app.keyboard.press("mod+shift+a");
  await app.waitFor("text:关闭标签页");
  await app.screenshot("docs/screenshots/nsh-p2-palette.png");
});

test("captures the running run widget", async () => {
  const app = await open(".");
  await app.setViewport(1180, 680);
  if (app.count("text:Main.java") === 0) {
    return;
  }
  try {
    await app.screenshot("docs/screenshots/nsh-p2-running.png");
  } catch (error) {
    if (String(error).includes("real platform")) {
      return;
    }
    throw error;
  }
  await app.keyboard.press("shift+f10");
  await app.waitFor("text:正在运行 Main…");
  await app.screenshot("docs/screenshots/nsh-p2-running.png");
});

test("captures the project tree context menu", async () => {
  const app = await open(".");
  await app.setViewport(1180, 680);
  if (app.count("text:pom.xml") === 0) {
    return;
  }
  try {
    await app.screenshot("docs/screenshots/nsh-p2-tree-menu.png");
  } catch (error) {
    if (String(error).includes("real platform")) {
      return;
    }
    throw error;
  }
  await app.rightClick("text:pom.xml");
  await app.waitFor("text:复制路径");
  await app.screenshot("docs/screenshots/nsh-p2-tree-menu.png");
});

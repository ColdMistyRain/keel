//! 随应用发布的 IDEA 字体：界面 Inter、编辑器 JetBrains Mono。
//!
//! 字体文件来自 IntelliJ IDEA 自带的 JBR 字体目录；nsh 在首帧前注册
//! `assets/fonts/` 下的字体，不安装到系统。Inter 与 JetBrains Mono
//! 均为 SIL Open Font License。

use nsh_core::element::FontAsset;
use nsh_core::prelude::*;

// 常规字重用来取字体族名；其余字重/斜体文件由 `assets!()` 一并发布，
// nsh 会按 `font_weight` / `italic()` 自动选择对应字形。
pub const INTER: FontAsset = asset!("fonts/Inter-Regular.otf");
pub const MONO: FontAsset = asset!("fonts/JetBrainsMono-Regular.ttf");

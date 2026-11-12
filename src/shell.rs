//! 工作台外壳的会话状态（IDEA 的 `IdeFrame` / `IdeRootPane` 的 Keel 版）。
//!
//! 工具窗口、底部窗口、外观和动作注册表都在这里；面板组件只读这些信号，
//! 动作通过 `keel-commands` 的注册表执行，菜单与快捷键共用一份定义。

use std::path::Path;
use std::sync::Arc;

use keel_commands::{ActionId, ActionRegistry, DataContext, Presentation};
use nsh_core::file_dialog::OpenDialog;
use nsh_core::prelude::*;

use crate::commands;
use crate::workspace::Workspace;

/// 左侧工具窗口（IDEA ToolWindow 的锚点位置）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tool {
    Project,
    Search,
    Version,
}

impl Tool {
    pub const ALL: [Tool; 3] = [Tool::Project, Tool::Search, Tool::Version];

    pub fn id(self) -> &'static str {
        match self {
            Tool::Project => "project",
            Tool::Search => "search",
            Tool::Version => "version",
        }
    }

    pub fn action_id(self) -> ActionId {
        match self {
            Tool::Project => "view.tool_window.project",
            Tool::Search => "view.tool_window.search",
            Tool::Version => "view.tool_window.version",
        }
    }

    pub fn glyph(self) -> &'static str {
        match self {
            Tool::Project => "▤",
            Tool::Search => "⌕",
            Tool::Version => "⑂",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Tool::Project => "项目",
            Tool::Search => "搜索",
            Tool::Version => "版本控制",
        }
    }
}

/// 底部工具窗口。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Bottom {
    Problems,
    Run,
    Terminal,
}

impl Bottom {
    pub const ALL: [Bottom; 3] = [Bottom::Problems, Bottom::Run, Bottom::Terminal];

    pub fn id(self) -> &'static str {
        match self {
            Bottom::Problems => "problems",
            Bottom::Run => "run",
            Bottom::Terminal => "terminal",
        }
    }

    pub fn action_id(self) -> ActionId {
        match self {
            Bottom::Problems => "view.bottom.problems",
            Bottom::Run => "view.bottom.run",
            Bottom::Terminal => "view.bottom.terminal",
        }
    }

    pub fn glyph(self) -> &'static str {
        match self {
            Bottom::Problems => "!",
            Bottom::Run => "▷",
            Bottom::Terminal => "❯",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Bottom::Problems => "问题",
            Bottom::Run => "运行",
            Bottom::Terminal => "终端",
        }
    }
}

/// 一次动作执行的上下文（IDEA `DataContext` 的 Keel 版）。
///
/// 动作的可用性 / 选中态只读这里，不直接依赖某个组件。
#[derive(Clone, Copy, Debug, Default)]
pub struct UiState {
    pub has_editor: bool,
    pub can_save: bool,
    pub open_docs: usize,
    pub project_open: bool,
    pub tool_window_visible: bool,
    pub active_tool: &'static str,
    pub bottom_visible: bool,
    pub active_bottom: &'static str,
    pub appearance: &'static str,
    pub running: bool,
}

/// 外壳状态。字段都是可复制的信号句柄，可以传进任意闭包。
#[derive(Clone, Copy)]
pub struct Shell {
    pub ws: Workspace,
    /// 左侧工具窗口是否显示。
    pub tools: Signal<bool>,
    /// 当前工具窗口。
    pub tool: Signal<Tool>,
    /// 底部工具窗口；`None` 时收起。
    pub bottom: Signal<Option<Bottom>>,
    pub appearance: Signal<Appearance>,
    /// 动作注册表：菜单、快捷键、命令面板共用。
    pub registry: Stored<Arc<ActionRegistry>>,
    /// 命令面板（IDEA 的 Search Everywhere / Find Action）。
    pub palette: Signal<bool>,
    pub palette_query: Signal<SharedString>,
    pub palette_selected: Signal<usize>,
    /// 运行中的状态：主工具栏的 run widget 由它切换绿色 / 红色。
    pub running: Signal<bool>,
    /// 编辑器标签条是否溢出（由测量tab 内容宽度与视口宽度得出）。
    pub tab_overflow: Signal<bool>,
    /// 标签条的横向滚动偏移，滚动箭头按它翻页。
    pub tab_scroll: Signal<f32>,
    pub tab_content_width: Signal<f32>,
    pub tab_viewport_width: Signal<f32>,
    /// 标签条视口与内容行，用于测量是否溢出。
    pub tab_viewport: ElementRef,
    pub tab_content: ElementRef,
}

impl Shell {
    pub fn new() -> Self {
        let demo = std::env::var_os("KEEL_DEMO").is_some();
        let shell = Self {
            ws: Workspace::new(),
            tools: signal(true),
            tool: signal(Tool::Project),
            // 演示模式直接展开「问题」窗口，方便截图验收。
            bottom: signal(demo.then_some(Bottom::Problems)),
            appearance: signal(Appearance::System),
            registry: stored(Arc::new(ActionRegistry::new())),
            palette: signal(false),
            palette_query: signal(SharedString::from("")),
            palette_selected: signal(0),
            running: signal(false),
            tab_overflow: signal(false),
            tab_scroll: signal(0.0),
            tab_content_width: signal(0.0),
            tab_viewport_width: signal(0.0),
            tab_viewport: element_ref(),
            tab_content: element_ref(),
        };
        shell.ws.open_startup();
        shell.registry.replace(Arc::new(commands::build(shell)));
        shell
    }

    pub fn registry(&self) -> Arc<ActionRegistry> {
        self.registry.get()
    }

    /// 当前上下文：动作的 `update` / `perform` 都拿它。
    pub fn context(&self) -> DataContext {
        DataContext::new().with(self.ui_state())
    }

    pub fn ui_state(&self) -> UiState {
        let bottom = self.bottom.get_untracked();
        UiState {
            has_editor: self.ws.active.get_untracked().is_some(),
            can_save: self.ws.active.get_untracked().is_some(),
            open_docs: self.ws.open_docs(),
            project_open: self.ws.root.get_untracked().is_some(),
            tool_window_visible: self.tools.get_untracked(),
            active_tool: self.tool.get_untracked().id(),
            bottom_visible: bottom.is_some(),
            active_bottom: bottom.map(Bottom::id).unwrap_or(""),
            appearance: appearance_id(self.appearance.get_untracked()),
            running: self.running.get_untracked(),
        }
    }

    pub fn presentation(&self, id: ActionId) -> Option<Presentation> {
        self.registry().presentation(id, &self.context())
    }

    pub fn action_enabled(&self, id: ActionId) -> bool {
        self.presentation(id).is_some_and(|item| item.enabled)
    }

    pub fn action_checked(&self, id: ActionId) -> Option<bool> {
        self.presentation(id).and_then(|item| item.checked)
    }

    pub fn perform(&self, id: ActionId) {
        let context = self.context();
        self.registry().perform(id, &context);
    }

    /// 命令面板：打开、关闭、切换。
    pub fn open_palette(&self) {
        self.palette_query.set(SharedString::from(""));
        self.palette.set(true);
        self.palette_selected.set(self.first_enabled_palette_item());
    }

    pub fn close_palette(&self) {
        self.palette.set(false);
    }

    /// 按当前查询过滤出的动作（不追踪信号，供事件处理使用）。
    pub fn palette_actions(&self) -> Vec<(ActionId, Presentation)> {
        let query = self.palette_query.get_untracked().to_lowercase();
        self.registry()
            .presentations(&self.context())
            .into_iter()
            .filter(|(id, item)| {
                item.visible
                    && *id != "help.find_action"
                    && (query.is_empty()
                        || item.text.to_lowercase().contains(&query)
                        || item.description.to_lowercase().contains(&query)
                        || id.to_lowercase().contains(&query))
            })
            .collect()
    }

    fn first_enabled_palette_item(&self) -> usize {
        self.palette_actions()
            .iter()
            .position(|(_, item)| item.enabled)
            .unwrap_or(0)
    }

    /// 上 / 下移动命令面板的高亮，跳过禁用项。
    pub fn palette_move(&self, delta: isize) {
        if !self.palette.get_untracked() {
            return;
        }
        let items = self.palette_actions();
        if items.is_empty() {
            return;
        }
        let len = items.len();
        let mut index = self.palette_selected.get_untracked().min(len - 1);
        for _ in 0..len {
            index = (index as isize + delta).rem_euclid(len as isize) as usize;
            if items[index].1.enabled {
                self.palette_selected.set(index);
                return;
            }
        }
    }

    /// 执行命令面板当前高亮的动作。
    pub fn perform_palette_selection(&self) {
        let index = self.palette_selected.get_untracked();
        self.perform_palette_at(index);
    }

    /// 执行命令面板第 `index` 项。
    pub fn perform_palette_at(&self, index: usize) {
        let Some((id, item)) = self.palette_actions().get(index).cloned() else {
            return;
        };
        if !item.enabled {
            return;
        }
        self.palette.set(false);
        self.perform(id);
    }

    /// 查询变化后把高亮落到第一个可用项。
    pub fn palette_query_changed(&self) {
        self.palette_selected.set(self.first_enabled_palette_item());
    }

    /// 工具窗口条 / 视图菜单：同一个窗口再点一次收起，否则切换过去并展开。
    pub fn toggle_tool(&self, tool: Tool) {
        if self.tools.get_untracked() && self.tool.get_untracked() == tool {
            self.tools.set(false);
        } else {
            self.tool.set(tool);
            self.tools.set(true);
        }
    }

    pub fn toggle_bottom(&self, bottom: Bottom) {
        if self.bottom.get_untracked() == Some(bottom) {
            self.bottom.set(None);
        } else {
            self.bottom.set(Some(bottom));
        }
    }

    pub fn set_appearance(&self, appearance: Appearance) {
        self.appearance.set(appearance);
        set_color_scheme(appearance);
    }

    /// 在项目树里定位当前文件（Navigate → 在项目中定位）。
    pub fn reveal_active(&self) {
        let Some(path) = self.ws.active_path() else {
            self.ws.say("没有打开的文件");
            return;
        };
        self.reveal_path(&path);
    }

    pub fn reveal_path(&self, path: &Path) {
        if self.ws.reveal(path) {
            self.tool.set(Tool::Project);
            self.tools.set(true);
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            self.ws.say(format!("已在项目中定位 {name}"));
        }
    }

    pub fn about(&self) {
        self.ws.say("Keel 0.1.0 · nsh 0.5.0 spike · Rust UI");
    }

    /// 当前运行配置名：打开的文件名，没有文件时用 IDEA 演示工程的 `main`。
    pub fn run_config_name(&self) -> SharedString {
        self.ws
            .active_path()
            .and_then(|path| {
                path.file_stem()
                    .map(|stem| stem.to_string_lossy().into_owned())
            })
            .unwrap_or_else(|| "main".to_owned())
            .into()
    }

    /// run widget 的主按钮：启动 / 停止「运行」工具窗口。
    pub fn toggle_run(&self) {
        if self.running.get_untracked() {
            self.stop_run();
        } else {
            self.start_run();
        }
    }

    /// 快捷键 Shift+F10 与 run widget 的绿色按钮。
    pub fn start_run(&self) {
        if self.running.get_untracked() {
            return;
        }
        self.running.set(true);
        self.bottom.set(Some(Bottom::Run));
        self.ws.say(format!("正在运行 {}…", self.run_config_name()));
    }

    /// 快捷键 Ctrl+F2 与运行中的红色停止按钮。
    pub fn stop_run(&self) {
        if !self.running.get_untracked() {
            return;
        }
        self.running.set(false);
        self.ws.say("已停止运行");
    }

    /// run widget 的调试按钮：P4 之前只打开运行窗口并提示。
    pub fn toggle_debug(&self) {
        if self.running.get_untracked() {
            return;
        }
        self.bottom.set(Some(Bottom::Run));
        self.ws.say("调试将在 P4 接入");
    }

    pub fn pick_project(self) {
        spawn(async move {
            match pick_paths(OpenDialog::folders().title("打开项目")).await {
                Ok(Some(paths)) => {
                    if let Some(dir) = paths.into_iter().next() {
                        self.ws.open_project(dir);
                    }
                }
                Ok(None) => {}
                Err(error) => self.ws.say(format!("打开项目失败：{error}")),
            }
        });
    }

    pub fn pick_files(self) {
        spawn(async move {
            match pick_paths(OpenDialog::files().multiple(true).title("打开文件")).await {
                Ok(Some(paths)) => self.ws.open_files(paths),
                Ok(None) => {}
                Err(error) => self.ws.say(format!("打开文件失败：{error}")),
            }
        });
    }
}

/// 外观的稳定标识（动作的选中态比较用）。
pub fn appearance_id(appearance: Appearance) -> &'static str {
    match appearance {
        Appearance::System => "system",
        Appearance::Light => "light",
        Appearance::Dark => "dark",
    }
}

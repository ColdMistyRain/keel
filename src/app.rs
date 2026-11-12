use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use gpui::{prelude::*, *};
use gpui_component::ActiveTheme;
use gpui_component::Sizable as _;
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{
    Copy, Cut, EditorState, Input, InputEvent, InputState, Paste, Redo, SelectAll, TabSize, Undo,
};
use gpui_component::menu::{DropdownMenu as _, PopupMenu};
use gpui_component_assets::IconName;

use crate::actions::{
    CloseAllTabs, CloseEditor, CloseOtherTabs, CloseSettings, CollapseProjectTree, FindInFile,
    LocateInProject, NextTab, OpenProject, PaletteCancel, PaletteConfirm, PaletteDown, PaletteUp,
    PreviousTab, RefreshProjectTree, ReplaceInFile, SaveFile, ShowAbout, ShowCommandPalette,
    ShowSettings, ToggleBottomPanel, ToggleBreadcrumbs, ToggleProjectPanel, ToggleTheme,
};
use crate::editor::Document;
use crate::idea_theme::ThemeKind;
use crate::model::{self, PersistedState, TreeNode, VisibleRow};
use crate::project::{self, ProjectModel};

const MAX_EDITABLE_FILE_SIZE: u64 = 8 * 1024 * 1024;

/// 从主题中取出的界面色板，避免在渲染过程中长期借用 `cx`。
#[derive(Clone)]
pub struct Chrome {
    /// 岛屿之间的画布底色。
    pub canvas: Hsla,
    /// 编辑器/工具窗口岛内部底色。
    pub bg: Hsla,
    pub fg: Hsla,
    pub border: Hsla,
    pub panel: Hsla,
    pub panel_fg: Hsla,
    pub muted_fg: Hsla,
    /// 面包屑等更次要的文字色。
    pub secondary_fg: Hsla,
    pub accent: Hsla,
    pub accent_fg: Hsla,
    pub header: Hsla,
    pub hover_bg: Hsla,
    /// 控件的中性悬停底色。
    pub control_hover: Hsla,
    /// 列表/树选中态的背景与文字色。
    pub selection: Hsla,
    pub selection_fg: Hsla,
    pub tab_underline: Hsla,
    pub tab_bg: Hsla,
    /// 选中标签的浅蓝底色。
    pub tab_selected_bg: Hsla,
    /// 未选中标签的文字色（前景色 75% 透明，`EditorTabs.unselectedAlpha`）。
    pub tab_fg_unselected: Hsla,
    pub status_bg: Hsla,
    /// 岛屿卡片的圆角半径（`Island.arc`）。
    pub island_arc: Pixels,
    /// 图标资源的主题后缀：`""` 或 `"_dark"`。
    pub icon_suffix: &'static str,
    pub mono_font: SharedString,
    pub mono_size: Pixels,
}

impl Chrome {
    pub fn from_theme(cx: &Context<KeelApp>, kind: ThemeKind) -> Self {
        let theme = cx.theme();
        let tokens = kind.tokens();
        Self {
            canvas: crate::idea_theme::color(tokens.canvas),
            bg: crate::idea_theme::color(tokens.content),
            fg: theme.foreground,
            border: theme.border,
            panel: crate::idea_theme::color(tokens.tool_window_header),
            panel_fg: theme.foreground,
            muted_fg: theme.muted_foreground,
            secondary_fg: crate::idea_theme::color(tokens.secondary),
            accent: crate::idea_theme::color(tokens.brand),
            accent_fg: theme.primary_foreground,
            header: crate::idea_theme::color(tokens.tool_window_header),
            hover_bg: crate::idea_theme::color_rgba(tokens.hover),
            control_hover: crate::idea_theme::color_rgba(tokens.control_hover),
            selection: crate::idea_theme::color(tokens.selection),
            selection_fg: crate::idea_theme::color(tokens.selection_fg),
            tab_underline: crate::idea_theme::color(tokens.tab_underline),
            tab_bg: crate::idea_theme::color(tokens.tab_background),
            tab_selected_bg: crate::idea_theme::color(tokens.tab_selected_background),
            tab_fg_unselected: {
                let mut color = theme.foreground;
                color.alpha = 0.75;
                color
            },
            status_bg: crate::idea_theme::color(tokens.status_bar),
            island_arc: px(tokens.island_arc),
            icon_suffix: if tokens.dark_icons { "_dark" } else { "" },
            mono_font: theme.mono_font_family.clone(),
            mono_size: theme.mono_font_size,
        }
    }
}

/// 底部面板当前标签页。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BottomTab {
    #[default]
    Problems,
    Run,
    Terminal,
}

impl BottomTab {
    pub const ALL: [BottomTab; 3] = [BottomTab::Problems, BottomTab::Run, BottomTab::Terminal];

    pub fn label(self) -> &'static str {
        match self {
            BottomTab::Problems => "问题",
            BottomTab::Run => "运行",
            BottomTab::Terminal => "终端",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            BottomTab::Problems => "problems",
            BottomTab::Run => "run",
            BottomTab::Terminal => "terminal",
        }
    }

    pub fn from_key(key: &str) -> Self {
        match key {
            "run" => BottomTab::Run,
            "terminal" => BottomTab::Terminal,
            _ => BottomTab::Problems,
        }
    }

    pub fn index(self) -> usize {
        match self {
            BottomTab::Problems => 0,
            BottomTab::Run => 1,
            BottomTab::Terminal => 2,
        }
    }
}

/// 状态栏提示。
#[derive(Clone, Debug)]
pub struct StatusMessage {
    pub text: String,
    pub is_error: bool,
}

impl StatusMessage {
    pub fn info(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            is_error: false,
        }
    }

    pub fn error(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            is_error: true,
        }
    }
}

/// 项目面板的状态。
#[derive(Default)]
pub struct ProjectState {
    pub root: Option<PathBuf>,
    pub tree: Option<TreeNode>,
    pub expanded: HashSet<PathBuf>,
    pub rows: Vec<VisibleRow>,
    pub selected: Option<PathBuf>,
    pub model: ProjectModel,
    /// 树滚动位置；「在项目中定位」用它把选中行滚进视野。
    pub scroll: ScrollHandle,
}

/// 命令面板中可执行的命令。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandId {
    OpenProject,
    SaveFile,
    CloseEditor,
    ToggleProjectPanel,
    ToggleBottomPanel,
}

impl CommandId {
    pub const ALL: [CommandId; 5] = [
        CommandId::OpenProject,
        CommandId::SaveFile,
        CommandId::CloseEditor,
        CommandId::ToggleProjectPanel,
        CommandId::ToggleBottomPanel,
    ];

    pub fn label(self) -> &'static str {
        match self {
            CommandId::OpenProject => "打开项目目录",
            CommandId::SaveFile => "保存当前文件",
            CommandId::CloseEditor => "关闭当前编辑器",
            CommandId::ToggleProjectPanel => "显示或隐藏项目面板",
            CommandId::ToggleBottomPanel => "显示或隐藏底部面板",
        }
    }

    pub fn shortcut(self) -> &'static str {
        match self {
            CommandId::OpenProject => "Ctrl+Shift+O",
            CommandId::SaveFile => "Ctrl+S",
            CommandId::CloseEditor => "Ctrl+W",
            CommandId::ToggleProjectPanel => "Alt+1",
            CommandId::ToggleBottomPanel => "Alt+4",
        }
    }
}

pub fn filtered_commands(query: &str) -> Vec<CommandId> {
    let query = query.trim().to_lowercase();
    CommandId::ALL
        .iter()
        .copied()
        .filter(|command| query.is_empty() || command.label().to_lowercase().contains(&query))
        .collect()
}

/// 命令面板的临时状态。
pub struct PaletteState {
    pub query: Entity<InputState>,
    pub selected: usize,
    pub _changes: Subscription,
}

/// 需要在下一帧获得焦点的控件。
enum FocusTarget {
    Document(PathBuf),
    Palette,
    FileDialog,
}

/// 右下角通知气泡的存活时间。
const TOAST_LIFETIME: Duration = Duration::from_secs(4);
/// 最多同时显示的通知条数，超出时丢弃最旧的一条。
const TOAST_LIMIT: usize = 4;

/// 右下角通知气泡。
#[derive(Clone)]
pub struct Toast {
    pub id: u64,
    pub text: String,
    pub is_error: bool,
}

/// 设置窗口当前显示的页面。
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SettingsPane {
    Appearance,
    About,
}

impl SettingsPane {
    pub fn title(self) -> &'static str {
        match self {
            SettingsPane::Appearance => "外观",
            SettingsPane::About => "关于",
        }
    }
}

/// 标签拖拽排序的进行中状态。
#[derive(Clone)]
pub struct TabDrag {
    pub path: PathBuf,
    pub hover: Option<PathBuf>,
}

/// 项目树里正在进行的文件操作。
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FileOpKind {
    NewFile,
    NewFolder,
    Rename,
}

impl FileOpKind {
    pub fn title(self) -> &'static str {
        match self {
            FileOpKind::NewFile => "新建文件",
            FileOpKind::NewFolder => "新建文件夹",
            FileOpKind::Rename => "重命名",
        }
    }

    fn placeholder(self) -> &'static str {
        match self {
            FileOpKind::NewFile => "文件名，例如 Main.java",
            FileOpKind::NewFolder => "文件夹名",
            FileOpKind::Rename => "新名称",
        }
    }
}

pub struct FileOpDialog {
    pub kind: FileOpKind,
    pub base_dir: PathBuf,
    pub target: Option<PathBuf>,
    pub input: Entity<InputState>,
    pub _changes: Subscription,
}

pub struct DeleteConfirm {
    pub path: PathBuf,
    pub is_dir: bool,
}

pub struct KeelApp {
    pub project: ProjectState,
    pub documents: Vec<Document>,
    pub active: Option<usize>,
    /// 标签栏滚动位置；打开或切换标签时把目标滚进视野。
    pub tab_scroll: ScrollHandle,
    pub tab_drag: Option<TabDrag>,
    pub project_panel_visible: bool,
    pub bottom_panel_visible: bool,
    /// 当前主题：IntelliJ Light 或 Darcula。
    pub theme_kind: ThemeKind,
    /// 导航栏面包屑，IDEA 里默认关闭，这里同样默认不显示。
    pub breadcrumbs_visible: bool,
    pub project_panel_width: f32,
    pub resizing_project_panel: bool,
    pub bottom_tab: BottomTab,
    pub status: StatusMessage,
    /// 右下角通知气泡，最新的排在最后。
    pub toasts: Vec<Toast>,
    next_toast_id: u64,
    /// 正在运行的后台任务数，状态栏据此显示进度提示。
    pub busy_tasks: usize,
    /// 设置窗口，打开时覆盖在主界面上。
    pub settings: Option<SettingsPane>,
    /// 是否使用次像素（ClearType）文本渲染。
    pub subpixel_text: bool,
    pub palette: Option<PaletteState>,
    pub pending_close: Option<PathBuf>,
    pub file_op: Option<FileOpDialog>,
    pub delete_confirm: Option<DeleteConfirm>,
    pending_focus: Option<FocusTarget>,
    focus_retries: u8,
    pub persisted: PersistedState,
    pub focus_handle: FocusHandle,
    pub tasks: Vec<Task<()>>,
}

impl KeelApp {
    pub fn new(window: &mut Window, persisted: PersistedState, cx: &mut Context<Self>) -> Self {
        let mut this = Self {
            project: ProjectState::default(),
            documents: Vec::new(),
            active: None,
            tab_scroll: ScrollHandle::new(),
            tab_drag: None,
            project_panel_visible: persisted.project_panel_visible,
            bottom_panel_visible: persisted.bottom_panel_visible,
            theme_kind: if persisted.theme_dark.unwrap_or(false) {
                ThemeKind::Dark
            } else {
                ThemeKind::Light
            },
            // IDEA 新 UI 默认显示底部面包屑。
            breadcrumbs_visible: persisted.breadcrumbs_visible.unwrap_or(true),
            project_panel_width: persisted
                .project_panel_width
                .unwrap_or(220.)
                .clamp(180., 600.),
            resizing_project_panel: false,
            bottom_tab: persisted
                .bottom_tab
                .as_deref()
                .map(BottomTab::from_key)
                .unwrap_or_default(),
            status: StatusMessage::info(""),
            toasts: Vec::new(),
            next_toast_id: 0,
            busy_tasks: 0,
            settings: None,
            subpixel_text: persisted.subpixel_text.unwrap_or(false),
            palette: None,
            pending_close: None,
            file_op: None,
            delete_confirm: None,
            pending_focus: None,
            focus_retries: 0,
            focus_handle: cx.focus_handle(),
            persisted,
            tasks: Vec::new(),
        };

        if let Some(root) = this.persisted.last_workspace.clone() {
            if root.is_dir() {
                this.load_workspace(root, cx);
            } else {
                this.persisted.last_workspace = None;
            }
        }

        this.focus_handle.focus(window, cx);
        this
    }

    pub fn index_of(&self, path: &Path) -> Option<usize> {
        self.documents.iter().position(|doc| doc.path == path)
    }

    pub fn active_document(&self) -> Option<&Document> {
        self.active.and_then(|idx| self.documents.get(idx))
    }

    pub fn refresh_rows(&mut self) {
        self.project.rows = match &self.project.tree {
            Some(tree) => model::flatten_visible(tree, &self.project.expanded),
            None => Vec::new(),
        };
    }

    pub fn persist(&mut self) {
        // 配置写入失败不影响主流程，避免在状态栏制造噪音。
        let _ = self.persisted.save();
    }

    pub fn run_command(&mut self, command: CommandId, cx: &mut Context<Self>) {
        match command {
            CommandId::OpenProject => self.open_project(cx),
            CommandId::SaveFile => match self.active.and_then(|idx| self.documents.get(idx)) {
                Some(doc) => {
                    let path = doc.path.clone();
                    self.save_document(path, cx);
                }
                None => {
                    self.status = StatusMessage::error("没有可以保存的文件");
                    cx.notify();
                }
            },
            CommandId::CloseEditor => {
                if let Some(path) = self.active_document().map(|doc| doc.path.clone()) {
                    self.request_close(path, cx);
                }
            }
            CommandId::ToggleProjectPanel => {
                self.project_panel_visible = !self.project_panel_visible;
                self.persisted.project_panel_visible = self.project_panel_visible;
                self.persist();
                cx.notify();
            }
            CommandId::ToggleBottomPanel => {
                self.bottom_panel_visible = !self.bottom_panel_visible;
                self.persisted.bottom_panel_visible = self.bottom_panel_visible;
                self.persist();
                cx.notify();
            }
        }
    }

    pub fn open_project(&mut self, cx: &mut Context<Self>) {
        self.status = StatusMessage::info("等待选择项目目录…");
        cx.notify();
        let task = cx.spawn(async move |this, cx| {
            let picked = cx
                .background_spawn(async move {
                    rfd::FileDialog::new()
                        .set_title("选择项目目录")
                        .pick_folder()
                })
                .await;
            let Some(root) = picked else {
                this.update(cx, |app, cx| {
                    app.status = StatusMessage::info("已取消打开项目");
                    cx.notify();
                })
                .ok();
                return;
            };
            this.update(cx, |app, cx| app.load_workspace(root, cx)).ok();
        });
        self.tasks.push(task);
    }

    pub fn load_workspace(&mut self, root: PathBuf, cx: &mut Context<Self>) {
        self.project = ProjectState::default();
        self.refresh_rows();
        self.persisted.last_workspace = Some(root.clone());
        self.persist();
        cx.notify();

        let task = cx.spawn(async move |this, cx| {
            let read_root = root.clone();
            let project_root = root.clone();
            let loaded = cx
                .background_spawn(async move {
                    let children = model::read_dir_sorted(&read_root);
                    let info = project::detect(&project_root);
                    (children, info)
                })
                .await;
            this.update(cx, |app, cx| {
                let (children, info) = loaded;
                app.project.model = info;
                match children {
                    Ok(children) => {
                        let mut node = TreeNode::directory(root.clone(), dir_name(&root));
                        node.children = children;
                        node.loaded = true;
                        app.project.root = Some(root.clone());
                        app.project.tree = Some(node);
                        app.project.expanded.insert(root.clone());
                        app.project.selected = Some(root);
                        app.refresh_rows();
                    }
                    Err(err) => {
                        app.status = StatusMessage::error(format!("打开目录失败：{err}"));
                    }
                }
                cx.notify();
            })
            .ok();
        });
        self.tasks.push(task);
    }

    pub fn toggle_dir(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if self.project.expanded.contains(&path) {
            self.project.expanded.remove(&path);
            self.refresh_rows();
            cx.notify();
            return;
        }

        self.project.expanded.insert(path.clone());
        let needs_load = self
            .project
            .tree
            .as_ref()
            .and_then(|tree| tree.find(&path))
            .is_some_and(|node| !node.loaded);
        self.refresh_rows();
        cx.notify();

        if needs_load {
            self.load_children(path, cx);
        }
    }

    /// 与 IDEA 的「折叠全部」一致：根节点保持展开，其余目录全部收起。
    pub fn collapse_all_project_tree(&mut self, cx: &mut Context<Self>) {
        let Some(root) = self.project.root.clone() else {
            self.status = StatusMessage::info("尚未打开项目目录");
            cx.notify();
            return;
        };
        self.project.expanded.clear();
        self.project.expanded.insert(root);
        self.refresh_rows();
        self.status = StatusMessage::info("已折叠全部目录");
        cx.notify();
    }

    /// 从磁盘重新读取已加载过的目录，保留当前展开状态。
    pub fn refresh_project_tree(&mut self, cx: &mut Context<Self>) {
        let Some(tree) = self.project.tree.clone() else {
            self.status = StatusMessage::info("尚未打开项目目录");
            cx.notify();
            return;
        };
        self.status = StatusMessage::info("正在从磁盘刷新项目…");
        self.begin_background_task();
        cx.notify();

        let task = cx.spawn(async move |this, cx| {
            let refreshed = cx
                .background_spawn(async move { model::refresh_tree(&tree) })
                .await;
            this.update(cx, |app, cx| {
                app.end_background_task();
                app.project.tree = Some(refreshed);
                app.refresh_rows();
                app.status = StatusMessage::info("项目已从磁盘刷新");
                app.notify("项目已从磁盘刷新", false, cx);
                cx.notify();
            })
            .ok();
        });
        self.tasks.push(task);
    }

    fn load_children(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let task = cx.spawn(async move |this, cx| {
            let read_path = path.clone();
            let loaded = cx
                .background_spawn(async move { model::read_dir_sorted(&read_path) })
                .await;
            this.update(cx, |app, cx| {
                match loaded {
                    Ok(children) => {
                        if let Some(tree) = app.project.tree.as_mut()
                            && let Some(node) = tree.find_mut(&path)
                        {
                            node.children = children;
                            node.loaded = true;
                        }
                        app.refresh_rows();
                    }
                    Err(err) => {
                        app.status = StatusMessage::error(format!("读取目录失败：{err}"));
                    }
                }
                cx.notify();
            })
            .ok();
        });
        self.tasks.push(task);
    }

    pub fn open_file(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(idx) = self.index_of(&path) {
            self.active = Some(idx);
            self.tab_scroll.scroll_to_item(idx);
            self.request_focus(FocusTarget::Document(path));
            cx.notify();
            return;
        }

        cx.notify();

        let task = cx.spawn_in(window, async move |this, cx| {
            let read_path = path.clone();
            let loaded = cx
                .background_spawn(async move { read_text_file(&read_path) })
                .await;
            this.update_in(cx, |app, window, cx| {
                match loaded {
                    Ok(text) => app.insert_document(path, text, window, cx),
                    Err(err) => {
                        app.status = StatusMessage::error(format!("打开文件失败：{err}"));
                    }
                }
                cx.notify();
            })
            .ok();
        });
        self.tasks.push(task);
    }

    fn insert_document(
        &mut self,
        path: PathBuf,
        text: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let language = model::language_for_path(&path);
        let state = cx.new(|cx| {
            let editor = EditorState::new(window, cx)
                .line_number(true)
                .indent_guides(true)
                .soft_wrap(false)
                // 打开编辑器自带的查找/替换面板（Ctrl+F / Ctrl+R）。
                .searchable(true)
                // 与 IDEA 的 Java 默认缩进一致，状态栏显示的也是这个值。
                .tab_size(TabSize {
                    tab_size: 4,
                    hard_tabs: false,
                })
                .default_value(text.as_str());
            match language {
                Some(name) => editor.language(name.to_string()),
                None => editor,
            }
        });

        let changed_path = path.clone();
        let subscription = cx.subscribe(
            &state,
            move |this: &mut KeelApp, _state, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change)
                    && let Some(idx) = this.index_of(&changed_path)
                    && !this.documents[idx].dirty
                {
                    this.documents[idx].dirty = true;
                    cx.notify();
                }
            },
        );

        let observed_path = path.clone();
        let observe = cx.observe(&state, move |this: &mut KeelApp, _state, cx| {
            // 光标位置只影响状态栏，交给下一帧重绘即可。
            if this.index_of(&observed_path).is_some() {
                cx.notify();
            }
        });

        // 换行符以文件实际内容为准，保存时不会悄悄改写已有文件的风格。
        let eol = if text.contains("\r\n") { "CRLF" } else { "LF" };

        self.documents.push(Document::new(
            path.clone(),
            state,
            eol,
            subscription,
            observe,
        ));
        self.active = Some(self.documents.len() - 1);
        self.tab_scroll.scroll_to_item(self.documents.len() - 1);
        self.request_focus(FocusTarget::Document(path.clone()));
    }

    pub fn activate_document(&mut self, idx: usize, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(doc) = self.documents.get(idx) else {
            return;
        };
        self.active = Some(idx);
        self.tab_scroll.scroll_to_item(idx);
        self.request_focus(FocusTarget::Document(doc.path.clone()));
        cx.notify();
    }

    /// 弹出一条右下角通知，到期后自动消失。
    pub fn notify(&mut self, text: impl Into<String>, is_error: bool, cx: &mut Context<Self>) {
        let id = self.next_toast_id;
        self.next_toast_id += 1;
        self.toasts.push(Toast {
            id,
            text: text.into(),
            is_error,
        });
        if self.toasts.len() > TOAST_LIMIT {
            self.toasts.remove(0);
        }
        cx.notify();

        let task = cx.spawn(async move |this, cx| {
            cx.background_executor().timer(TOAST_LIFETIME).await;
            this.update(cx, |app, cx| {
                if app.dismiss_toast(id) {
                    cx.notify();
                }
            })
            .ok();
        });
        self.tasks.push(task);
    }

    /// 手动关闭一条通知（点击气泡）。
    pub fn dismiss_toast(&mut self, id: u64) -> bool {
        let before = self.toasts.len();
        self.toasts.retain(|toast| toast.id != id);
        before != self.toasts.len()
    }

    /// 后台任务计数：进入时 +1，完成时 -1，状态栏据此显示进度。
    fn begin_background_task(&mut self) {
        self.busy_tasks += 1;
    }

    fn end_background_task(&mut self) {
        self.busy_tasks = self.busy_tasks.saturating_sub(1);
    }

    /// 打开设置窗口，`pane` 决定初始显示的页面。
    pub fn open_settings(&mut self, pane: SettingsPane, cx: &mut Context<Self>) {
        self.settings = Some(pane);
        cx.notify();
    }

    /// 关闭设置窗口。
    pub fn close_settings(&mut self, cx: &mut Context<Self>) {
        if self.settings.take().is_some() {
            cx.notify();
        }
    }

    /// 切换到指定主题并记住选择。
    fn set_theme(&mut self, kind: ThemeKind, cx: &mut Context<Self>) {
        if self.theme_kind == kind {
            return;
        }
        self.theme_kind = kind;
        crate::idea_theme::apply(cx, kind);
        self.persisted.theme_dark = Some(kind.is_dark());
        self.persist();
        self.status = StatusMessage::info(if kind.is_dark() {
            "已切换到 Darcula 深色主题"
        } else {
            "已切换到 IntelliJ Light 主题"
        });
        cx.notify();
    }

    /// 切换文本抗锯齿方式，立即生效并记住选择。
    fn set_subpixel_text(&mut self, on: bool, cx: &mut Context<Self>) {
        if self.subpixel_text == on {
            return;
        }
        self.subpixel_text = on;
        cx.set_text_rendering_mode(if on {
            TextRenderingMode::Subpixel
        } else {
            TextRenderingMode::Grayscale
        });
        self.persisted.subpixel_text = Some(on);
        self.persist();
        self.status = StatusMessage::info(if on {
            "已启用次像素文本渲染"
        } else {
            "已启用灰度文本渲染"
        });
        cx.notify();
    }

    /// 按下标签开始拖拽；真正的换位在鼠标松开时一次性完成。
    pub fn begin_tab_drag(&mut self, path: PathBuf) {
        self.tab_drag = Some(TabDrag { path, hover: None });
    }

    /// 拖拽经过某个标签时记录落点，返回状态是否需要重绘（高亮）。
    pub fn hover_tab(&mut self, path: PathBuf) -> bool {
        let Some(drag) = &mut self.tab_drag else {
            return false;
        };
        if drag.hover.as_ref() == Some(&path) {
            return false;
        }
        drag.hover = Some(path);
        true
    }

    pub fn finish_tab_drag(&mut self, cx: &mut Context<Self>) {
        let Some(drag) = self.tab_drag.take() else {
            return;
        };
        if let Some(target) = drag.hover
            && target != drag.path
        {
            self.move_document(&drag.path, &target);
        }
        cx.notify();
    }

    /// 把 `from` 标签移动到 `to` 标签原来的位置，并保持当前激活文档不变。
    fn move_document(&mut self, from: &Path, to: &Path) -> bool {
        let active_path = self.active_document().map(|doc| doc.path.clone());
        let (Some(from), Some(to)) = (self.index_of(from), self.index_of(to)) else {
            return false;
        };
        if from == to {
            return false;
        }
        let doc = self.documents.remove(from);
        self.documents.insert(to, doc);
        self.active = active_path.and_then(|path| self.index_of(&path));
        true
    }

    pub fn save_document(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let Some(idx) = self.index_of(&path) else {
            return;
        };
        let text = self.documents[idx].state.read(cx).value().to_string();
        self.status = StatusMessage::info(format!("正在保存 {}", short_name(&path)));
        self.begin_background_task();
        cx.notify();

        let task = cx.spawn(async move |this, cx| {
            let write_path = path.clone();
            let result = cx
                .background_spawn(async move { std::fs::write(&write_path, text) })
                .await;
            this.update(cx, |app, cx| {
                app.end_background_task();
                match result {
                    Ok(()) => {
                        if let Some(idx) = app.index_of(&path) {
                            app.documents[idx].dirty = false;
                        }
                        let message = format!("已保存 {}", short_name(&path));
                        app.status = StatusMessage::info(message.clone());
                        app.notify(message, false, cx);
                    }
                    Err(err) => {
                        let message = format!("保存失败：{err}");
                        app.status = StatusMessage::error(message.clone());
                        app.notify(message, true, cx);
                    }
                }
                cx.notify();
            })
            .ok();
        });
        self.tasks.push(task);
    }

    pub fn request_close(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        match self.index_of(&path) {
            Some(idx) if self.documents[idx].dirty => {
                self.pending_close = Some(path);
                cx.notify();
            }
            Some(_) => self.close_document(path, cx),
            None => {}
        }
    }

    pub fn close_document(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let Some(idx) = self.index_of(&path) else {
            return;
        };
        self.documents.remove(idx);
        if self.documents.is_empty() {
            self.active = None;
        } else {
            self.active = Some(idx.min(self.documents.len() - 1));
        }
        if self.pending_close.as_ref() == Some(&path) {
            self.pending_close = None;
        }
        cx.notify();
    }

    pub fn save_and_close(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let Some(idx) = self.index_of(&path) else {
            self.pending_close = None;
            cx.notify();
            return;
        };
        let text = self.documents[idx].state.read(cx).value().to_string();
        self.pending_close = None;

        let task = cx.spawn(async move |this, cx| {
            let write_path = path.clone();
            let result = cx
                .background_spawn(async move { std::fs::write(&write_path, text) })
                .await;
            this.update(cx, |app, cx| match result {
                Ok(()) => {
                    app.status = StatusMessage::info(format!("已保存 {}", short_name(&path)));
                    app.close_document(path.clone(), cx);
                }
                Err(err) => {
                    app.status = StatusMessage::error(format!("保存失败，未关闭标签：{err}"));
                    cx.notify();
                }
            })
            .ok();
        });
        self.tasks.push(task);
        cx.notify();
    }

    pub fn discard_and_close(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.pending_close = None;
        self.close_document(path, cx);
    }

    /// 标签右键菜单「关闭其他」：有未保存修改的标签保留，避免无提示丢内容。
    pub fn close_other_documents(&mut self, keep: &Path, cx: &mut Context<Self>) {
        let (closed, kept) = self.remove_documents(|doc| doc.path != keep);
        self.active = self.index_of(keep).or(if self.documents.is_empty() {
            None
        } else {
            Some(0)
        });
        self.status = close_summary(closed, kept, "没有其他标签页可关闭");
        cx.notify();
    }

    /// 标签右键菜单「关闭全部」：同样只关闭没有未保存修改的标签。
    pub fn close_all_documents(&mut self, cx: &mut Context<Self>) {
        let (closed, kept) = self.remove_documents(|_| true);
        self.active = if self.documents.is_empty() {
            None
        } else {
            Some(0)
        };
        if kept == 0 {
            self.pending_close = None;
        }
        self.status = close_summary(closed, kept, "没有可以关闭的标签页");
        cx.notify();
    }

    /// 删除满足条件的标签，返回（已关闭数，因未保存而保留数）。
    fn remove_documents(&mut self, matches: impl Fn(&Document) -> bool) -> (usize, usize) {
        let mut closed = 0;
        let mut kept = 0;
        let mut ix = 0;
        while ix < self.documents.len() {
            if !matches(&self.documents[ix]) {
                ix += 1;
            } else if self.documents[ix].dirty {
                kept += 1;
                ix += 1;
            } else {
                self.documents.remove(ix);
                closed += 1;
            }
        }
        (closed, kept)
    }

    /// 标签右键菜单「在项目中定位」：展开祖先目录并选中文件。
    pub fn locate_in_project(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let Some(root) = self.project.root.clone() else {
            self.status = StatusMessage::error("尚未打开项目目录");
            cx.notify();
            return;
        };
        if !path.starts_with(&root) {
            self.status = StatusMessage::error("该文件不在当前项目中");
            cx.notify();
            return;
        }

        // 从文件向上收集祖先目录，再翻转成根到父级的顺序。
        let mut ancestors = Vec::new();
        let mut cursor = path.parent();
        while let Some(dir) = cursor {
            ancestors.push(dir.to_path_buf());
            if dir == root {
                break;
            }
            cursor = dir.parent();
        }
        ancestors.reverse();

        let mut pending_loads = Vec::new();
        for dir in &ancestors {
            self.project.expanded.insert(dir.clone());
            let needs_load = self
                .project
                .tree
                .as_ref()
                .and_then(|tree| tree.find(dir))
                .is_some_and(|node| !node.loaded);
            if needs_load {
                pending_loads.push(dir.clone());
            }
        }

        if !self.project_panel_visible {
            self.project_panel_visible = true;
            self.persisted.project_panel_visible = true;
            self.persist();
        }
        self.project.selected = Some(path.clone());
        self.refresh_rows();
        cx.notify();

        if pending_loads.is_empty() {
            self.reveal_project_row(&path);
            cx.notify();
            return;
        }

        // 目录按需加载，链式读完祖先目录后行才会出现。
        let task = cx.spawn(async move |this, cx| {
            for dir in pending_loads {
                let read_path = dir.clone();
                let loaded = cx
                    .background_spawn(async move { model::read_dir_sorted(&read_path) })
                    .await;
                this.update(cx, |app, cx| {
                    match loaded {
                        Ok(children) => {
                            if let Some(tree) = app.project.tree.as_mut()
                                && let Some(node) = tree.find_mut(&dir)
                            {
                                node.children = children;
                                node.loaded = true;
                            }
                        }
                        Err(err) => {
                            app.status = StatusMessage::error(format!("读取目录失败：{err}"));
                        }
                    }
                    app.refresh_rows();
                    cx.notify();
                })
                .ok();
            }
            this.update(cx, |app, cx| {
                app.reveal_project_row(&path);
                cx.notify();
            })
            .ok();
        });
        self.tasks.push(task);
    }

    /// 定位完成后滚动到选中行；文件被忽略目录挡住时在状态栏说明。
    fn reveal_project_row(&mut self, path: &Path) {
        match self.project.rows.iter().position(|row| row.path == path) {
            Some(ix) => {
                self.project.scroll.scroll_to_item(ix);
                self.status = StatusMessage::info(format!("已在项目中定位 {}", short_name(path)));
            }
            None => {
                self.status =
                    StatusMessage::error("无法在项目树中定位该文件（可能位于已忽略的目录）");
            }
        }
    }

    /// 项目树右键「新建文件」。
    pub fn open_new_file_dialog(
        &mut self,
        dir: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_file_op(FileOpKind::NewFile, dir, None, "", window, cx);
    }

    /// 项目树右键「新建文件夹」。
    pub fn open_new_folder_dialog(
        &mut self,
        dir: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_file_op(FileOpKind::NewFolder, dir, None, "", window, cx);
    }

    /// 项目树右键「重命名」：输入框预填原名称。
    pub fn open_rename_dialog(
        &mut self,
        path: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let base_dir = path
            .parent()
            .map(|parent| parent.to_path_buf())
            .unwrap_or_else(|| path.clone());
        let initial = short_name(&path);
        self.open_file_op(
            FileOpKind::Rename,
            base_dir,
            Some(path),
            &initial,
            window,
            cx,
        );
    }

    fn open_file_op(
        &mut self,
        kind: FileOpKind,
        base_dir: PathBuf,
        target: Option<PathBuf>,
        initial: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(kind.placeholder())
                .default_value(initial)
        });
        let subscription = cx.subscribe(&input, |this: &mut KeelApp, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::PressEnter { .. }) {
                this.confirm_file_op(cx);
            }
        });
        self.file_op = Some(FileOpDialog {
            kind,
            base_dir,
            target,
            input,
            _changes: subscription,
        });
        self.request_focus(FocusTarget::FileDialog);
        cx.notify();
    }

    pub fn cancel_file_op(&mut self, cx: &mut Context<Self>) {
        self.file_op = None;
        self.request_focus_root();
        cx.notify();
    }

    /// 在后台创建文件/文件夹或重命名，完成后只刷新受影响的父目录。
    pub fn confirm_file_op(&mut self, cx: &mut Context<Self>) {
        let Some(dialog) = self.file_op.take() else {
            return;
        };
        let name = dialog.input.read(cx).value().trim().to_string();
        if name.is_empty() || name.contains(['/', '\\']) || name == "." || name == ".." {
            self.file_op = Some(dialog);
            self.status = StatusMessage::error("名称不能为空，且不能包含路径分隔符");
            cx.notify();
            return;
        }

        let kind = dialog.kind;
        let target = dialog.base_dir.join(&name);
        let old = dialog.target;
        if kind == FileOpKind::Rename && old.as_deref() == Some(target.as_path()) {
            self.request_focus_root();
            cx.notify();
            return;
        }

        self.request_focus_root();
        self.begin_background_task();
        let op_target = target.clone();
        let task = cx.spawn(async move |this, cx| {
            let op_old = old.clone();
            let result = cx
                .background_spawn(async move {
                    match kind {
                        FileOpKind::NewFile => std::fs::OpenOptions::new()
                            .write(true)
                            .create_new(true)
                            .open(&op_target)
                            .map(|_| ()),
                        FileOpKind::NewFolder => std::fs::create_dir(&op_target),
                        FileOpKind::Rename => match op_old {
                            Some(old) => std::fs::rename(old, &op_target),
                            None => Err(io::Error::new(io::ErrorKind::InvalidInput, "缺少原路径")),
                        },
                    }
                })
                .await;

            this.update(cx, |app, cx| {
                app.end_background_task();
                match result {
                    Ok(()) => {
                        if kind == FileOpKind::Rename
                            && let Some(old) = &old
                        {
                            app.remap_document_paths(old, &target);
                        }
                        app.after_file_op(kind, &target, cx);
                    }
                    Err(err) => {
                        let message = format!("{}失败：{err}", kind.title());
                        app.status = StatusMessage::error(message.clone());
                        app.notify(message, true, cx);
                        cx.notify();
                    }
                }
            })
            .ok();
        });
        self.tasks.push(task);
    }

    /// 文件操作成功后展开父目录、选中目标并刷新树行。
    fn after_file_op(&mut self, kind: FileOpKind, target: &Path, cx: &mut Context<Self>) {
        if kind != FileOpKind::Rename
            && let Some(parent) = target.parent()
        {
            self.project.expanded.insert(parent.to_path_buf());
        }
        if let Some(parent) = target.parent() {
            self.load_children(parent.to_path_buf(), cx);
        }
        self.project.selected = Some(target.to_path_buf());
        let message = match kind {
            FileOpKind::NewFile => format!("已新建文件 {}", short_name(target)),
            FileOpKind::NewFolder => format!("已新建文件夹 {}", short_name(target)),
            FileOpKind::Rename => format!("已重命名为 {}", short_name(target)),
        };
        self.status = StatusMessage::info(message.clone());
        self.notify(message, false, cx);
        self.refresh_rows();
        cx.notify();
    }

    /// 重命名时同步已打开文档的路径，避免标题和保存目标失效。
    fn remap_document_paths(&mut self, old: &Path, new: &Path) {
        for doc in &mut self.documents {
            if doc.path == old {
                doc.path = new.to_path_buf();
            } else if let Ok(rest) = doc.path.strip_prefix(old) {
                doc.path = new.join(rest);
            }
        }

        let remapped: Vec<PathBuf> = self
            .project
            .expanded
            .iter()
            .filter_map(|path| path.strip_prefix(old).ok().map(|rest| new.join(rest)))
            .collect();
        self.project.expanded.retain(|path| !path.starts_with(old));
        self.project.expanded.extend(remapped);
    }

    /// 项目树右键「删除」：先弹确认框。
    pub fn request_delete(&mut self, path: PathBuf, is_dir: bool, cx: &mut Context<Self>) {
        self.delete_confirm = Some(DeleteConfirm { path, is_dir });
        cx.notify();
    }

    pub fn cancel_delete(&mut self, cx: &mut Context<Self>) {
        self.delete_confirm = None;
        cx.notify();
    }

    pub fn confirm_delete(&mut self, cx: &mut Context<Self>) {
        let Some(pending) = self.delete_confirm.take() else {
            return;
        };
        let path = pending.path;
        let is_dir = pending.is_dir;
        let op_path = path.clone();
        self.begin_background_task();
        let task = cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    if is_dir {
                        std::fs::remove_dir_all(&op_path)
                    } else {
                        std::fs::remove_file(&op_path)
                    }
                })
                .await;

            this.update(cx, |app, cx| {
                app.end_background_task();
                match result {
                    Ok(()) => {
                        app.close_documents_under(&path, cx);
                        if let Some(parent) = path.parent() {
                            app.load_children(parent.to_path_buf(), cx);
                        }
                        if app.project.selected.as_deref().is_some_and(|selected| {
                            selected == path.as_path() || selected.starts_with(path.as_path())
                        }) {
                            app.project.selected = path.parent().map(|parent| parent.to_path_buf());
                        }
                        let message = format!("已删除 {}", short_name(&path));
                        app.status = StatusMessage::info(message.clone());
                        app.notify(message, false, cx);
                    }
                    Err(err) => {
                        let message = format!("删除失败：{err}");
                        app.status = StatusMessage::error(message.clone());
                        app.notify(message, true, cx);
                    }
                }
                app.refresh_rows();
                cx.notify();
            })
            .ok();
        });
        self.tasks.push(task);
    }

    /// 删除文件/目录后关闭其下所有编辑器，避免留下指向已删除路径的标签。
    fn close_documents_under(&mut self, path: &Path, _cx: &mut Context<Self>) {
        let active_path = self.active_document().map(|doc| doc.path.clone());
        self.documents
            .retain(|doc| doc.path != path && !doc.path.starts_with(path));
        self.active = active_path.and_then(|path| self.index_of(&path));
        if self
            .pending_close
            .as_deref()
            .is_some_and(|pending| pending == path || pending.starts_with(path))
        {
            self.pending_close = None;
        }
    }

    pub fn open_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let query = cx.new(|cx| InputState::new(window, cx).placeholder("输入命令名称…"));
        let subscription = cx.subscribe(
            &query,
            |this: &mut KeelApp, _state, event: &InputEvent, cx| match event {
                InputEvent::Change => {
                    if let Some(palette) = this.palette.as_mut() {
                        palette.selected = 0;
                    }
                    cx.notify();
                }
                InputEvent::PressEnter { .. } => this.confirm_palette(cx),
                _ => {}
            },
        );
        self.palette = Some(PaletteState {
            query,
            selected: 0,
            _changes: subscription,
        });
        self.request_focus(FocusTarget::Palette);
        cx.notify();
    }

    fn confirm_palette(&mut self, cx: &mut Context<Self>) {
        let Some(palette) = &self.palette else {
            return;
        };
        let query = palette.query.read(cx).value().to_string();
        let matches = filtered_commands(&query);
        let index = palette.selected.min(matches.len().saturating_sub(1));
        let Some(command) = matches.get(index).copied() else {
            return;
        };
        self.palette = None;
        self.run_command(command, cx);
    }

    pub fn close_palette(&mut self, cx: &mut Context<Self>) {
        self.palette = None;
        self.request_focus_root();
        cx.notify();
    }

    pub fn palette_match_count(&self, cx: &Context<Self>) -> usize {
        match &self.palette {
            Some(palette) => {
                let query = palette.query.read(cx).value().to_lowercase();
                filtered_commands(&query).len()
            }
            None => 0,
        }
    }

    fn request_focus(&mut self, target: FocusTarget) {
        self.pending_focus = Some(target);
        self.focus_retries = 2;
    }

    fn request_focus_root(&mut self) {
        // 面板关闭后焦点交还根视图即可，无需重试。
        self.pending_focus = None;
    }

    fn apply_pending_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.focus_retries == 0 {
            self.pending_focus = None;
            return;
        }
        let Some(target) = self.pending_focus.take() else {
            return;
        };
        self.focus_retries -= 1;

        let handle = match &target {
            FocusTarget::Document(path) => self
                .index_of(path)
                .map(|idx| self.documents[idx].state.focus_handle(cx)),
            FocusTarget::Palette => self.palette.as_ref().map(|p| p.query.focus_handle(cx)),
            FocusTarget::FileDialog => self
                .file_op
                .as_ref()
                .map(|dialog| dialog.input.focus_handle(cx)),
        };

        let focused = match handle {
            Some(handle) => {
                if handle.is_focused(window) {
                    true
                } else {
                    handle.focus(window, cx);
                    false
                }
            }
            None => true,
        };

        if !focused && self.focus_retries > 0 {
            self.pending_focus = Some(target);
        }
    }

    fn on_open_project(&mut self, _: &OpenProject, _window: &mut Window, cx: &mut Context<Self>) {
        self.run_command(CommandId::OpenProject, cx);
    }

    fn on_save_file(&mut self, _: &SaveFile, _window: &mut Window, cx: &mut Context<Self>) {
        self.run_command(CommandId::SaveFile, cx);
    }

    fn on_close_editor(&mut self, _: &CloseEditor, _window: &mut Window, cx: &mut Context<Self>) {
        self.run_command(CommandId::CloseEditor, cx);
    }

    fn on_toggle_project_panel(
        &mut self,
        _: &ToggleProjectPanel,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.run_command(CommandId::ToggleProjectPanel, cx);
    }

    fn on_toggle_bottom_panel(
        &mut self,
        _: &ToggleBottomPanel,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.run_command(CommandId::ToggleBottomPanel, cx);
    }

    fn on_collapse_project_tree(
        &mut self,
        _: &CollapseProjectTree,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.collapse_all_project_tree(cx);
    }

    fn on_refresh_project_tree(
        &mut self,
        _: &RefreshProjectTree,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.refresh_project_tree(cx);
    }

    /// 在 IntelliJ Light 与 Darcula 之间切换，并记住选择。
    fn on_toggle_theme(&mut self, _: &ToggleTheme, _window: &mut Window, cx: &mut Context<Self>) {
        let kind = if self.theme_kind.is_dark() {
            ThemeKind::Light
        } else {
            ThemeKind::Dark
        };
        self.set_theme(kind, cx);
    }

    /// 设置入口：已打开时再次触发即关闭。
    fn on_show_settings(&mut self, _: &ShowSettings, _window: &mut Window, cx: &mut Context<Self>) {
        if self.settings.is_some() {
            self.close_settings(cx);
        } else {
            self.open_settings(SettingsPane::Appearance, cx);
        }
    }

    fn on_show_about(&mut self, _: &ShowAbout, _window: &mut Window, cx: &mut Context<Self>) {
        self.open_settings(SettingsPane::About, cx);
    }

    fn on_close_settings(
        &mut self,
        _: &CloseSettings,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.close_settings(cx);
    }

    /// 在当前编辑器里打开查找/替换面板（面板由 gpui-component 的编辑器提供）。
    pub fn open_search_in_file(&mut self, replace_mode: bool, cx: &mut Context<Self>) {
        let Some(idx) = self.active else {
            self.status = StatusMessage::info("先打开一个文件再查找");
            cx.notify();
            return;
        };
        // 焦点交给面板自己管理，这里不能把焦点抢回编辑器，否则输入会落进正文。
        self.documents[idx]
            .state
            .update(cx, |state, cx| state.open_search(replace_mode, cx));
        cx.notify();
    }

    fn on_find_in_file(&mut self, _: &FindInFile, _window: &mut Window, cx: &mut Context<Self>) {
        self.open_search_in_file(false, cx);
    }

    fn on_replace_in_file(
        &mut self,
        _: &ReplaceInFile,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_search_in_file(true, cx);
    }

    fn on_toggle_breadcrumbs(
        &mut self,
        _: &ToggleBreadcrumbs,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.breadcrumbs_visible = !self.breadcrumbs_visible;
        self.persisted.breadcrumbs_visible = Some(self.breadcrumbs_visible);
        self.persist();
        self.status = StatusMessage::info(if self.breadcrumbs_visible {
            "已显示导航栏面包屑"
        } else {
            "已隐藏导航栏面包屑"
        });
        cx.notify();
    }

    fn on_next_tab(&mut self, _: &NextTab, window: &mut Window, cx: &mut Context<Self>) {
        self.cycle_tab(1, window, cx);
    }

    fn on_previous_tab(&mut self, _: &PreviousTab, window: &mut Window, cx: &mut Context<Self>) {
        self.cycle_tab(-1, window, cx);
    }

    /// 按打开顺序循环切换标签，没有打开文件时保持安静。
    fn cycle_tab(&mut self, delta: isize, window: &mut Window, cx: &mut Context<Self>) {
        if self.documents.is_empty() {
            return;
        }
        let count = self.documents.len() as isize;
        let current = self.active.map_or(0, |idx| idx as isize);
        let next = (current + delta).rem_euclid(count) as usize;
        self.activate_document(next, window, cx);
    }

    fn on_locate_in_project(
        &mut self,
        _: &LocateInProject,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match self.active_document().map(|doc| doc.path.clone()) {
            Some(path) => self.locate_in_project(path, cx),
            None => {
                self.status = StatusMessage::info("没有打开的编辑器");
                cx.notify();
            }
        }
    }

    fn on_close_other_tabs(
        &mut self,
        _: &CloseOtherTabs,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match self.active_document().map(|doc| doc.path.clone()) {
            Some(path) => self.close_other_documents(&path, cx),
            None => {
                self.status = StatusMessage::info("没有打开的编辑器");
                cx.notify();
            }
        }
    }

    fn on_close_all_tabs(
        &mut self,
        _: &CloseAllTabs,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.close_all_documents(cx);
    }

    fn on_show_palette(
        &mut self,
        _: &ShowCommandPalette,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_palette(window, cx);
    }

    fn on_palette_confirm(
        &mut self,
        _: &PaletteConfirm,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.confirm_palette(cx);
    }

    fn on_palette_cancel(
        &mut self,
        _: &PaletteCancel,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.close_palette(cx);
    }

    fn on_palette_up(&mut self, _: &PaletteUp, _window: &mut Window, cx: &mut Context<Self>) {
        if self.palette.is_none() {
            return;
        }
        let count = self.palette_match_count(cx);
        if let Some(palette) = self.palette.as_mut() {
            palette.selected = if palette.selected == 0 {
                count.saturating_sub(1)
            } else {
                palette.selected - 1
            };
        }
        cx.notify();
    }

    fn on_palette_down(&mut self, _: &PaletteDown, _window: &mut Window, cx: &mut Context<Self>) {
        if self.palette.is_none() {
            return;
        }
        let count = self.palette_match_count(cx);
        if let Some(palette) = self.palette.as_mut() {
            palette.selected = if count == 0 {
                0
            } else {
                (palette.selected + 1) % count
            };
        }
        cx.notify();
    }

    /// IDEA 新 UI 的顶部主工具栏：汉堡菜单、项目、分支、搜索/设置和窗口按钮。
    /// 它同时是自绘标题栏，空白处可以拖动窗口。
    fn render_title_bar(&mut self, chrome: &Chrome, cx: &mut Context<Self>) -> AnyElement {
        let icon = |name: &str| format!("icons/{name}{}.svg", chrome.icon_suffix);
        let project_name = self
            .project
            .root
            .as_deref()
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "Keel".to_string());
        let initials: String = project_name.chars().take(2).collect();
        let branch = self.git_branch();

        let menu = Button::new("main-menu")
            .ghost()
            .small()
            .w(px(28.))
            .h(px(28.))
            .p_0()
            .icon(IconName::Menu)
            .tooltip("主菜单")
            .dropdown_menu(build_main_menu);

        let app_logo = Button::new("app-logo")
            .ghost()
            .small()
            .w(px(28.))
            .h(px(28.))
            .p_0()
            .tooltip("主菜单")
            .child(
                div()
                    .size(px(20.))
                    .rounded(px(4.))
                    .bg(chrome.accent)
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(chrome.accent_fg)
                    .text_xs()
                    .child("K"),
            )
            .dropdown_menu(build_main_menu);

        let project_widget = div()
            .id("title-project")
            .flex()
            .items_center()
            .gap_1()
            .px_1()
            .rounded_sm()
            .cursor_pointer()
            .hover(|style| style.bg(chrome.control_hover))
            .child(
                div()
                    .size(px(20.))
                    .rounded(px(4.))
                    .bg(chrome.accent)
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(chrome.accent_fg)
                    .text_xs()
                    .child(initials),
            )
            .child(div().whitespace_nowrap().child(project_name))
            .child(
                svg()
                    .path(icon("chevron_down"))
                    .size(px(14.))
                    .text_color(chrome.muted_fg),
            )
            .on_click(cx.listener(|this, _event, _window, cx| {
                this.run_command(CommandId::ToggleProjectPanel, cx);
            }));

        let branch_widget = branch.map(|branch| {
            div()
                .id("title-branch")
                .flex()
                .items_center()
                .gap_1()
                .px_1()
                .rounded_sm()
                .text_color(chrome.muted_fg)
                .hover(|style| style.bg(chrome.control_hover))
                .child(
                    svg()
                        .path(icon("branch"))
                        .size(px(16.))
                        .text_color(chrome.muted_fg),
                )
                .child(div().whitespace_nowrap().child(branch))
        });

        let external = Button::new("title-external")
            .small()
            .ghost()
            .w(px(24.))
            .h(px(24.))
            .p_0()
            .icon(IconName::ExternalLink)
            .tooltip("在文件管理器中打开项目")
            .on_click(cx.listener(|this, _event, _window, _cx| {
                this.open_project_in_file_manager();
            }));

        // IDEA 顶部工具栏左侧有一块项目色渐变（600×300 的渐变区域）。
        let seed = self
            .project
            .root
            .as_deref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "Keel".to_string());
        let tint = crate::idea_theme::project_gradient(self.theme_kind, &seed);
        let mut fade = tint;
        fade.alpha = 0.;
        let gradient = div()
            .absolute()
            .top_0()
            .left_0()
            .w(px(600.))
            .h(px(300.))
            .bg(linear_gradient(
                120.,
                linear_color_stop(tint, 0.),
                linear_color_stop(fade, 1.),
            ));

        let control = |id: &'static str, area: WindowControlArea, glyph: &'static str| {
            div()
                .id(id)
                .w(px(46.))
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(chrome.muted_fg)
                .bg(chrome.canvas)
                .hover(|style| style.bg(chrome.control_hover))
                .window_control_area(area)
                .child(glyph)
        };

        div()
            .relative()
            .overflow_hidden()
            .h(px(40.))
            .w_full()
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap_2()
            .pl(px(6.))
            .bg(chrome.canvas)
            .window_control_area(WindowControlArea::Drag)
            .child(gradient)
            .child(app_logo)
            .child(menu)
            .child(project_widget)
            .children(branch_widget)
            .child(external)
            .child(div().flex_1())
            .child(
                icon_button("title-search", icon("search"), chrome.muted_fg)
                    .tooltip("搜索文件与命令")
                    .on_click(
                        cx.listener(|this, _event, window, cx| this.open_palette(window, cx)),
                    ),
            )
            .child(
                icon_button("title-settings", icon("settings"), chrome.muted_fg)
                    .tooltip("设置")
                    .on_click(cx.listener(|this, _event, _window, cx| {
                        this.open_settings(SettingsPane::Appearance, cx)
                    })),
            )
            .child(control("window-min", WindowControlArea::Min, "—"))
            .child(control("window-max", WindowControlArea::Max, "□"))
            .child(control("window-close", WindowControlArea::Close, "✕"))
            .into_any_element()
    }

    /// 在系统文件管理器中打开项目根目录。
    fn open_project_in_file_manager(&mut self) {
        let Some(root) = self.project.root.clone() else {
            return;
        };
        let _ = std::process::Command::new("explorer").arg(root).spawn();
    }

    /// 项目根上的 git 分支名，读取 `.git/HEAD`；不是 git 仓库时返回 `None`。
    fn git_branch(&self) -> Option<String> {
        let root = self.project.root.as_deref()?;
        let head = std::fs::read_to_string(root.join(".git").join("HEAD")).ok()?;
        let head = head.trim();
        head.strip_prefix("ref: refs/heads/")
            .map(|branch| branch.to_string())
    }

    fn render_status_bar(&mut self, chrome: &Chrome, cx: &mut Context<Self>) -> AnyElement {
        let left_color = if self.status.is_error {
            hsla(0.0, 0.65, 0.56, 1.0)
        } else {
            chrome.muted_fg
        };

        // 左侧：项目选择器 + 临时消息与后台任务进度，分支等信息在顶部工具栏。
        let mut left = div().min_w_0().flex().items_center().h_full();
        if let Some(root) = self.project.root.clone() {
            let name = root
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_default();
            left = left.child(
                div()
                    .id("status-project")
                    .flex()
                    .items_center()
                    .gap_1()
                    .h_full()
                    .px_2()
                    .cursor_pointer()
                    .hover(|style| style.bg(chrome.control_hover))
                    .child(div().size(px(12.)).rounded(px(3.)).bg(chrome.accent))
                    .child(div().whitespace_nowrap().child(name))
                    .on_click(cx.listener(|this, _event, _window, cx| {
                        this.run_command(CommandId::ToggleProjectPanel, cx);
                    })),
            );
        }
        if !self.status.text.is_empty() {
            left = left.child(
                div()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .px_2()
                    .text_color(left_color)
                    .child(self.status.text.clone()),
            );
        }
        if self.busy_tasks > 0 {
            left = left.child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .px_2()
                    .text_color(chrome.muted_fg)
                    .child("⟳")
                    .child(format!("后台任务 {}", self.busy_tasks)),
            );
        }

        // 右侧按 IDEA Islands 状态栏的顺序排：行列号、换行符、编码、缩进。
        let mut widgets: Vec<AnyElement> = Vec::new();
        if let Some(doc) = self.active_document() {
            let position = doc.state.read(cx).cursor_position();
            widgets.push(status_widget(
                format!("{}:{}", position.line + 1, position.character + 1),
                chrome,
            ));
            widgets.push(status_widget(doc.eol, chrome));
            widgets.push(status_widget("UTF-8", chrome));
            widgets.push(status_widget("4 个空格", chrome));
        }

        div()
            .flex()
            .items_center()
            .justify_between()
            .h(px(26.))
            .flex_shrink_0()
            .text_xs()
            .bg(chrome.status_bg)
            .child(left)
            .child(div().flex().items_center().h_full().children(widgets))
            .into_any_element()
    }

    /// 右下角通知气泡，点击立即关闭，超时由 `notify` 里的定时任务移除。
    fn render_toasts(&self, chrome: &Chrome, cx: &mut Context<Self>) -> AnyElement {
        if self.toasts.is_empty() {
            return div().into_any_element();
        }

        let mut cards = Vec::new();
        for toast in &self.toasts {
            let id = toast.id;
            let accent = if toast.is_error {
                hsla(0.0, 0.65, 0.56, 1.0)
            } else {
                chrome.accent
            };
            cards.push(
                div()
                    .id(("toast", id as usize))
                    .w(px(340.))
                    .flex()
                    .items_start()
                    .gap_2()
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .bg(chrome.panel)
                    .border_1()
                    .border_color(chrome.border)
                    .text_xs()
                    .text_color(chrome.panel_fg)
                    .cursor_pointer()
                    .child(div().w(px(3.)).h(px(16.)).rounded_sm().bg(accent))
                    .child(div().flex_1().min_w_0().child(toast.text.clone()))
                    .on_click(cx.listener(move |this, _event, _window, cx| {
                        if this.dismiss_toast(id) {
                            cx.notify();
                        }
                    })),
            );
        }

        div()
            .absolute()
            .right(px(12.))
            .bottom(px(32.))
            .flex()
            .flex_col()
            .gap_2()
            .children(cards)
            .into_any_element()
    }

    /// 设置窗口：左侧分类，右侧内容。外观页可切换主题与文本渲染方式。
    fn render_settings(&self, chrome: &Chrome, cx: &mut Context<Self>) -> AnyElement {
        let Some(pane) = self.settings else {
            return div().into_any_element();
        };

        let mut nav_items = Vec::new();
        for item in [SettingsPane::Appearance, SettingsPane::About] {
            let is_active = pane == item;
            nav_items.push(
                div()
                    .id(("settings-nav", item as usize))
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .cursor_pointer()
                    .text_sm()
                    .when(is_active, |row| {
                        row.bg(chrome.accent).text_color(chrome.accent_fg)
                    })
                    .when(!is_active, |row| {
                        row.text_color(chrome.panel_fg)
                            .hover(|style| style.bg(chrome.hover_bg))
                    })
                    .child(item.title())
                    .on_click(cx.listener(move |this, _event, _window, cx| {
                        this.open_settings(item, cx);
                    }))
                    .into_any_element(),
            );
        }

        let content = match pane {
            SettingsPane::Appearance => {
                let light_theme = if self.theme_kind == ThemeKind::Light {
                    Button::new("theme-light").label("浅色").primary()
                } else {
                    Button::new("theme-light").label("浅色").ghost()
                };
                let dark_theme = if self.theme_kind.is_dark() {
                    Button::new("theme-dark").label("深色").primary()
                } else {
                    Button::new("theme-dark").label("深色").ghost()
                };
                let gray_text = if self.subpixel_text {
                    Button::new("text-gray").label("灰度抗锯齿").ghost()
                } else {
                    Button::new("text-gray").label("灰度抗锯齿").primary()
                };
                let subpixel_text = if self.subpixel_text {
                    Button::new("text-subpixel").label("次像素抗锯齿").primary()
                } else {
                    Button::new("text-subpixel").label("次像素抗锯齿").ghost()
                };

                div()
                    .flex()
                    .flex_col()
                    .gap_5()
                    .child(setting_row(
                        "主题",
                        "切换后立即生效，并写入 state.json",
                        div()
                            .flex()
                            .gap_2()
                            .child(light_theme.on_click(cx.listener(
                                |this, _event, _window, cx| {
                                    this.set_theme(ThemeKind::Light, cx);
                                },
                            )))
                            .child(dark_theme.on_click(cx.listener(
                                |this, _event, _window, cx| {
                                    this.set_theme(ThemeKind::Dark, cx);
                                },
                            ))),
                        chrome,
                    ))
                    .child(setting_row(
                        "文本渲染",
                        "灰度抗锯齿字形更干净，次像素渲染更接近 ClearType",
                        div()
                            .flex()
                            .gap_2()
                            .child(gray_text.on_click(cx.listener(|this, _event, _window, cx| {
                                this.set_subpixel_text(false, cx);
                            })))
                            .child(subpixel_text.on_click(cx.listener(
                                |this, _event, _window, cx| {
                                    this.set_subpixel_text(true, cx);
                                },
                            ))),
                        chrome,
                    ))
                    .into_any_element()
            }
            SettingsPane::About => div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    div()
                        .text_xl()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Keel"),
                )
                .child(
                    div()
                        .text_sm()
                        .child(format!("版本 {}", env!("CARGO_PKG_VERSION"))),
                )
                .child(
                    div()
                        .text_sm()
                        .child("使用 Rust 与 GPUI 构建的 IntelliJ 风格桌面编辑器。"),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(chrome.muted_fg)
                        .child("当前阶段：Phase 1 · IDEA UI 复刻收尾"),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(chrome.muted_fg)
                        .child("配置文件：%APPDATA%\\Keel\\state.json"),
                )
                .into_any_element(),
        };

        div()
            .id("settings-backdrop")
            .key_context("KeelSettings")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(hsla(0.0, 0.0, 0.0, 0.45))
            .on_action(cx.listener(Self::on_close_settings))
            .child(
                div()
                    .w(px(640.))
                    .h(px(400.))
                    .flex()
                    .flex_row()
                    .overflow_hidden()
                    .rounded_lg()
                    .bg(chrome.bg)
                    .text_color(chrome.fg)
                    .border_1()
                    .border_color(chrome.border)
                    .shadow_lg()
                    .child(
                        div()
                            .w(px(150.))
                            .h_full()
                            .flex_shrink_0()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .p_2()
                            .bg(chrome.panel)
                            .border_r_1()
                            .border_color(chrome.border)
                            .child(
                                div()
                                    .px_3()
                                    .py_2()
                                    .text_xs()
                                    .text_color(chrome.muted_fg)
                                    .child("设置"),
                            )
                            .children(nav_items),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .p_5()
                            .gap_4()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(pane.title()),
                            )
                            .child(
                                div()
                                    .id("settings-content")
                                    .flex_1()
                                    .min_h_0()
                                    .overflow_y_scroll()
                                    .child(content),
                            )
                            .child(
                                div().flex().justify_end().child(
                                    Button::new("settings-close")
                                        .label("关闭")
                                        .primary()
                                        .on_click(cx.listener(|this, _event, _window, cx| {
                                            this.close_settings(cx);
                                        })),
                                ),
                            ),
                    ),
            )
            .into_any_element()
    }

    fn render_close_confirm(&self, chrome: &Chrome, cx: &mut Context<Self>) -> AnyElement {
        let Some(path) = self.pending_close.clone() else {
            return div().into_any_element();
        };
        let name = short_name(&path);

        let save_path = path.clone();
        let save_and_close = Button::new("confirm-save")
            .label("保存并关闭")
            .primary()
            .on_click(cx.listener(move |this, _event, _window, cx| {
                this.save_and_close(save_path.clone(), cx);
            }));

        let discard_path = path.clone();
        let discard = Button::new("confirm-discard")
            .label("直接关闭")
            .ghost()
            .on_click(cx.listener(move |this, _event, _window, cx| {
                this.discard_and_close(discard_path.clone(), cx);
            }));

        div()
            .id("close-confirm-backdrop")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(hsla(0.0, 0.0, 0.0, 0.45))
            .child(
                div()
                    .w(px(420.))
                    .p_5()
                    .rounded_lg()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .bg(chrome.bg)
                    .text_color(chrome.fg)
                    .border_1()
                    .border_color(chrome.border)
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(format!("{name} 有未保存的修改")),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(chrome.muted_fg)
                            .child("关闭前要先保存吗？"),
                    )
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap_2()
                            .child(
                                Button::new("confirm-cancel")
                                    .label("取消")
                                    .ghost()
                                    .on_click(cx.listener(|this, _event, _window, cx| {
                                        this.pending_close = None;
                                        cx.notify();
                                    })),
                            )
                            .child(discard)
                            .child(save_and_close),
                    ),
            )
            .into_any_element()
    }

    fn render_file_op_dialog(&self, chrome: &Chrome, cx: &mut Context<Self>) -> AnyElement {
        let Some(dialog) = &self.file_op else {
            return div().into_any_element();
        };
        let kind = dialog.kind;

        div()
            .id("file-op-backdrop")
            .key_context("KeelFileDialog")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(hsla(0.0, 0.0, 0.0, 0.45))
            .child(
                div()
                    .w(px(460.))
                    .p_5()
                    .rounded_lg()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .bg(chrome.bg)
                    .text_color(chrome.fg)
                    .border_1()
                    .border_color(chrome.border)
                    .child(div().font_weight(FontWeight::SEMIBOLD).child(kind.title()))
                    .child(
                        div()
                            .text_xs()
                            .text_color(chrome.muted_fg)
                            .child(format!("位置：{}", dialog.base_dir.display())),
                    )
                    .child(Input::new(&dialog.input))
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap_2()
                            .child(
                                Button::new("file-op-cancel")
                                    .label("取消")
                                    .ghost()
                                    .on_click(cx.listener(|this, _event, _window, cx| {
                                        this.cancel_file_op(cx);
                                    })),
                            )
                            .child(
                                Button::new("file-op-confirm")
                                    .label("确定")
                                    .primary()
                                    .on_click(cx.listener(|this, _event, _window, cx| {
                                        this.confirm_file_op(cx);
                                    })),
                            ),
                    ),
            )
            .into_any_element()
    }

    fn render_delete_confirm(&self, chrome: &Chrome, cx: &mut Context<Self>) -> AnyElement {
        let Some(pending) = &self.delete_confirm else {
            return div().into_any_element();
        };
        let name = short_name(&pending.path);
        let what = if pending.is_dir { "目录" } else { "文件" };

        div()
            .id("delete-confirm-backdrop")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(hsla(0.0, 0.0, 0.0, 0.45))
            .child(
                div()
                    .w(px(440.))
                    .p_5()
                    .rounded_lg()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .bg(chrome.bg)
                    .text_color(chrome.fg)
                    .border_1()
                    .border_color(chrome.border)
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(format!("删除{what} {name}？")),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(chrome.muted_fg)
                            .child("此操作不可撤销，相关的已打开编辑器也会被关闭。"),
                    )
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap_2()
                            .child(Button::new("delete-cancel").label("取消").ghost().on_click(
                                cx.listener(|this, _event, _window, cx| {
                                    this.cancel_delete(cx);
                                }),
                            ))
                            .child(
                                Button::new("delete-confirm")
                                    .label("删除")
                                    .danger()
                                    .on_click(cx.listener(|this, _event, _window, cx| {
                                        this.confirm_delete(cx);
                                    })),
                            ),
                    ),
            )
            .into_any_element()
    }
}

impl Render for KeelApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.apply_pending_focus(window, cx);
        let chrome = Chrome::from_theme(cx, self.theme_kind);

        let title_bar = self.render_title_bar(&chrome, cx);
        let status_bar = self.render_status_bar(&chrome, cx);

        let stripe = self.render_tool_window_stripe(&chrome, cx);
        let workspace = self.render_workspace_area(&chrome, cx);

        let body = div()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .flex()
            .flex_row()
            .px(px(4.))
            .pb(px(4.))
            .child(stripe)
            .child(workspace);

        let mut root = div()
            .id("keel-root")
            .key_context("Keel")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_open_project))
            .on_action(cx.listener(Self::on_save_file))
            .on_action(cx.listener(Self::on_close_editor))
            .on_action(cx.listener(Self::on_toggle_project_panel))
            .on_action(cx.listener(Self::on_toggle_bottom_panel))
            .on_action(cx.listener(Self::on_toggle_breadcrumbs))
            .on_action(cx.listener(Self::on_toggle_theme))
            .on_action(cx.listener(Self::on_collapse_project_tree))
            .on_action(cx.listener(Self::on_refresh_project_tree))
            .on_action(cx.listener(Self::on_next_tab))
            .on_action(cx.listener(Self::on_previous_tab))
            .on_action(cx.listener(Self::on_locate_in_project))
            .on_action(cx.listener(Self::on_close_other_tabs))
            .on_action(cx.listener(Self::on_close_all_tabs))
            .on_action(cx.listener(Self::on_show_palette))
            .on_action(cx.listener(Self::on_palette_confirm))
            .on_action(cx.listener(Self::on_palette_cancel))
            .on_action(cx.listener(Self::on_palette_up))
            .on_action(cx.listener(Self::on_palette_down))
            .on_action(cx.listener(Self::on_show_settings))
            .on_action(cx.listener(Self::on_show_about))
            .on_action(cx.listener(Self::on_close_settings))
            .on_action(cx.listener(Self::on_find_in_file))
            .on_action(cx.listener(Self::on_replace_in_file))
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .bg(chrome.canvas)
            .text_color(chrome.fg)
            .child(title_bar)
            .child(body)
            .child(status_bar);

        if self.pending_close.is_some() {
            root = root.child(self.render_close_confirm(&chrome, cx));
        }

        if self.palette.is_some() {
            root = root.child(self.render_palette(&chrome, cx));
        }

        if self.file_op.is_some() {
            root = root.child(self.render_file_op_dialog(&chrome, cx));
        }

        if self.delete_confirm.is_some() {
            root = root.child(self.render_delete_confirm(&chrome, cx));
        }

        if self.settings.is_some() {
            root = root.child(self.render_settings(&chrome, cx));
        }

        root = root.child(self.render_toasts(&chrome, cx));

        root
    }
}

fn read_text_file(path: &Path) -> io::Result<String> {
    let metadata = std::fs::metadata(path)?;
    if metadata.len() > MAX_EDITABLE_FILE_SIZE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "文件超过 {} MB，暂不支持在编辑器中打开",
                MAX_EDITABLE_FILE_SIZE / 1024 / 1024
            ),
        ));
    }
    let bytes = std::fs::read(path)?;
    String::from_utf8(bytes)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "文件不是有效的 UTF-8 文本"))
}

fn short_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

/// 关闭标签后的状态栏提示，未保存的标签会被明确点名保留。
fn close_summary(closed: usize, kept: usize, empty: &str) -> StatusMessage {
    match (closed, kept) {
        (0, 0) => StatusMessage::info(empty),
        (closed, 0) => StatusMessage::info(format!("已关闭 {closed} 个标签")),
        (closed, kept) => StatusMessage::info(format!(
            "已关闭 {closed} 个标签，{kept} 个未保存的标签已保留"
        )),
    }
}

fn dir_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

/// 设置页中的一行：左侧是标题与说明，右侧是控件。
fn setting_row(
    title: &'static str,
    hint: &'static str,
    controls: impl IntoElement,
    chrome: &Chrome,
) -> AnyElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_6()
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap_1()
                .child(div().text_sm().child(title))
                .child(div().text_xs().text_color(chrome.muted_fg).child(hint)),
        )
        .child(div().flex_shrink_0().child(controls))
        .into_any_element()
}

/// IDEA 风格的图标按钮：图标来自随程序打包的 expUI SVG，悬停时铺浅色底。
pub fn icon_button(id: impl Into<ElementId>, icon: impl Into<SharedString>, color: Hsla) -> Button {
    Button::new(id)
        .small()
        .ghost()
        .w(px(24.))
        .h(px(24.))
        .p_0()
        .child(svg().path(icon.into()).size(px(16.)).text_color(color))
}

/// 状态栏右端的一个部件，悬停时前景变正文色并给出轻微底色。
fn status_widget(text: impl Into<SharedString>, chrome: &Chrome) -> AnyElement {
    div()
        .h_full()
        .flex()
        .items_center()
        .px_2()
        .text_color(chrome.muted_fg)
        .hover(|style| style.bg(chrome.control_hover).text_color(chrome.fg))
        .child(text.into())
        .into_any_element()
}

pub fn empty_editor_state(chrome: &Chrome, root: Option<&Path>) -> AnyElement {
    let hint = match root {
        Some(root) => format!("在左侧项目树中双击文件开始编辑 · {}", root.display()),
        None => "先通过“打开项目目录”选择一个工作区".to_string(),
    };

    div()
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_2()
        .text_color(chrome.muted_fg)
        .child(div().text_lg().child("没有打开的文件"))
        .child(div().child(hint))
        .into_any_element()
}

/// 顶部工具栏和系统菜单共用的主菜单结构。
fn build_main_menu(menu: PopupMenu, window: &mut Window, cx: &mut Context<PopupMenu>) -> PopupMenu {
    menu.submenu("文件", window, cx, |menu, _, _| {
        menu.menu("打开项目目录…", Box::new(OpenProject))
            .separator()
            .menu("保存", Box::new(SaveFile))
            .menu("关闭编辑器", Box::new(CloseEditor))
            .separator()
            .menu("设置…", Box::new(ShowSettings))
    })
    .submenu("编辑", window, cx, |menu, _, _| {
        menu.menu("撤销", Box::new(Undo))
            .menu("重做", Box::new(Redo))
            .separator()
            .menu("剪切", Box::new(Cut))
            .menu("复制", Box::new(Copy))
            .menu("粘贴", Box::new(Paste))
            .separator()
            .menu("全选", Box::new(SelectAll))
            .separator()
            .menu("查找…", Box::new(FindInFile))
            .menu("替换…", Box::new(ReplaceInFile))
    })
    .submenu("视图", window, cx, |menu, _, _| {
        menu.menu("显示或隐藏项目面板", Box::new(ToggleProjectPanel))
            .menu("显示或隐藏底部面板", Box::new(ToggleBottomPanel))
            .menu("显示或隐藏导航栏面包屑", Box::new(ToggleBreadcrumbs))
            .separator()
            .menu("切换深色 / 浅色主题", Box::new(ToggleTheme))
            .separator()
            .menu("命令面板…", Box::new(ShowCommandPalette))
    })
    .submenu("导航", window, cx, |menu, _, _| {
        menu.menu("上一个标签", Box::new(PreviousTab))
            .menu("下一个标签", Box::new(NextTab))
            .separator()
            .menu("在项目中定位", Box::new(LocateInProject))
    })
    .submenu("工具", window, cx, |menu, _, _| {
        menu.menu("从磁盘重新加载项目", Box::new(RefreshProjectTree))
            .menu("折叠全部目录", Box::new(CollapseProjectTree))
    })
    .submenu("窗口", window, cx, |menu, _, _| {
        menu.menu("关闭其他标签", Box::new(CloseOtherTabs))
            .menu("关闭全部标签", Box::new(CloseAllTabs))
    })
    .submenu("帮助", window, cx, |menu, _, _| {
        menu.menu("关于 Keel…", Box::new(ShowAbout))
    })
}

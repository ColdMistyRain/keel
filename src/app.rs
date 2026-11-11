use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};

use gpui::{prelude::*, *};
use gpui_component::ActiveTheme;
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{EditorState, InputEvent, InputState, TabSize};
use gpui_component::menu::AppMenuBar;

use crate::actions::{
    CloseEditor, OpenProject, PaletteCancel, PaletteConfirm, PaletteDown, PaletteUp, SaveFile,
    ShowCommandPalette, ToggleBottomPanel, ToggleProjectPanel,
};
use crate::editor::Document;
use crate::model::{self, PersistedState, TreeNode, VisibleRow};
use crate::project::{self, ProjectModel};

const MAX_EDITABLE_FILE_SIZE: u64 = 8 * 1024 * 1024;

/// 从主题中取出的界面色板，避免在渲染过程中长期借用 `cx`。
#[derive(Clone)]
pub struct Chrome {
    pub bg: Hsla,
    pub fg: Hsla,
    pub border: Hsla,
    pub panel: Hsla,
    pub panel_fg: Hsla,
    pub muted_fg: Hsla,
    pub accent: Hsla,
    pub accent_fg: Hsla,
    pub header: Hsla,
    pub hover_bg: Hsla,
    pub tab_underline: Hsla,
    pub mono_font: SharedString,
    pub mono_size: Pixels,
}

impl Chrome {
    pub fn from_theme(cx: &Context<KeelApp>) -> Self {
        let theme = cx.theme();
        Self {
            bg: theme.background,
            fg: theme.foreground,
            border: theme.border,
            panel: theme.sidebar,
            panel_fg: theme.sidebar_foreground,
            muted_fg: theme.muted_foreground,
            accent: theme.primary,
            accent_fg: theme.primary_foreground,
            header: crate::idea_theme::color(crate::idea_theme::token::TOOL_WINDOW_HEADER),
            hover_bg: crate::idea_theme::color(crate::idea_theme::token::TREE_HOVER),
            tab_underline: crate::idea_theme::color(crate::idea_theme::token::TAB_UNDERLINE),
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
            BottomTab::Problems => "Problems",
            BottomTab::Run => "Run",
            BottomTab::Terminal => "Terminal",
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

    pub fn id(self) -> &'static str {
        match self {
            CommandId::OpenProject => "open-project",
            CommandId::SaveFile => "save-file",
            CommandId::CloseEditor => "close-editor",
            CommandId::ToggleProjectPanel => "toggle-project-panel",
            CommandId::ToggleBottomPanel => "toggle-bottom-panel",
        }
    }

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
}

pub struct KeelApp {
    pub menu_bar: Entity<AppMenuBar>,
    pub project: ProjectState,
    pub documents: Vec<Document>,
    pub active: Option<usize>,
    pub project_panel_visible: bool,
    pub bottom_panel_visible: bool,
    pub bottom_tab: BottomTab,
    pub status: StatusMessage,
    pub palette: Option<PaletteState>,
    pub pending_close: Option<PathBuf>,
    pending_focus: Option<FocusTarget>,
    focus_retries: u8,
    pub persisted: PersistedState,
    pub focus_handle: FocusHandle,
    pub tasks: Vec<Task<()>>,
}

impl KeelApp {
    pub fn new(window: &mut Window, persisted: PersistedState, cx: &mut Context<Self>) -> Self {
        let mut this = Self {
            menu_bar: AppMenuBar::new(cx),
            project: ProjectState::default(),
            documents: Vec::new(),
            active: None,
            project_panel_visible: persisted.project_panel_visible,
            bottom_panel_visible: persisted.bottom_panel_visible,
            bottom_tab: persisted
                .bottom_tab
                .as_deref()
                .map(BottomTab::from_key)
                .unwrap_or_default(),
            status: StatusMessage::info("就绪"),
            palette: None,
            pending_close: None,
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
        self.status = StatusMessage::info(format!("正在打开 {}", root.display()));
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
                        app.project.expanded.insert(root);
                        app.project.selected = None;
                        app.refresh_rows();
                        app.status = StatusMessage::info("项目已打开");
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
            self.request_focus(FocusTarget::Document(path));
            cx.notify();
            return;
        }

        self.status = StatusMessage::info(format!("正在打开 {}", short_name(&path)));
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
            language.unwrap_or("plain"),
            eol,
            subscription,
            observe,
        ));
        self.active = Some(self.documents.len() - 1);
        self.request_focus(FocusTarget::Document(path.clone()));
        self.status = StatusMessage::info(format!("已打开 {}", short_name(&path)));
    }

    pub fn activate_document(&mut self, idx: usize, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(doc) = self.documents.get(idx) else {
            return;
        };
        self.active = Some(idx);
        self.request_focus(FocusTarget::Document(doc.path.clone()));
        cx.notify();
    }

    pub fn save_document(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let Some(idx) = self.index_of(&path) else {
            return;
        };
        let text = self.documents[idx].state.read(cx).value().to_string();
        self.status = StatusMessage::info(format!("正在保存 {}", short_name(&path)));
        cx.notify();

        let task = cx.spawn(async move |this, cx| {
            let write_path = path.clone();
            let result = cx
                .background_spawn(async move { std::fs::write(&write_path, text) })
                .await;
            this.update(cx, |app, cx| {
                match result {
                    Ok(()) => {
                        if let Some(idx) = app.index_of(&path) {
                            app.documents[idx].dirty = false;
                        }
                        app.status = StatusMessage::info(format!("已保存 {}", short_name(&path)));
                    }
                    Err(err) => {
                        app.status = StatusMessage::error(format!("保存失败：{err}"));
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

    fn render_menu_bar(&self, chrome: &Chrome) -> AnyElement {
        div()
            .h(px(30.))
            .w_full()
            .flex_shrink_0()
            .bg(chrome.panel)
            .border_b_1()
            .border_color(chrome.border)
            .child(self.menu_bar.clone())
            .into_any_element()
    }

    fn render_toolbar(&self, chrome: &Chrome, cx: &mut Context<Self>) -> AnyElement {
        let mut buttons = Vec::new();
        for command in CommandId::ALL.iter().copied() {
            buttons.push(
                Button::new(command.id())
                    .label(command.label())
                    .ghost()
                    .on_click(cx.listener(move |this, _event, _window, cx| {
                        this.run_command(command, cx);
                    }))
                    .into_any_element(),
            );
        }

        div()
            .flex()
            .items_center()
            .gap_1()
            .h(px(40.))
            .px_2()
            .flex_shrink_0()
            .bg(chrome.panel)
            .border_b_1()
            .border_color(chrome.border)
            .child(
                div()
                    .px_3()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(chrome.panel_fg)
                    .child("Keel"),
            )
            .child(div().w(px(1.)).h(px(16.)).mx_1().bg(chrome.border))
            .children(buttons)
            .child(div().flex_1())
            .child(
                div()
                    .px_3()
                    .text_xs()
                    .text_color(chrome.muted_fg)
                    .child("Ctrl+Shift+P 命令面板"),
            )
            .into_any_element()
    }

    fn render_status_bar(&self, chrome: &Chrome, cx: &App) -> AnyElement {
        let left_color = if self.status.is_error {
            hsla(0.0, 0.65, 0.56, 1.0)
        } else {
            chrome.muted_fg
        };

        // 右侧按 IDEA 状态栏的顺序排：行列号、换行符、编码、缩进、文件类型。
        let mut widgets: Vec<AnyElement> = Vec::new();
        match self.active_document() {
            Some(doc) => {
                let position = doc.state.read(cx).cursor_position();
                widgets.push(status_widget(
                    format!("{}:{}", position.line + 1, position.character + 1),
                    chrome,
                ));
                widgets.push(status_widget(doc.eol, chrome));
                widgets.push(status_widget("UTF-8", chrome));
                widgets.push(status_widget("空格: 4", chrome));
                widgets.push(status_widget(
                    model::language_display_name(doc.language),
                    chrome,
                ));
            }
            None => widgets.push(status_widget(
                format!("{} 个标签页", self.documents.len()),
                chrome,
            )),
        }

        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_2()
            .h(px(24.))
            .flex_shrink_0()
            .text_xs()
            .bg(chrome.panel)
            .border_t_1()
            .border_color(chrome.border)
            .child(
                div()
                    .min_w_0()
                    .px_3()
                    .text_color(left_color)
                    .child(self.status.text.clone()),
            )
            .child(div().flex().items_center().h_full().children(widgets))
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
}

impl Render for KeelApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.apply_pending_focus(window, cx);
        let chrome = Chrome::from_theme(cx);

        let menu_bar = self.render_menu_bar(&chrome);
        let toolbar = self.render_toolbar(&chrome, cx);
        let status_bar = self.render_status_bar(&chrome, cx);

        let project_panel = if self.project_panel_visible {
            Some(self.render_project_panel(&chrome, cx))
        } else {
            None
        };

        let main_column = self.render_main_column(&chrome, cx);
        let stripe = self.render_tool_window_stripe(&chrome, cx);

        let mut body = div().flex_1().min_h_0().flex().flex_row().child(stripe);
        if let Some(panel) = project_panel {
            body = body.child(panel);
        }
        body = body.child(main_column);

        let mut root = div()
            .id("keel-root")
            .key_context("Keel")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_open_project))
            .on_action(cx.listener(Self::on_save_file))
            .on_action(cx.listener(Self::on_close_editor))
            .on_action(cx.listener(Self::on_toggle_project_panel))
            .on_action(cx.listener(Self::on_toggle_bottom_panel))
            .on_action(cx.listener(Self::on_show_palette))
            .on_action(cx.listener(Self::on_palette_confirm))
            .on_action(cx.listener(Self::on_palette_cancel))
            .on_action(cx.listener(Self::on_palette_up))
            .on_action(cx.listener(Self::on_palette_down))
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .bg(chrome.bg)
            .text_color(chrome.fg)
            .child(menu_bar)
            .child(toolbar)
            .child(body)
            .child(status_bar);

        if self.pending_close.is_some() {
            root = root.child(self.render_close_confirm(&chrome, cx));
        }

        if self.palette.is_some() {
            root = root.child(self.render_palette(&chrome, cx));
        }

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

fn dir_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

/// 状态栏右端的一个部件，悬停时给出 IDEA 里的浅色高亮。
fn status_widget(text: impl Into<SharedString>, chrome: &Chrome) -> AnyElement {
    div()
        .h_full()
        .flex()
        .items_center()
        .px_2()
        .text_color(chrome.muted_fg)
        .hover(|style| style.bg(chrome.hover_bg))
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

//! Keel 的动作系统，对应 IDEA 的 `ActionManager` / `AnAction` / `ActionGroup` /
//! `DataContext`：
//!
//! - **`Action`**：稳定 ID + 展示信息 + `update`（可用性/选中态）+ `perform`；
//! - **`ActionGroup`**：菜单/工具栏的层级结构，支持分隔符与子菜单；
//! - **`DataContext`**：动作执行时的上下文（当前编辑器、项目、工具窗口……），
//!   按类型取值，可带父上下文；
//! - **`ActionRegistry`**：注册表；菜单、快捷键、命令面板都从这里取同一份定义。
//!
//! 不依赖任何 UI 框架，GPUI 与 nsh 共用。

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

/// 动作的稳定 ID（IDEA 的 action id），菜单、快捷键、命令面板共用。
pub type ActionId = &'static str;

/// 动作的展示状态（IDEA 的 `Presentation`）。
///
/// `update` 先填入默认值，再由具体动作按 `DataContext` 调整可用性和选中态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Presentation {
    pub text: String,
    pub description: String,
    pub shortcut: Option<String>,
    pub enabled: bool,
    pub visible: bool,
    /// `Some` 时是可选中项（菜单显示勾选），例如外观、工具窗口开关。
    pub checked: Option<bool>,
}

impl Presentation {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            description: String::new(),
            shortcut: None,
            enabled: true,
            visible: true,
            checked: None,
        }
    }
}

/// 动作执行时的上下文（IDEA 的 `DataContext`）。
///
/// 值按类型存放，查找会沿父上下文向上；UI 层把自己的状态（打开的文档、
/// 当前工具窗口等）放进来，动作只依赖这里，不直接读全局变量。
#[derive(Clone, Default)]
pub struct DataContext {
    parent: Option<Arc<DataContext>>,
    values: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
}

impl DataContext {
    pub fn new() -> Self {
        Self::default()
    }

    /// 以上下文 `parent` 为父：本级找不到的值向上找。
    pub fn with_parent(parent: Arc<DataContext>) -> Self {
        Self {
            parent: Some(parent),
            values: HashMap::new(),
        }
    }

    /// 放入一个值，返回自身以便链式构建。
    pub fn put<T: Any + Send + Sync>(&mut self, value: T) -> &mut Self {
        self.values.insert(TypeId::of::<T>(), Arc::new(value));
        self
    }

    /// 链式版本的 [`Self::put`]。
    pub fn with<T: Any + Send + Sync>(mut self, value: T) -> Self {
        self.put(value);
        self
    }

    /// 取一个值；本级没有时向父上下文找。
    pub fn get<T: Any + Send + Sync>(&self) -> Option<&T> {
        if let Some(value) = self.values.get(&TypeId::of::<T>()) {
            return value.downcast_ref::<T>();
        }
        self.parent.as_ref().and_then(|parent| parent.get::<T>())
    }
}

/// 一个动作（IDEA 的 `AnAction`）。
pub trait Action: Send + Sync {
    fn id(&self) -> ActionId;
    fn text(&self) -> &str;
    fn description(&self) -> &str {
        ""
    }
    fn shortcut(&self) -> Option<&str> {
        None
    }
    /// 计算展示状态：默认只有文本；可用性、选中态由动作自己决定。
    fn update(&self, _context: &DataContext, presentation: &mut Presentation) {
        presentation.text = self.text().to_owned();
    }
    fn perform(&self, _context: &DataContext);
}

type Predicate = Box<dyn Fn(&DataContext) -> bool + Send + Sync>;
type Handler = Box<dyn Fn(&DataContext) + Send + Sync>;

/// 用闭包定义的动作：spike 和 GPUI 版本都不必为每个菜单项建一个类型。
pub struct CallbackAction {
    id: ActionId,
    text: String,
    description: String,
    shortcut: Option<String>,
    enabled: Option<Predicate>,
    checked: Option<Predicate>,
    handler: Handler,
}

impl CallbackAction {
    pub fn new(
        id: ActionId,
        text: impl Into<String>,
        handler: impl Fn(&DataContext) + Send + Sync + 'static,
    ) -> Self {
        Self {
            id,
            text: text.into(),
            description: String::new(),
            shortcut: None,
            enabled: None,
            checked: None,
            handler: Box::new(handler),
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// 可用性谓词：返回 `false` 时菜单项变灰、快捷键不执行。
    pub fn enabled(
        mut self,
        predicate: impl Fn(&DataContext) -> bool + Send + Sync + 'static,
    ) -> Self {
        self.enabled = Some(Box::new(predicate));
        self
    }

    /// 选中态谓词：`Some` 时菜单项带勾选。
    pub fn checked(
        mut self,
        predicate: impl Fn(&DataContext) -> bool + Send + Sync + 'static,
    ) -> Self {
        self.checked = Some(Box::new(predicate));
        self
    }
}

impl Action for CallbackAction {
    fn id(&self) -> ActionId {
        self.id
    }

    fn text(&self) -> &str {
        &self.text
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn shortcut(&self) -> Option<&str> {
        self.shortcut.as_deref()
    }

    fn update(&self, context: &DataContext, presentation: &mut Presentation) {
        presentation.text = self.text.clone();
        presentation.description = self.description.clone();
        presentation.shortcut = self.shortcut.clone();
        presentation.enabled = self
            .enabled
            .as_ref()
            .is_none_or(|predicate| predicate(context));
        presentation.checked = self.checked.as_ref().map(|predicate| predicate(context));
    }

    fn perform(&self, context: &DataContext) {
        (self.handler)(context);
    }
}

/// 动作组里的一项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupEntry {
    Action(ActionId),
    Separator,
    Group(ActionGroup),
}

/// 一组动作（IDEA 的 `ActionGroup`）：菜单栏的一项、子菜单或工具栏。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionGroup {
    id: &'static str,
    text: &'static str,
    entries: Vec<GroupEntry>,
}

impl ActionGroup {
    pub fn new(id: &'static str, text: &'static str) -> Self {
        Self {
            id,
            text,
            entries: Vec::new(),
        }
    }

    pub fn add_action(mut self, id: ActionId) -> Self {
        self.entries.push(GroupEntry::Action(id));
        self
    }

    pub fn add_separator(mut self) -> Self {
        self.entries.push(GroupEntry::Separator);
        self
    }

    pub fn add_group(mut self, group: ActionGroup) -> Self {
        self.entries.push(GroupEntry::Group(group));
        self
    }

    pub fn id(&self) -> &'static str {
        self.id
    }

    pub fn text(&self) -> &'static str {
        self.text
    }

    pub fn entries(&self) -> &[GroupEntry] {
        &self.entries
    }
}

/// 解析后的菜单节点：UI 直接按它渲染，不再重复计算可用性。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuNode {
    Action {
        id: ActionId,
        presentation: Presentation,
    },
    Separator,
    Submenu {
        id: &'static str,
        text: String,
        entries: Vec<MenuNode>,
    },
}

/// 动作注册表（IDEA 的 `ActionManager`）。
#[derive(Default)]
pub struct ActionRegistry {
    actions: HashMap<ActionId, Arc<dyn Action>>,
    groups: HashMap<&'static str, ActionGroup>,
    /// 注册顺序，菜单栏按它排列。
    order: Vec<&'static str>,
}

impl ActionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册动作；同一 ID 再次注册会替换旧定义。
    pub fn register<A: Action + 'static>(&mut self, action: A) -> &mut Self {
        self.actions.insert(action.id(), Arc::new(action));
        self
    }

    /// 注册顶层动作组（菜单栏的一项）。
    pub fn register_group(&mut self, group: ActionGroup) -> &mut Self {
        let id = group.id();
        if !self.groups.contains_key(&id) {
            self.order.push(id);
        }
        self.groups.insert(id, group);
        self
    }

    pub fn action(&self, id: ActionId) -> Option<Arc<dyn Action>> {
        self.actions.get(id).cloned()
    }

    /// 顶层动作组，按注册顺序。
    pub fn groups(&self) -> Vec<&ActionGroup> {
        self.order
            .iter()
            .filter_map(|id| self.groups.get(id))
            .collect()
    }

    /// 所有动作此刻的展示状态，按文本排序（命令面板用）。
    pub fn presentations(&self, context: &DataContext) -> Vec<(ActionId, Presentation)> {
        let mut all: Vec<(ActionId, Presentation)> = self
            .actions
            .keys()
            .filter_map(|id| {
                self.presentation(id, context)
                    .map(|presentation| (*id, presentation))
            })
            .collect();
        all.sort_by(|a, b| a.1.text.cmp(&b.1.text));
        all
    }

    /// 计算动作此刻的展示状态。
    pub fn presentation(&self, id: ActionId, context: &DataContext) -> Option<Presentation> {
        let action = self.actions.get(id)?;
        let mut presentation = Presentation::new(action.text());
        presentation.description = action.description().to_owned();
        presentation.shortcut = action.shortcut().map(str::to_owned);
        action.update(context, &mut presentation);
        Some(presentation)
    }

    /// 执行动作；未知、隐藏或不可用时返回 `false`。
    pub fn perform(&self, id: ActionId, context: &DataContext) -> bool {
        let Some(action) = self.actions.get(id) else {
            return false;
        };
        let Some(presentation) = self.presentation(id, context) else {
            return false;
        };
        if !presentation.enabled || !presentation.visible {
            return false;
        }
        action.perform(context);
        true
    }

    /// 把动作组解析成菜单节点：隐藏的动作/空子菜单会被跳过，分隔符合并。
    pub fn menu(&self, group_id: ActionId, context: &DataContext) -> Vec<MenuNode> {
        self.groups
            .get(group_id)
            .map(|group| self.resolve(group, context))
            .unwrap_or_default()
    }

    /// 所有定义了快捷键的动作，按动作 ID 排序。
    pub fn shortcuts(&self) -> Vec<(String, ActionId)> {
        let mut shortcuts: Vec<(String, ActionId)> = self
            .actions
            .values()
            .filter_map(|action| {
                action
                    .shortcut()
                    .map(|shortcut| (shortcut.to_owned(), action.id()))
            })
            .collect();
        shortcuts.sort_by_key(|(_, id)| *id);
        shortcuts
    }

    fn resolve(&self, group: &ActionGroup, context: &DataContext) -> Vec<MenuNode> {
        let mut nodes = Vec::new();
        for entry in &group.entries {
            match entry {
                GroupEntry::Action(id) => {
                    if let Some(presentation) = self.presentation(id, context)
                        && presentation.visible
                    {
                        nodes.push(MenuNode::Action { id, presentation });
                    }
                }
                GroupEntry::Separator => {
                    if !nodes.is_empty() && !matches!(nodes.last(), Some(MenuNode::Separator)) {
                        nodes.push(MenuNode::Separator);
                    }
                }
                GroupEntry::Group(submenu) => {
                    let entries = self.resolve(submenu, context);
                    if entries
                        .iter()
                        .any(|node| !matches!(node, MenuNode::Separator))
                    {
                        nodes.push(MenuNode::Submenu {
                            id: submenu.id,
                            text: submenu.text.to_owned(),
                            entries,
                        });
                    }
                }
            }
        }
        while matches!(nodes.last(), Some(MenuNode::Separator)) {
            nodes.pop();
        }
        nodes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Clone, Copy, PartialEq, Eq)]
    struct Cursor {
        line: u32,
    }

    fn counter() -> Arc<AtomicUsize> {
        Arc::new(AtomicUsize::new(0))
    }

    #[test]
    fn registers_actions_and_reports_their_presentation() {
        let mut registry = ActionRegistry::new();
        registry.register(
            CallbackAction::new("file.save", "保存", |_| {})
                .shortcut("mod+s")
                .description("保存当前文件"),
        );

        let presentation = registry
            .presentation("file.save", &DataContext::new())
            .unwrap();
        assert_eq!(presentation.text, "保存");
        assert_eq!(presentation.description, "保存当前文件");
        assert_eq!(presentation.shortcut.as_deref(), Some("mod+s"));
        assert!(presentation.enabled);
        assert!(presentation.visible);
        assert_eq!(presentation.checked, None);
        assert!(
            registry
                .presentation("missing", &DataContext::new())
                .is_none()
        );
    }

    #[test]
    fn context_values_follow_the_parent() {
        let parent = DataContext::new().with(Cursor { line: 7 });
        let child = DataContext::with_parent(Arc::new(parent)).with("project".to_owned());
        assert_eq!(child.get::<Cursor>().map(|cursor| cursor.line), Some(7));
        assert_eq!(child.get::<String>().map(String::as_str), Some("project"));
        assert!(DataContext::new().get::<Cursor>().is_none());
    }

    #[test]
    fn enabled_and_checked_predicates_see_the_context() {
        let mut registry = ActionRegistry::new();
        registry.register(
            CallbackAction::new("edit.save", "保存", |_| {})
                .enabled(|ctx| ctx.get::<Cursor>().is_some_and(|cursor| cursor.line > 0))
                .checked(|ctx| ctx.get::<Cursor>().is_some_and(|cursor| cursor.line == 7)),
        );

        let empty = DataContext::new();
        let presentation = registry.presentation("edit.save", &empty).unwrap();
        assert!(!presentation.enabled);
        assert_eq!(presentation.checked, Some(false));

        let ready = DataContext::new().with(Cursor { line: 7 });
        let presentation = registry.presentation("edit.save", &ready).unwrap();
        assert!(presentation.enabled);
        assert_eq!(presentation.checked, Some(true));
    }

    #[test]
    fn menu_resolves_groups_separators_and_submenus() {
        let mut registry = ActionRegistry::new();
        registry.register(CallbackAction::new("file.save", "保存", |_| {}));
        registry.register(CallbackAction::new("file.close", "关闭", |_| {}));
        registry.register(CallbackAction::new("view.light", "浅色", |_| {}).checked(|_| true));
        registry.register_group(
            ActionGroup::new("menu.file", "文件")
                .add_separator()
                .add_action("file.save")
                .add_separator()
                .add_separator()
                .add_action("file.close")
                .add_separator()
                .add_group(
                    ActionGroup::new("menu.file.appearance", "外观").add_action("view.light"),
                ),
        );

        let nodes = registry.menu("menu.file", &DataContext::new());
        assert_eq!(nodes.len(), 5);
        assert!(matches!(
            &nodes[0],
            MenuNode::Action {
                id: "file.save",
                ..
            }
        ));
        assert_eq!(nodes[1], MenuNode::Separator);
        assert!(matches!(
            &nodes[2],
            MenuNode::Action {
                id: "file.close",
                ..
            }
        ));
        assert_eq!(nodes[3], MenuNode::Separator);
        match &nodes[4] {
            MenuNode::Submenu { text, entries, .. } => {
                assert_eq!(text, "外观");
                assert_eq!(entries.len(), 1);
            }
            other => panic!("expected submenu, got {other:?}"),
        }
    }

    #[test]
    fn perform_dispatches_and_respects_enabled() {
        struct Hidden;

        impl Action for Hidden {
            fn id(&self) -> ActionId {
                "test.hidden"
            }
            fn text(&self) -> &str {
                "隐藏"
            }
            fn update(&self, _: &DataContext, presentation: &mut Presentation) {
                presentation.visible = false;
            }
            fn perform(&self, _: &DataContext) {}
        }

        let hits = counter();
        let mut registry = ActionRegistry::new();
        let counted = hits.clone();
        registry.register(CallbackAction::new("test.hit", "执行", move |_| {
            counted.fetch_add(1, Ordering::SeqCst);
        }));
        registry.register(
            CallbackAction::new("test.off", "禁用", |_| {})
                .enabled(|ctx| ctx.get::<Cursor>().is_some()),
        );
        registry.register(Hidden);

        assert!(registry.perform("test.hit", &DataContext::new()));
        assert_eq!(hits.load(Ordering::SeqCst), 1);
        assert!(!registry.perform("test.off", &DataContext::new()));
        assert!(!registry.perform("test.hidden", &DataContext::new()));
        assert!(!registry.perform("test.missing", &DataContext::new()));
        assert_eq!(hits.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn presentations_are_listed_in_label_order() {
        let mut registry = ActionRegistry::new();
        registry.register(CallbackAction::new("b.second", "保存", |_| {}));
        registry.register(CallbackAction::new("a.first", "关闭", |_| {}));
        let all = registry.presentations(&DataContext::new());
        let labels: Vec<_> = all.iter().map(|(_, item)| item.text.as_str()).collect();
        assert_eq!(labels, vec!["保存", "关闭"]);
        assert_eq!(all[0].0, "b.second");
    }

    #[test]
    fn shortcuts_and_groups_are_listed_in_order() {
        let mut registry = ActionRegistry::new();
        registry.register(CallbackAction::new("b.second", "B", |_| {}).shortcut("mod+w"));
        registry.register(CallbackAction::new("a.first", "A", |_| {}).shortcut("mod+o"));
        registry.register(CallbackAction::new("c.none", "C", |_| {}));
        registry.register_group(ActionGroup::new("menu.one", "一"));
        registry.register_group(ActionGroup::new("menu.two", "二"));

        assert_eq!(
            registry.shortcuts(),
            vec![
                ("mod+o".to_owned(), "a.first"),
                ("mod+w".to_owned(), "b.second")
            ]
        );
        let ids: Vec<_> = registry.groups().iter().map(|group| group.id()).collect();
        assert_eq!(ids, vec!["menu.one", "menu.two"]);
    }
}

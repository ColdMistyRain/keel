//! IDEA 风格的下拉菜单外壳。
//!
//! gpui-component 的弹层用「10% 前景色的阴影环」充当边框（shadcn 的做法，
//! 见组件库 `styled.rs::popover_style`），而 IDEA 的弹层是 1px 实线
//! （`Popup.borderColor` = `popup-border-windows`，浅色 gray-130、深色 gray-40）。
//! 组件库没有提供描边样式的接口，所以这里在菜单表面内侧再叠一层 1px 描边；
//! 定位、子菜单、键盘导航等行为仍然交给组件库的 `PopupMenu`。

use std::rc::Rc;

use gpui::{prelude::*, *};
use gpui_component::menu::PopupMenu;
use gpui_component::popover::Popover;
use gpui_component::{ActiveTheme as _, Selectable};

/// 菜单表面的圆角，对应 `PopupMenu.borderCornerRadius` = 8（Jewel 桥接把它
/// 当作实际圆角半径使用，不是弧直径）。组件库的 `popover_style` 用
/// `theme.radius`（控件半径 4），所以这里单独取 `radius_lg`，避免菜单表面
/// 跟着按钮一起变成 4。
const MENU_RADIUS: Pixels = px(8.);

/// 菜单表面的圆角（下拉菜单与右键菜单共用）。
pub fn menu_radius(cx: &App) -> Pixels {
    cx.theme().radius_lg.min(MENU_RADIUS)
}

/// 菜单表面 + IDEA 的 1px 描边。
///
/// 组件库的 `popover_style` 只有阴影环，这里在菜单表面内侧补一层实线，
/// 描边颜色取 `Popup.borderColor`。外层同时铺一层 `popover` 底色：组件库
/// 内部仍按 `theme.radius`(4) 绘制菜单背景，外层 8px 圆角与内层 4px 之间的
/// 空隙由这层底色补齐，菜单表面因此呈现 IDEA 的 8px 圆角。
pub fn bordered_surface(
    menu: Entity<PopupMenu>,
    border: Hsla,
    radius: Pixels,
    background: Hsla,
) -> AnyElement {
    div()
        .relative()
        .rounded(radius)
        .bg(background)
        .child(menu)
        // 盖在菜单背景之上画 1px 实线，模拟 IDEA 的 `Popup.borderColor`。
        .child(
            div()
                .absolute()
                .inset_0()
                .rounded(radius)
                .border_1()
                .border_color(border),
        )
        .into_any_element()
}

/// 缓存的菜单实体，避免每次重绘都重建菜单项。
#[derive(Default)]
struct MenuSlot {
    menu: Option<Entity<PopupMenu>>,
}

/// 构建菜单项的回调，与组件库 `dropdown_menu` 的参数一致。
type MenuBuilder = dyn Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu;

/// 带 IDEA 1px 描边的下拉菜单，用法与组件库的 `dropdown_menu` 相同。
pub fn dropdown_menu<T, F>(
    id: impl Into<ElementId>,
    border: Hsla,
    trigger: T,
    build: F,
) -> impl IntoElement
where
    T: Selectable + IntoElement + 'static,
    F: Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
{
    let id = id.into();
    let slot_id = id.clone();
    let builder: Rc<MenuBuilder> = Rc::new(build);

    Popover::new(id)
        .appearance(false)
        .overlay_closable(false)
        .trigger(trigger)
        .content(move |_, window, cx| {
            let slot = window.use_keyed_state(slot_id.clone(), cx, |_, _| MenuSlot::default());
            let menu = match slot.read(cx).menu.clone() {
                Some(menu) => menu,
                None => {
                    let builder = builder.clone();
                    let menu = PopupMenu::build(window, cx, move |menu, window, cx| {
                        builder(menu, window, cx)
                    });
                    slot.update(cx, |slot, _| slot.menu = Some(menu.clone()));
                    menu.focus_handle(cx).focus(window, cx);

                    // 菜单自行关闭时同步关掉外层弹层，并清空缓存，
                    // 让下次打开时重新构建菜单项。
                    let popover = cx.entity().downgrade();
                    let slot = slot.downgrade();
                    window
                        .subscribe(&menu, cx, move |_, _: &DismissEvent, window, cx| {
                            if let Some(popover) = popover.upgrade() {
                                popover.update(cx, |state, cx| state.dismiss(window, cx));
                            }
                            _ = slot.update(cx, |slot, _| slot.menu = None);
                        })
                        .detach();

                    menu
                }
            };

            bordered_surface(menu, border, menu_radius(cx), cx.theme().popover)
        })
}

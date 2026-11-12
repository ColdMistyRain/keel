//! Keel 的基础机制，对应 IDEA 平台的三件事：
//!
//! - **Service Container**：应用级 / 项目级服务容器，按需创建、可释放；
//! - **Disposable 树**：项目关闭、窗口关闭时统一清理；
//! - **MessageBus / Topic**：应用级、项目级发布订阅。
//!
//! 这里不依赖任何 UI 框架；GPUI 与 nsh 共用。

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, Weak};

/// 一个可释放的对象树。
#[derive(Default)]
pub struct Disposer {
    actions: Mutex<Vec<Box<dyn FnOnce() + Send>>>,
}

impl Disposer {
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册清理动作；释放时按注册的逆序执行。
    pub fn add(&self, action: impl FnOnce() + Send + 'static) {
        self.actions.lock().unwrap().push(Box::new(action));
    }

    /// 释放：逆序执行全部清理动作，之后注册的也立即执行。
    pub fn dispose(&self) {
        let actions = std::mem::take(&mut *self.actions.lock().unwrap());
        for action in actions.into_iter().rev() {
            action();
        }
    }
}

/// 服务容器。IDEA 的 `Application` / `Project` 各持有一个。
///
/// 服务是 `Arc<T>`，第一次 `service` 时创建；`put` 用来注入测试替身或外部资源。
#[derive(Default)]
pub struct ServiceContainer {
    services: Mutex<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
    disposer: Disposer,
}

impl ServiceContainer {
    pub fn new() -> Self {
        Self::default()
    }

    /// 放入一个已经创建好的服务。
    pub fn put<T: Any + Send + Sync>(&self, service: T) -> Arc<T> {
        let service = Arc::new(service);
        self.services
            .lock()
            .unwrap()
            .insert(TypeId::of::<T>(), service.clone());
        service
    }

    /// 取服务；没有时返回 `None`。
    pub fn get<T: Any + Send + Sync>(&self) -> Option<Arc<T>> {
        self.services
            .lock()
            .unwrap()
            .get(&TypeId::of::<T>())
            .cloned()
            .map(|service| service.downcast::<T>().expect("服务类型与注册时一致"))
    }

    /// 取服务，没有时用 `init` 创建并缓存（IDEA 的懒加载服务）。
    pub fn service<T: Any + Send + Sync>(&self, init: impl FnOnce(&Self) -> T) -> Arc<T> {
        if let Some(service) = self.get::<T>() {
            return service;
        }
        self.put(init(self))
    }

    /// 注册一个清理动作，随容器释放。
    pub fn on_dispose(&self, action: impl FnOnce() + Send + 'static) {
        self.disposer.add(action);
    }

    pub fn dispose(&self) {
        self.disposer.dispose();
        self.services.lock().unwrap().clear();
    }
}

/// 应用级服务容器（IDEA 的 `Application`）。
pub type AppServices = ServiceContainer;
/// 项目级服务容器（IDEA 的 `Project`）。
pub type ProjectServices = ServiceContainer;

/// 一个类型化的事件主题（IDEA 的 `Topic`）。
pub struct Topic<T> {
    name: &'static str,
    _event: PhantomData<fn(&T)>,
}

impl<T> Topic<T> {
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            _event: PhantomData,
        }
    }

    pub fn name(&self) -> &'static str {
        self.name
    }
}

type Listener = Arc<dyn Fn(&dyn Any) + Send + Sync>;

#[derive(Default)]
struct BusState {
    next: AtomicU64,
    listeners: HashMap<(&'static str, TypeId), Vec<(u64, Listener)>>,
}

/// 发布订阅总线。应用级与项目级各一个实例。
#[derive(Default)]
pub struct MessageBus {
    state: Mutex<BusState>,
}

impl MessageBus {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// 订阅主题；`Subscription` 释放时自动退订。
    pub fn subscribe<T: Any + Send + Sync>(
        self: &Arc<Self>,
        topic: &Topic<T>,
        listener: impl Fn(&T) + Send + Sync + 'static,
    ) -> Subscription {
        let key = (topic.name, TypeId::of::<T>());
        let listener: Listener = Arc::new(move |event: &dyn Any| {
            if let Some(event) = event.downcast_ref::<T>() {
                listener(event);
            }
        });
        let id = {
            let mut state = self.state.lock().unwrap();
            let id = state.next.fetch_add(1, Ordering::Relaxed);
            state.listeners.entry(key).or_default().push((id, listener));
            id
        };
        Subscription {
            bus: Arc::downgrade(self),
            key,
            id,
        }
    }

    /// 发布事件：同步通知当前订阅者。
    pub fn publish<T: Any + Send + Sync>(&self, topic: &Topic<T>, event: &T) {
        let key = (topic.name, TypeId::of::<T>());
        let listeners = {
            let state = self.state.lock().unwrap();
            state
                .listeners
                .get(&key)
                .map(|listeners| listeners.iter().map(|(_, l)| l.clone()).collect::<Vec<_>>())
                .unwrap_or_default()
        };
        for listener in listeners {
            listener(event);
        }
    }
}

/// 订阅句柄：释放时退订。
pub struct Subscription {
    bus: Weak<MessageBus>,
    key: (&'static str, TypeId),
    id: u64,
}

impl Drop for Subscription {
    fn drop(&mut self) {
        if let Some(bus) = self.bus.upgrade() {
            let mut state = bus.state.lock().unwrap();
            if let Some(listeners) = state.listeners.get_mut(&self.key) {
                listeners.retain(|(id, _)| *id != self.id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn services_are_lazy_and_cached() {
        let services = AppServices::new();
        let created = AtomicUsize::new(0);
        let first = services.service(|_| {
            created.fetch_add(1, Ordering::Relaxed);
            42_u32
        });
        let second = services.service(|_| {
            created.fetch_add(1, Ordering::Relaxed);
            7_u32
        });
        assert_eq!(*first, 42);
        assert_eq!(*second, 42);
        assert_eq!(created.load(Ordering::Relaxed), 1);
        assert!(services.get::<u32>().is_some());
        assert!(services.get::<String>().is_none());
    }

    #[test]
    fn disposer_runs_in_reverse_order() {
        let disposer = Disposer::new();
        let order = Arc::new(Mutex::new(Vec::new()));
        for value in [1, 2, 3] {
            let order = order.clone();
            disposer.add(move || order.lock().unwrap().push(value));
        }
        disposer.dispose();
        assert_eq!(*order.lock().unwrap(), vec![3, 2, 1]);
    }

    #[test]
    fn message_bus_publishes_typed_events() {
        struct Changed(u32);
        static CHANGED: Topic<Changed> = Topic::new("keel.test.changed");

        let bus = MessageBus::new();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let first = {
            let seen = seen.clone();
            bus.subscribe(&CHANGED, move |event| seen.lock().unwrap().push(event.0))
        };
        let second = {
            let seen = seen.clone();
            bus.subscribe(&CHANGED, move |event| {
                seen.lock().unwrap().push(event.0 * 10)
            })
        };

        bus.publish(&CHANGED, &Changed(1));
        assert_eq!(*seen.lock().unwrap(), vec![1, 10]);

        drop(first);
        bus.publish(&CHANGED, &Changed(2));
        assert_eq!(*seen.lock().unwrap(), vec![1, 10, 20]);
        drop(second);
    }
}

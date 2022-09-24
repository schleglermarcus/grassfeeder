use crate::buildconfig::BuildConfigContainer;
use crate::Buildable;
use crate::StartupWithAppContext;
use std::any::Any;
use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Default)]
pub struct AppContext {
    typmap: HashMap<TypeId, Box<dyn Any>>,
    startups: Vec<Rc<RefCell<dyn StartupWithAppContext>>>,
    /// delivered by startup
    conf_system: Rc<RefCell<HashMap<String, String>>>,
    /// resides in configmanager
    conf_user: Rc<RefCell<HashMap<String, String>>>,
}


impl AppContext {
    pub fn set<T: Any + 'static>(&mut self, t: T) {
        self.typmap.insert(TypeId::of::<T>(), Box::new(t));
    }

    pub fn has<T: 'static + Any>(&self) -> bool {
        self.typmap.contains_key(&TypeId::of::<T>())
    }

    pub fn get_rc<T: 'static + Any>(&self) -> Option<Rc<RefCell<T>>> {
        self.typmap
            .get(&TypeId::of::<Rc<RefCell<T>>>())
            .map(|t| t.downcast_ref::<Rc<RefCell<T>>>().unwrap().clone())
    }

    pub fn get<T: 'static + Any>(&self) -> Option<&T> {
        self.typmap
            .get(&TypeId::of::<T>())
            .map(|t| t.downcast_ref::<T>().unwrap())
    }

    pub fn get_mut<T: 'static + Any>(&mut self) -> Option<&mut T> {
        self.typmap
            .get_mut(&TypeId::of::<T>())
            .map(|t| t.downcast_mut::<T>().unwrap())
    }

    pub fn build<T>(&mut self)
    where
        T: Buildable + 'static,
        <T as Buildable>::Output: StartupWithAppContext,
    {
        let mut prop_copy: HashMap<String, String> = (*self.conf_system).borrow().clone();
        {
            let conf_user = (*self.conf_user).borrow();
            if !conf_user.is_empty() {
                prop_copy.extend(conf_user.clone());
            }
        }
        let b_co = T::build(Box::new(BuildConfigContainer { map: prop_copy }), self);
        let rc_app = Rc::new(RefCell::new(b_co));
        self.set(rc_app.clone());
        self.startups.push(rc_app);
    }

    pub fn store_obj<T>(&mut self, obj: Rc<RefCell<T>>)
    where
        T: Buildable + 'static + StartupWithAppContext,
    {
        self.set(obj.clone());
        self.startups.push(obj);
    }

    pub fn startup(&mut self) {
        self.startups.iter().for_each(|s| {
            (*s).borrow_mut().startup(self);
        });
        self.startups.clear(); // reduce cyclic dependencies
    }

    pub fn new(sys_conf: HashMap<String, String>) -> AppContext {
        AppContext {
            conf_system: Rc::new(RefCell::new(sys_conf)),
            conf_user: Rc::new(RefCell::new(HashMap::new())),
            ..Default::default()
        }
    }

    pub fn set_user_conf(&mut self, u_c: Rc<RefCell<HashMap<String, String>>>) {
        self.conf_user = u_c;
    }

    pub fn get_system_config(&self) -> Rc<RefCell<HashMap<String, String>>> {
        self.conf_system.clone()
    }
}

#[cfg(test)]
pub mod appcontext_test {
    use super::*;
    use crate::BuildConfig;
    use std::sync::atomic::AtomicBool;

    pub static IN_USE: AtomicBool = AtomicBool::new(false);

    struct CoA {}
    impl Buildable for CoA {
        type Output = CoA;
        fn build(_conf: Box<dyn BuildConfig>, _ac: &AppContext) -> Self::Output {
            CoA {}
        }
    }
    impl StartupWithAppContext for CoA {}

    struct CoB {}
    impl Buildable for CoB {
        type Output = CoB;
        fn build(_conf: Box<dyn BuildConfig>, _ac: &AppContext) -> Self::Output {
            CoB {}
        }
    }
    impl StartupWithAppContext for CoB {}

    #[test]
    fn appcontext_drop() {
        {
            let mut appcontext = AppContext::default();
            appcontext.build::<CoA>();
            appcontext.build::<CoB>();
        }
        let in_use = IN_USE.load(core::sync::atomic::Ordering::Relaxed);
        assert!(!in_use);
    }
}

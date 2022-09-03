use crate::buildconfig::BuildConfigContainer;
use crate::Buildable;
use crate::StartupWithAppContext;
use ini::Ini;
use std::any::Any;
use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Default)]
pub struct AppContext {
    typmap: HashMap<TypeId, Box<dyn Any>>,
    ini_r: Rc<RefCell<Ini>>,
    startups: Vec<Rc<RefCell<dyn StartupWithAppContext>>>,
}

#[allow(dead_code)]
/// Drop   and Copy are exclusive
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
        let mut sectionmap: HashMap<String, String> = HashMap::new();
        if let Some(s) = (*self.ini_r).borrow().section(Some(T::section_name())) {
            sectionmap = s
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
        }
        let b_co = T::build(Box::new(BuildConfigContainer { map: sectionmap }), self);
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
        (*self.ini_r).borrow_mut().clear(); // we don't need the ini information no more
    }

    pub fn store_ini(&mut self, r_ini: Rc<RefCell<Ini>>) {
        self.ini_r = r_ini;
    }

    pub fn get_ini(&self) -> Rc<RefCell<Ini>> {
        self.ini_r.clone()
    }

    pub fn new_with_ini(ini_rc: Rc<RefCell<Ini>>) -> AppContext {
        AppContext {
            ini_r: ini_rc,
            ..Default::default()
        }
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
        fn section_name() -> String {
            "A".to_string()
        }
    }
    impl StartupWithAppContext for CoA {}

    struct CoB {}
    impl Buildable for CoB {
        type Output = CoB;
        fn build(_conf: Box<dyn BuildConfig>, _ac: &AppContext) -> Self::Output {
            CoB {}
        }
        fn section_name() -> String {
            "B".to_string()
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

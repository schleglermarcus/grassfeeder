use gui_layer::abstract_ui::AValue;
use std::collections::HashMap;

pub type DiaDistContent<'a> = &'a Vec<AValue>;
pub type DDDMap = HashMap<u8, Box<dyn Fn(DiaDistContent)>>;

#[derive(Default)]
pub struct DialogDataDistributor {
    // dist_map: HashMap<u8, Box<dyn Fn(DiaDistContent)>>,
    dist_map: DDDMap,
}

impl DialogDataDistributor {
    pub fn dialog_distribute(&self, idx: u8, dialog_data: &Vec<AValue>) {
        if let Some(distf) = self.dist_map.get(&idx) {
            (distf)(dialog_data);
        }
    }

    pub fn set_dialog_distribute(&mut self, idx: u8, func: impl Fn(DiaDistContent) + 'static) {
        self.dist_map.insert(idx, Box::new(func));
    }
}

#[cfg(test)]
mod ddd {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    pub fn test1() {
        let mut ddd = DialogDataDistributor::default();
        let x: Rc<RefCell<u32>> = Rc::new(RefCell::new(0));
        let x2 = x.clone();
        ddd.set_dialog_distribute(0, move |dd| {
            let v: u32 = dd.get(0).unwrap().uint().unwrap().clone();
            (*x2).replace(v);
        });

        let mut va = Vec::<AValue>::default();
        va.push(AValue::AU32(4));
        ddd.dialog_distribute(0, &va);
        assert_eq!(*(*x).borrow(), 4);
    }
}

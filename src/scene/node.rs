use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use hashbrown::HashMap;

use crate::core::name::StringName;

#[derive(Debug)]
pub struct Node {
    name: StringName,

    children: Vec<Rc<RefCell<Node>>>,

    /// child name to map lookup
    children_name_lookup: HashMap<StringName, u32>,

    parent: Weak<RefCell<Node>>,

    self_ptr: Weak<RefCell<Node>>,
}

/// Naming error
#[derive(thiserror::Error, Debug)]
pub enum RenameError {
    #[error("Cannot name a node to the same name as a sibling (index: {sibling_index})")]
    DuplicateName { sibling_index: u32 },

    #[error("Cannot rename child while its parent has been borrowed elsewhere: {0}")]
    FailedToBorrowParent(#[from] std::cell::BorrowMutError),
}

impl Node {
    #[must_use]
    pub fn new() -> Rc<RefCell<Self>> {
        Rc::new_cyclic(|s| {
            RefCell::new(Self {
                name: "Node".into(),
                children: vec![],
                parent: Weak::new(),
                children_name_lookup: HashMap::new(),
                self_ptr: s.clone(),
            })
        })
    }

    #[must_use]
    pub fn name(&self) -> &StringName {
        &self.name
    }

    /// whether or not this node has a parent attached or not
    #[must_use]
    pub fn has_parent(&self) -> bool {
        self.parent.upgrade().is_some()
    }

    pub fn add_child(&mut self, child: Rc<RefCell<Node>>) -> eyre::Result<()> {
        let child_name = {
            let Ok(mut child_ref) = child.try_borrow_mut() else {
                eyre::bail!("Cannot borrow child as mutable.");
            };

            if child_ref.has_parent() {
                eyre::bail!("A child cannot have multiple parent nodes at once");
            }

            while let Some(_) = self.child_name_to_index(child_ref.name()) {
                let new_name = StringName::from(format!("{}_", &child_ref.name));

                child_ref
                .try_rename(new_name)
                .expect("Child in invalid state, try_rename should always suceed if the child has no parent");
            }

            // update link in child
            child_ref.parent = self.self_ptr.clone();

            // get a copy of its name for book keeping
            child_ref.name().clone()
        };

        let idx = self.children.len();

        // push child to list
        self.children.push(child);

        // book keeping
        self.children_name_lookup.insert(child_name, idx as u32);

        Ok(())
    }

    /// Attempts to rename this node to a new name.
    ///
    /// # Errors
    ///
    /// This function will return an error if it cannot borrow its parent mutably or if the given
    /// name conflicts with the name of a sibling node.
    pub fn try_rename(&mut self, new_name: impl Into<StringName>) -> Result<(), RenameError> {
        let new_name = new_name.into();

        let Some(parent) = self.parent.upgrade() else {
            // if there is no parent then there is no restrictions
            self.name = new_name;
            return Ok(());
        };

        let mut parent = parent.try_borrow_mut()?;

        // fail if a sibling has the same name
        if let Some(sibling_idx) = parent.children_name_lookup.get(&new_name) {
            return Err(RenameError::DuplicateName {
                sibling_index: *sibling_idx,
            });
        }

        let child_index = parent.children_name_lookup[&self.name];

        // remove old name's lookup
        parent.children_name_lookup.remove(&self.name);

        parent
            .children_name_lookup
            .insert(new_name.clone(), child_index);

        self.name = new_name;

        Ok(())
    }

    /// How many children does this node have
    #[must_use]
    pub fn child_len(&self) -> usize {
        self.children.len()
    }

    #[must_use]
    pub fn nth_child(&self, idx: usize) -> Option<Rc<RefCell<Node>>> {
        Some(self.children.get(idx)?.clone())
    }

    #[must_use]
    pub fn child_name_to_index(&self, name: &StringName) -> Option<usize> {
        self.children_name_lookup.get(name).map(|i| *i as usize)
    }

    /// gets a child who has the given nmae
    #[must_use]
    pub fn get_child_by_name(&self, name: &StringName) -> Option<Rc<RefCell<Node>>> {
        Some(self.children[self.child_name_to_index(name)?].clone())
    }
}

#[cfg(test)]
mod test {
    use std::rc::Rc;

    use crate::{core::name::StringName, scene::node::Node};

    #[test]
    fn naming_no_parent() {
        let node = Node::new();

        let mut node = node.borrow_mut();

        assert_eq!(node.name(), &"Node".into());

        node.try_rename("Hello").unwrap();

        node.try_rename("Node").unwrap();
    }

    #[test]
    fn simple_parent() {
        color_eyre::install().unwrap();

        let parent = Node::new();

        assert_eq!(parent.borrow().child_len(), 0);

        let child_name = StringName::from("Child");

        assert!(parent.borrow().get_child_by_name(&child_name).is_none());

        let child = Node::new();
        child.borrow_mut().try_rename(child_name.clone()).unwrap();

        parent.borrow_mut().add_child(child.clone()).unwrap();

        assert_eq!(parent.borrow().child_len(), 1);
        assert_eq!(child.borrow().name(), &child_name);

        {
            // get the child, rename it, check the re ference really does match
            let node = parent.borrow().get_child_by_name(&child_name).unwrap();

            let new_name = StringName::from("Huh");

            {
                let first_child = parent.borrow().nth_child(0).unwrap();

                first_child
                    .borrow_mut()
                    .try_rename(new_name.clone())
                    .unwrap();
            }

            assert_eq!(&new_name, node.borrow().name());
        }
    }
}
